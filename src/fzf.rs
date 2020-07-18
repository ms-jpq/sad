use super::argparse::Options;
use super::errors::*;
use super::subprocess::SubprocessCommand;
use super::types::Task;
use async_std::sync::{channel, Receiver, Sender};
use futures::future::{select, try_join, Either};
use std::{collections::HashMap, env, process::Stdio};
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  process::Command,
  task,
};
use which::which;

pub fn run_fzf(
  opts: &Options,
  stream: Receiver<SadResult<String>>,
) -> (Task, Receiver<SadResult<String>>) {
  let preview_args = env::args().collect::<Vec<_>>().join("\x04");
  let execute = format!(
    "abort+execute:{}\x04--internal-patch\x04{{+f}}",
    preview_args
  );
  let mut arguments = vec![
    "--read0".to_owned(),
    "--print0".to_owned(),
    "-m".to_owned(),
    "--ansi".to_owned(),
    format!("--bind=enter:{}", execute),
    format!("--bind=double-click:{}", execute),
    format!("--preview={}\x04--internal-preview\x04{{f}}", preview_args),
    "--preview-window=70%:wrap".to_owned(),
  ];
  arguments.extend(opts.fzf.clone().unwrap_or_default());
  let mut env = HashMap::new();
  env.insert("SHELL".to_owned(), opts.name.clone());
  let cmd = SubprocessCommand {
    program: "fzf".to_owned(),
    arguments,
    env,
  };
  stream_fzf(&cmd, stream)
}

fn stream_fzf(
  cmd: &SubprocessCommand,
  stream: Receiver<SadResult<String>>,
) -> (Task, Receiver<SadResult<String>>) {
  let (tx, rx) = channel::<SadResult<String>>(1);
  let (tix, rix) = channel::<Failure>(1);
  let ta = Sender::clone(&tx);

  let subprocess = Command::new(&cmd.program)
    .args(&cmd.arguments)
    .envs(&cmd.env)
    .kill_on_drop(true)
    .stdin(Stdio::piped())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .spawn();

  let mut child = match subprocess.into_sadness() {
    Ok(child) => child,
    Err(err) => {
      let handle = task::spawn(async move { tx.send(Err(err)).await });
      return (handle, rx);
    }
  };

  let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

  let handle_in = task::spawn(async move {
    while let Ok(print) = stream.recv().await {
      match print {
        Ok(val) => {
          if let Err(err) = stdin.write(val.as_bytes()).await.into_sadness() {
            tix.send(err).await;
          }
        }
        Err(err) => tix.send(err).await,
      }
    }
    if let Err(err) = stdin.shutdown().await {
      tix.send(err.into()).await
    }
  });

  let handle_kill = task::spawn(async move {
    match rix.recv().await {
      Ok(err) => Some(err),
      Err(_) => None,
    }
  });

  let handle_child = task::spawn(async move {
    match select(child, handle_kill).await {
      Either::Left((Ok(status), _)) => process_status_code(status.code(), tx).await,
      Either::Left((Err(err), _)) => tx.send(Err(err.into())).await,
      Either::Right((handle, mut child)) => {
        let maybe_failure = match handle.into_sadness() {
          Ok(err) => err,
          Err(err) => Some(err),
        };
        match maybe_failure {
          Some(err) => {
            let err = combine_err(err, child.kill().into_sadness());
            let err = combine_err(err, child.await.into_sadness());
            let err = combine_err(err, reset_term().await);
            tx.send(Err(err)).await
          }
          None => match child.await.into_sadness() {
            Err(err) => tx.send(Err(err)).await,
            Ok(status) => process_status_code(status.code(), tx).await,
          },
        }
      }
    }
  });

  let handle = task::spawn(async move {
    if let Err(err) = try_join(handle_child, handle_in).await {
      ta.send(Err(err.into())).await;
    }
  });

  (handle, rx)
}

fn combine_err<T>(err: Failure, res: SadResult<T>) -> Failure {
  match res {
    Ok(_) => err,
    Err(e) => Failure::Compound(Box::new(err), Box::new(e)),
  }
}

async fn process_status_code(code: Option<i32>, tx: Sender<SadResult<String>>) {
  match code {
    Some(0) | Some(1) | None => {}
    Some(130) => tx.send(Err(Failure::Interrupt)).await,
    Some(c) => {
      tx.send(Err(Failure::Fzf(format!("Error exit - {}", c))))
        .await
    }
  }
}

async fn reset_term() -> SadResult<()> {
  io::stdout().flush().await.into_sadness()?;
  io::stderr().flush().await.into_sadness()?;
  if which("tput").is_ok() {
    Command::new("tput")
      .arg("reset")
      .status()
      .await
      .into_sadness()?;
  } else if which("reset").is_ok() {
    Command::new("reset").status().await.into_sadness()?;
  } else {
    return Err(Failure::Fzf("Unable to clear screen".to_owned()));
  };
  Ok(())
}
