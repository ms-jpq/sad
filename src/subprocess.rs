use {
  super::types::Die,
  futures::{
    future::ready,
    stream::{once, select, try_unfold, Stream, StreamExt},
  },
  std::{collections::HashMap, ffi::OsString, marker::Unpin, path::PathBuf, process::Stdio},
  tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufWriter},
    process::Command,
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
  stream: impl Stream<Item = Result<OsString, Die>> + Send + Unpin,
) -> impl Stream<Item = Result<(), Die>> + Send
where
{
  let buf = BufWriter::new(writer);
  try_unfold((stream, buf, path), |mut s| async {
    match s.0.next().await {
      None => {
        s.1
          .shutdown()
          .await
          .map_err(|e| Die::IO(s.2.clone(), e.kind()))?;
        Ok(None)
      }
      Some(Err(e)) => {
        let _ = s.1.shutdown().await;
        Err(e)
      }
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
        s.1
          .write_all(bytes)
          .await
          .map_err(|e| Die::IO(s.2.clone(), e.kind()))?;
        Ok(Some(((), s)))
      }
    }
  })
}

pub fn stream_subproc<'a>(
  cmd: SubprocCommand,
  stream: impl Stream<Item = Result<OsString, Die>> + Unpin + Send + 'a,
) -> Box<dyn Stream<Item = Result<(), Die>> + Send + 'a> {
  let subprocess = Command::new(&cmd.prog)
    .kill_on_drop(true)
    .args(&cmd.args)
    .envs(&cmd.env)
    .stdin(Stdio::piped())
    .spawn();

  match subprocess {
    Err(e) => {
      let err = Die::IO(cmd.prog, e.kind());
      Box::new(once(ready(Err(err))))
    }
    Ok(mut child) => {
      let stdin = child.stdin.take().expect("child process stdin");
      let out = stream_into(cmd.prog.clone(), stdin, stream);
      let die = once(async move {
        match child.wait().await {
          Err(e) => Err(Die::IO(cmd.prog, e.kind())),
          Ok(status) if status.success() => Ok(()),
          Ok(status) => {
            let code = status.code().unwrap_or(1);
            Err(Die::BadExit(cmd.prog, code))
          }
        }
      });
      Box::new(select(out, die))
    }
  }
}
