use super::errors::{SadResult, SadnessFrom};
use super::types::Task;
use async_channel::{bounded, Receiver, Sender};
use futures::future::try_join;
use std::{collections::HashMap, path::PathBuf, process::Stdio};
use tokio::{
  io::{AsyncWriteExt, BufWriter},
  process::Command,
  task,
};

#[derive(Clone, Debug)]
pub struct SubprocessCommand {
  pub program: PathBuf,
  pub arguments: Vec<String>,
  pub env: HashMap<String, String>,
}

impl SubprocessCommand {
  pub fn stream(&self, stream: Receiver<SadResult<String>>) -> (Task, Receiver<SadResult<String>>) {
    let (tx, rx) = bounded::<SadResult<String>>(1);
    let tt = Sender::clone(&tx);
    let ta = Sender::clone(&tx);

    let subprocess = Command::new(&self.program)
      .kill_on_drop(true)
      .args(&self.arguments)
      .envs(&self.env)
      .stdin(Stdio::piped())
      .spawn();

    let mut child = match subprocess.into_sadness() {
      Ok(child) => child,
      Err(err) => {
        let handle = task::spawn(async move { tx.send(Err(err)).await.expect("<CHAN>") });
        return (handle, rx);
      }
    };

    let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

    let handle_in = task::spawn(async move {
      while let Ok(print) = stream.recv().await {
        match print {
          Ok(val) => {
            if let Err(err) = stdin.write(val.as_bytes()).await.into_sadness() {
              tx.send(Err(err)).await.expect("<CHAN>");
              break;
            }
          }
          Err(err) => {
            tx.send(Err(err)).await.expect("<CHAN>");
            break;
          }
        }
      }
      if let Err(err) = stdin.shutdown().await {
        tx.send(Err(err.into())).await.expect("<CHAN>")
      }
    });

    let handle_child = task::spawn(async move {
      if let Err(err) = child.wait().await {
        tt.send(Err(err.into())).await.expect("<CHAN>")
      }
    });

    let handle = task::spawn(async move {
      if let Err(err) = try_join(handle_child, handle_in).await {
        ta.send(Err(err.into())).await.expect("<CHAN>")
      }
    });

    (handle, rx)
  }
}
