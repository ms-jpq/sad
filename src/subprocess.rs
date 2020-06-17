use super::errors::*;
use super::types::Task;
use async_std::sync::{channel, Receiver, Sender};
use futures::future::try_join4;
use std::{collections::HashMap, process::Stdio};
use tokio::{
  io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
  process::Command,
  task,
};

#[derive(Clone, Debug)]
pub struct SubprocessCommand {
  pub program: String,
  pub arguments: Vec<String>,
  pub env: HashMap<String, String>,
}

impl SubprocessCommand {
  pub fn stream(&self, stream: Receiver<SadResult<String>>) -> (Task, Receiver<SadResult<String>>) {
    let (tx, rx) = channel::<SadResult<String>>(1);
    let to = Sender::clone(&tx);
    let te = Sender::clone(&tx);
    let tt = Sender::clone(&tx);
    let ta = Sender::clone(&tx);

    let subprocess = Command::new(&self.program)
      .kill_on_drop(true)
      .args(&self.arguments)
      .envs(&self.env)
      .kill_on_drop(true)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn();

    let mut child = match subprocess.into_sadness() {
      Ok(child) => child,
      Err(err) => {
        let handle = task::spawn(async move { tx.send(Err(err)).await });
        return (handle, rx);
      }
    };

    let mut stdin = child.stdin.take().map(BufWriter::new).unwrap();
    let mut stdout = child.stdout.take().map(BufReader::new).unwrap();
    let mut stderr = child.stderr.take().map(BufReader::new).unwrap();

    let handle_in = task::spawn(async move {
      while let Ok(print) = stream.recv().await {
        match print {
          Ok(val) => {
            if let Err(err) = stdin.write(val.as_bytes()).await.into_sadness() {
              tx.send(Err(err)).await;
            }
          }
          Err(err) => tx.send(Err(err)).await,
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
        ta.send(Err(err.into())).await;
      }
    });

    (handle, rx)
  }
}

