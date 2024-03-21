use {
  super::{
    argparse::{Action, Options, Printer},
    fzf::stream_fzf_proc,
    subprocess::stream_subproc,
    types::Fail,
  },
  futures::{
    future::ready,
    sink::{unfold, Sink, SinkExt},
    stream::{Stream, StreamExt, TryStreamExt},
  },
  std::{ffi::OsString, path::PathBuf, sync::Arc},
  tokio::io::{self, AsyncWrite, AsyncWriteExt, BufWriter},
};

fn stream_into<W>(
  path: PathBuf,
  writer: BufWriter<W>,
) -> impl Sink<Result<OsString, Fail>, Error = Fail>
where
  W: AsyncWrite + Send + Unpin,
{
  unfold((writer, path), |mut s, line: Result<OsString, Fail>| async {
    match line {
      Err(e) => Err(e),
      Ok(print) => {
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
          .map_err(|e| Fail::IO(s.1.clone(), e.kind()))?;
        Ok(s)
      }
    }
  })
}

pub fn stream_sink(opts: &Options) -> impl Sink<Result<OsString, Fail>, Error = Fail> {
  match (&opts.action, &opts.printer) {
    (Action::FzfPreview(fzf_p, fzf_a), _) => {
      //stream_fzf_proc(abort, fzf_p.clone(), fzf_a.clone(), stream)

      todo!()
    }
    (_, Printer::Pager(cmd)) => {
      //stream_subproc(abort, cmd.clone(), stream)

      todo!()
    }
    (_, Printer::Stdout) => {
      let stdout = BufWriter::new(io::stdout());
      stream_into(PathBuf::from("/dev/stdout"), stdout)
    }
  }
}
