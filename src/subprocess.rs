use super::types::{Abort, Task};
use async_channel::Receiver;
use futures::future::try_join;
use std::{collections::HashMap, path::PathBuf, process::Stdio};
use tokio::{
  io::{AsyncWriteExt, BufWriter},
  process::Command,
  select, task,
};

#[derive(Clone, Debug)]
pub struct SubprocessCommand {
  pub program: PathBuf,
  pub arguments: Vec<String>,
  pub env: HashMap<String, String>,
}

impl SubprocessCommand {
  pub fn stream(&self, abort: Abort, stream: Receiver<String>) -> Task {
    let subprocess = Command::new(&self.program)
      .kill_on_drop(true)
      .args(&self.arguments)
      .envs(&self.env)
      .stdin(Stdio::piped())
      .spawn();

    let mut child = match subprocess {
      Ok(child) => child,
      Err(err) => {
        abort.tx.send(Box::new(err)).expect("<CHAN>");
        return task::spawn(async move {});
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
                  abort.tx.send(Box::new(err)).expect("<CHAN>");
                  break;
                }
              }
              Err(err) => {
                abort.tx.send(Box::new(err)).expect("<CHAN>");
                break;
              }
            }

          }
        }
      }
      if let Err(err) = stdin.shutdown().await {
        abort.tx.send(Box::new(err)).expect("<CHAN>")
      }
    });

    let handle_child = task::spawn(async move {
      if let Err(err) = child.wait().await {
        abort.tx.send(Box::new(err)).expect("<CHAN>")
      }
    });

    task::spawn(async move {
      if let Err(err) = try_join(handle_child, handle_in).await {
        abort.tx.send(Box::new(err)).expect("<CHAN>")
      }
    })
  }
}
