use {
  super::types::Fail,
  futures::{
    future::{ready, select, try_join, Either},
    stream::{once, try_unfold, Stream, StreamExt},
  },
  std::{
    collections::HashMap, ffi::OsString, marker::Unpin, path::PathBuf, process::Stdio, sync::Arc,
  },
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

pub fn stream_into(
  path: PathBuf,
  writer: impl AsyncWrite + Send + Unpin,
  stream: impl Stream<Item = Result<OsString, Fail>> + Unpin,
) -> impl Stream<Item = Result<(), Fail>>
where
{
  let buf = BufWriter::new(writer);
  try_unfold((buf, stream, path), |mut s| async {
    match s.1.next().await {
      None => Ok(None),
      Some(Err(e)) => Err(e),
      Some(Ok(print)) => {
        #[cfg(target_family = "unix")]
        let bytes = {
          use std::os::unix::ffi::OsStrExt;
          print.as_bytes()
        };
        #[cfg(target_family = "windows")]
        let bytes = {
          let tmp = print.to_string_lossy();
          tmp.as_bytes()
        };
        s.0
          .write_all(bytes)
          .await
          .map_err(|e| Fail::IO(s.2.clone(), e.kind()))?;
        Ok(Some(((), s)))
      }
    }
  })
}

pub fn stream_subproc(
  cmd: SubprocCommand,
  stream: impl Stream<Item = Result<OsString, Fail>>,
) -> Box<dyn Stream<Item = Result<(), Fail>>> {
  let subprocess = Command::new(&cmd.prog)
    .kill_on_drop(true)
    .args(&cmd.args)
    .envs(&cmd.env)
    .stdin(Stdio::piped())
    .spawn();

  match subprocess {
    Err(e) => {
      let err = Fail::IO(cmd.prog, e.kind());
      Box::new(once(ready(Err(err))))
    }
    Ok(mut child) => {
      todo!()
      //let mut stdin = child
      //  .stdin
      //  .take()
      //  .map(BufWriter::new)
      //  .expect("child process stdin");

      //stream_into( p1.clone(),  stdin);

      //let p1 = cmd.prog.clone();
      //let handle_in = spawn(async move {
      //  .await;
      //  if let Err(err) = stdin.shutdown().await {
      //    abort_1.send(Fail::IO(p1, err.kind())).await;
      //  }
      //});

      //let handle_child = spawn(async move {
      //  if let Err(err) = child.wait().await {
      //    abort_2.send(Fail::IO(p2, err.kind())).await;
      //  }
      //});

      //if let Err(err) = try_join(handle_child, handle_in).await {
      //  abort.send(err.into()).await;
      //}
    }
  }
}
