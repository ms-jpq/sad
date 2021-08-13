use super::types::{Abort, Fail};
use futures::future::try_join;
use std::{collections::HashMap, path::PathBuf, process::Stdio};
use tokio::{
  io::{AsyncWriteExt, BufWriter},
  process::Command,
  select,
  sync::mpsc::Receiver,
  task::{spawn, JoinHandle},
};

#[derive(Clone, Debug)]
pub struct SubprocessCommand {
  pub prog: PathBuf,
  pub args: Vec<String>,
  pub env: HashMap<String, String>,
}

pub fn stream_subprocess(
  abort: &Abort,
  cmd: &SubprocessCommand,
  stream: Receiver<String>,
) -> JoinHandle<()> {
  let subprocess = Command::new(&cmd.prog)
    .kill_on_drop(true)
    .args(&cmd.args)
    .envs(&cmd.env)
    .stdin(Stdio::piped())
    .spawn();

  spawn(async move {
    match subprocess {
      Err(err) => {
        let _ = abort.send(Fail::IO(cmd.prog, err.kind()));
      }
      Ok(child) => {
        let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

        let handle_in = spawn(async move {
          let mut on_abort = abort.subscribe();
          loop {
            select! {
              _ = on_abort.recv() => break,
              print = stream.recv() => {
                match print {
                  Some(val) => {
                    if let Err(err) = stdin.write(val.as_bytes()).await {
                      let _ = abort.send(Fail::IO(cmd.prog, err.kind()));
                      break;
                    }
                  }
                  None => break
                }
              }
            }
          }
          if let Err(err) = stdin.shutdown().await {
            let _ = abort.send(Fail::IO(cmd.prog, err.kind()));
          }
        });

        let handle_child = spawn(async move {
          if let Err(err) = child.wait().await {
            let _ = abort.send(Fail::IO(cmd.prog, err.kind()));
          }
        });

        if let Err(err) = try_join(handle_child, handle_in).await {
          let _ = abort.send(Fail::IO(cmd.prog, err.kind()));
        }
      }
    }
  })
}
