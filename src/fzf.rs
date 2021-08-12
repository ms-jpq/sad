use super::argparse::Options;
use super::errors::{Failure, SadResult, SadnessFrom};
use super::subprocess::SubprocessCommand;
use super::types::Task;
use async_channel::{bounded, Receiver, Sender};
use futures::future::try_join;
use std::{collections::HashMap, env, process::Stdio};
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  process::Command,
  select, task,
};
use which::which;

pub fn run_fzf(
  opts: &Options,
  stream: Receiver<SadResult<String>>,
) -> (Task, Receiver<SadResult<String>>) {
  let sad = env::current_exe()
    .or_else(|_| which("sad".to_owned()))
    .map(|p| format!("{}", p.display()))
    .unwrap_or("sad".to_owned());

  let preview_args = env::args().skip(1).collect::<Vec<_>>().join("\x04");
  let execute = format!(
    "abort+execute:{}\x04--internal-patch\x04{{+f}}\x04{}",
    sad, preview_args
  );
  let mut arguments = vec![
    "--read0".to_owned(),
    "--print0".to_owned(),
    "-m".to_owned(),
    "--ansi".to_owned(),
    format!("--bind=enter:{}", execute),
    format!("--bind=double-click:{}", execute),
    format!(
      "--preview={}\x04--internal-preview\x04{{f}}\x04{}",
      sad, preview_args
    ),
    "--preview-window=70%:wrap".to_owned(),
  ];
  arguments.extend(opts.fzf.clone().unwrap_or_default());
  let mut env = HashMap::new();
  env.insert("SHELL".to_owned(), sad);
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
  let (tx, rx) = bounded::<SadResult<String>>(1);
  let (tix, rix) = bounded::<Failure>(1);
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
      let handle = task::spawn(async move { tx.send(Err(err)).await.expect("<CHANNEL>") });
      return (handle, rx);
    }
  };

  let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

  let handle_in = task::spawn(async move {
    while let Ok(print) = stream.recv().await {
      match print {
        Ok(val) => {
          if let Err(err) = stdin.write(val.as_bytes()).await.into_sadness() {
            tix.send(err).await.expect("<CHAN>")
          }
        }
        Err(err) => tix.send(err).await.expect("<CHANNEL>"),
      }
    }
    if let Err(err) = stdin.shutdown().await {
      tix.send(err.into()).await.expect("<CHANNEL>")
    }
  });

  let handle_kill = task::spawn(async move {
    match rix.recv().await {
      Ok(err) => Some(err),
      Err(_) => None,
    }
  });

  let handle_child = task::spawn(async move {
    select! {
      lhs = child.wait() => {
        match lhs {
          Ok(status) => process_status_code(status.code(), tx).await,
          Err(err) => tx.send(Err(err.into())).await.expect("<CHANNEL>")
        }
      },
      rhs = handle_kill => {
        match rhs {
          Ok(Some(err)) => {
            let err = combine_err(err, child.kill().await.into_sadness());
            let err = combine_err(err, child.wait().await.into_sadness());
            let err = combine_err(err, reset_term().await);
            tx.send(Err(err)).await.expect("<CHAN>")
          },
          Ok(None) => match child.wait().await.into_sadness() {
            Err(err) => tx.send(Err(err)).await.expect("<CHANNEL>"),
            Ok(status) => process_status_code(status.code(), tx).await,
          }
          Err(err) => tx.send(Err(err.into())).await.expect("<CHANNEL>")
        }
      }
    }
  });

  let handle = task::spawn(async move {
    if let Err(err) = try_join(handle_child, handle_in).await {
      ta.send(Err(err.into())).await.expect("<CHAN>")
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
    Some(130) => tx.send(Err(Failure::Interrupt)).await.expect("<CHANNEL>"),
    Some(c) => tx
      .send(Err(Failure::Fzf(format!("Error exit - {}", c))))
      .await
      .expect("<CHANNEL>"),
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
    Ok(())
  } else if which("reset").is_ok() {
    Command::new("reset").status().await.into_sadness()?;
    Ok(())
  } else {
    Err(Failure::Fzf("Unable to clear screen".to_owned()))
  }
}
