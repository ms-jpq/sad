use {
  super::types::{Abort, Fail},
  futures::{
    future::{select, try_join, Either},
    pin_mut,
  },
  std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc},
  tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufWriter},
    process::Command,
    sync::mpsc::Receiver,
    task::{spawn, JoinHandle},
  },
};

#[derive(Clone, Debug)]
pub struct SubprocCommand {
  pub prog: PathBuf,
  pub args: Vec<String>,
  pub env: HashMap<String, String>,
}

pub async fn stream_into(
  abort: &Arc<Abort>,
  path: PathBuf,
  writer: &mut BufWriter<impl AsyncWrite + Send + Unpin>,
  mut stream: Receiver<String>,
) {
  loop {
    let f1 = abort.notified();
    let f2 = stream.recv();
    pin_mut!(f1);
    pin_mut!(f2);
    match select(f1, f2).await {
      Either::Right((Some(print), _)) => {
        if let Err(err) = writer.write(print.as_bytes()).await {
          abort.send(Fail::IO(path, err.kind())).await;
          break;
        }
      }
      _ => break,
    }
  }
}

pub fn stream_subproc(
  abort: &Arc<Abort>,
  cmd: SubprocCommand,
  stream: Receiver<String>,
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
      Err(err) => abort.send(Fail::IO(cmd.prog, err.kind())).await,
      Ok(mut child) => {
        let mut stdin = child.stdin.take().map(BufWriter::new).expect("nil stdin");

        let abort_1 = abort.clone();
        let p1 = cmd.prog.clone();
        let handle_in = spawn(async move {
          stream_into(&abort_1, p1.clone(), &mut stdin, stream).await;
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
          abort.send(err.into()).await;
        }
      }
    }
  })
}
