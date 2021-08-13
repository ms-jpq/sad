use super::types::{Abort, Fail};
use futures::future::try_join;
use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc};
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
  abort: &Arc<Abort>,
  cmd: SubprocessCommand,
  mut stream: Receiver<String>,
) -> JoinHandle<()> {
  let abort= abort.clone();

  spawn(async move {
    let subprocess = Command::new(&cmd.prog)
      .kill_on_drop(true)
      .args(&cmd.args)
      .envs(&cmd.env)
      .stdin(Stdio::piped())
      .spawn();

    match subprocess {
      Err(err) => abort.send(Fail::IO(cmd.prog, err.kind())).await,
      Ok(mut child) => {
        let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

        let abort_1 = abort.clone();
        let p1 = cmd.prog.clone();
        let handle_in = spawn(async move {
          loop {
            select! {
              _ = abort_1.rx.notified() => break,
              print = stream.recv() => {
                match print {
                  Some(val) => {
                    if let Err(err) = stdin.write(val.as_bytes()).await {
                      abort_1.send(Fail::IO(p1.clone(), err.kind())).await;
                      break;
                    }
                  }
                  None => break
                }
              }
            }
          }
          if let Err(err) = stdin.shutdown().await {
            abort_1.send(Fail::IO(p1, err.kind())).await;
          }
        });

        let abort_2 = abort.clone();
        let p2 = cmd.prog.clone();
        let handle_child = spawn(async move {
          if let Err(err) = child.wait().await {
            abort_2.send(Fail::IO(p2, err.kind())).await;
          }
        });

        if let Err(err) = try_join(handle_child, handle_in).await {
          abort.send(err.into()).await
        }
      }
    }
  })
}
