use super::errors::*;
use super::types::Task;
use async_std::sync::{channel, Receiver, Sender};
use futures::future::try_join4;
use std::process::Stdio;
use tokio::{
  io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
  process::Command,
  task,
};

#[derive(Clone)]
pub struct SubprocessCommand {
  pub program: String,
  pub arguments: Vec<String>,
}

pub fn stream(
  cmd: &SubprocessCommand,
  stream: Receiver<SadResult<String>>,
) -> (Task, Receiver<SadResult<String>>) {
  let subprocess = Command::new(&cmd.program)
    .args(&cmd.arguments)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn();

  let mut child = match subprocess {
    Ok(child) => child,
    Err(err) => err_exit(err.into()),
  };

  let mut stdin = match child.stdin.take() {
    Some(stdin) => BufWriter::new(stdin),
    None => err_exit(Failure::Pager("Invalid stdin".into())),
  };
  let mut stdout = match child.stdout.take() {
    Some(stdout) => BufReader::new(stdout),
    None => err_exit(Failure::Pager("Invalid stdout".into())),
  };
  let mut stderr = match child.stderr.take() {
    Some(stderr) => BufReader::new(stderr),
    None => err_exit(Failure::Pager("Invalid stderr".into())),
  };

  let (tx, rx) = channel::<SadResult<String>>(1);
  let to = Sender::clone(&tx);
  let te = Sender::clone(&tx);
  let tt = Sender::clone(&tx);
  let t4 = Sender::clone(&tx);

  let handle_in = task::spawn(async move {
    while let Some(print) = stream.recv().await {
      match print {
        Ok(val) => {
          if let Err(e) = stdin.write(val.as_bytes()).await {
            err_exit(e.into())
          }
        }
        Err(e) => err_exit(e),
      }
    }
    if let Err(err) = stdin.shutdown().await {
      tx.send(Err(err.into())).await;
    }
  });

  let handle_out = task::spawn(async move {
    loop {
      let mut buf = String::new();
      match stdout.read_line(&mut buf).await.into_sadness() {
        Ok(0) => return,
        Ok(_) => {
          to.send(Ok(buf)).await;
        }
        Err(err) => to.send(Err(err)).await,
      }
    }
  });

  let handle_err = task::spawn(async move {
    let mut buf = String::new();
    match stderr.read_to_string(&mut buf).await.into_sadness() {
      Err(err) => {
        te.send(Err(err)).await;
      }
      Ok(_) => {
        if !buf.is_empty() {
          te.send(Err(Failure::Pager(buf))).await
        }
      }
    }
  });

  let handle_child = task::spawn(async move {
    if let Err(err) = child.await {
      tt.send(Err(err.into())).await;
    }
  });

  let handle = task::spawn(async move {
    if let Err(err) = try_join4(handle_child, handle_in, handle_out, handle_err).await {
      t4.send(Err(err.into())).await;
    }
  });

  (handle, rx)
}
