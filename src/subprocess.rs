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
  cmd: SubprocessCommand,
  mut stream: Receiver<String>,
) -> JoinHandle<()> {
  let abort = abort.clone();

  spawn(async move {
    let subprocess = Command::new(&cmd.prog)
      .kill_on_drop(true)
      .args(&cmd.args)
      .envs(&cmd.env)
      .stdin(Stdio::piped())
      .spawn();

    match subprocess {
      Err(err) => {
        let _ = abort.send(Fail::IO(cmd.prog, err.kind()));
      }
      Ok(mut child) => {
        let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

        let abort_1 = abort.clone();
        let p1 = cmd.prog.clone();
        let handle_in = spawn(async move {
          let mut on_abort = abort_1.subscribe();
          loop {
            select! {
              _ = on_abort.recv() => break,
              print = stream.recv() => {
                match print {
                  Some(val) => {
                    if let Err(err) = stdin.write(val.as_bytes()).await {
                      let _ = abort_1.send(Fail::IO(p1.clone(), err.kind()));
                      break;
                    }
                  }
                  None => break
                }
              }
            }
          }
          if let Err(err) = stdin.shutdown().await {
            let _ = abort_1.send(Fail::IO(p1, err.kind()));
          }
        });

        let p2 = cmd.prog.clone();
        let abort_2 = abort.clone();
        let handle_child = spawn(async move {
          if let Err(err) = child.wait().await {
            let _ = abort_2.send(Fail::IO(p2, err.kind()));
          }
        });

        if let Err(err) = try_join(handle_child, handle_in).await {
          if !err.is_cancelled() {
            let _ = abort.send(Fail::Join);
          }
        }
      }
    }
  })
}
