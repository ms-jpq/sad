use super::errors::Failure;
use super::subprocess::SubprocessCommand;
use super::types::{Abort, Task};
use async_channel::{bounded, Receiver, Sender};
use futures::future::try_join;
use std::{
  collections::HashMap,
  env,
  error::Error,
  path::PathBuf,
  process::{ExitStatus, Stdio},
};
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  process::Command,
  select, task,
};
use which::which;

async fn reset_term(abort: Abort) {
  try_join(io::stdout().flush(), io::stderr()).await?;
  if let Ok(path) = which("tput") {
    Command::new("tput").arg("reset").status().await
  } else if let Ok(path) = which("reset") {
    Command::new("reset").status().await
  } else {
    abort.tx.send(Failure::Sucks("")).expect("<CHANNEL>")
  }
}
async fn process_status_code(abort: Abort, status: ExitStatus) {
  match status.code() {
    Some(0) | Some(1) | None => {}
    Some(130) => abort
      .tx
      .send(Err(Box::new(Failure::Interrupt)))
      .await
      .expect("<CHANNEL>"),
    Some(c) => abort
      .tx
      .send(Err(Box::new(Failure::Fzf(format!("Error exit - {}", c)))))
      .await
      .expect("<CHANNEL>"),
  }
}

fn stream_fzf(abort: Abort, cmd: &SubprocessCommand, stream: Receiver<String>) -> Task {
  let subprocess = Command::new(&cmd.program)
    .args(&cmd.arguments)
    .envs(&cmd.env)
    .kill_on_drop(true)
    .stdin(Stdio::piped())
    .spawn();

  let mut child = match subprocess {
    Ok(child) => child,
    Err(err) => {
      abort.tx.send(Box::new(err)).expect("<CHANNEL>");
      task::spawn(async move {  });
    }
  };
  let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

  let handle_in = task::spawn(async move {
    loop {
      select! {
        _ = abort.rx.changed() => break,
        print = stream.recv() => {
        match print {
          Ok(val) => {
            if let Err(err) = stdin.write(val.as_bytes()).await {
              abort.tx.send(err).await.expect("<CHAN>")
            }
          }
          Err(err) => {
            abort.tx.send(err).await.expect("<CHANNEL>");
            break;
          }
        }
        }
      }
    }
    if let Err(err) = stdin.shutdown().await {
      abort.tx.send(err).await.expect("<CHANNEL>")
    }
  });

  let handle_child = task::spawn(async move {
    select! {
      rhs = abort.rx.changed() => {
        match rhs {
          Ok(Some(err)) => {
            let err1 = child.kill().await;
            let err2 = child.wait().await;
            let err3 = reset_term().await;
            if let Err(err) = err1 {
              abort.tx.send(Box::new(err1)).expect("<CHAN>")
            } else
            if let Err(err) = err2 {
              abort.tx.send(Box::new(err1)).expect("<CHAN>")
            } else
            if let Err(err) = err3 {
              abort.tx.send(Box::new(err1)).expect("<CHAN>")
            }
          },
          Ok(None) => match child.wait().await {
            Err(err) => abort.tx.send(Box::new(err)).expect("<CHANNEL>"),
            Ok(status) => process_status_code(abort, status).await,
          }
          Err(err) =>abort. tx.send(Box::new(err())).await.expect("<CHANNEL>")
        }
      },
      lhs = child.wait() => {
        match lhs {
            Ok(status) => process_status_code(abort, status).await,
          Err(err) => abort.tx.send(Box::new(err())).expect("<CHANNEL>")
        }
      },
    }
  });

  task::spawn(async move {
    if let Err(err) = try_join(handle_child, handle_in).await {
      abort.send(Box::new(err())).expect("<CHAN>")
    }
  })
}

pub fn run_fzf(abort: Abort, bin: PathBuf, args: Vec<String>, stream: Receiver<String>) -> Task {
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
  arguments.extend(args);
  let mut env = HashMap::new();
  env.insert("SHELL".to_owned(), sad);
  let cmd = SubprocessCommand {
    program: bin,
    arguments,
    env,
  };
  stream_fzf(&cmd, stream)
}
