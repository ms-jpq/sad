use super::errors::*;
use super::types::Task;
use async_std::sync::{channel, Receiver, Sender};
use std::process::Stdio;
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
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
) -> (Task, Task, Receiver<SadResult<String>>) {
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

  let mut stdin = BufWriter::new(child.stdin.take().unwrap());
  let mut stdout = BufReader::new(child.stdout.take().unwrap());
  let mut stderr = BufReader::new(child.stderr.take().unwrap());

  let (tx, rx) = channel::<SadResult<String>>(1);
  let tt = Sender::clone(&tx);

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
    };
  });

  let handle_out = task::spawn(async move {
    loop {
      let mut buf = String::new();
      match stdout.read_line(&mut buf).await.into_sadness() {
        Ok(0) => return,
        Ok(_) => {
          tt.send(Ok(buf)).await;
        }
        Err(err) => tt.send(Err(err)).await,
      }
    }
  });

  (handle_in, handle_out, rx)
}
