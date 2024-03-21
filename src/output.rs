use {
  super::{
    argparse::{Action, Options, Printer},
    fzf::stream_fzf_proc,
    subprocess::stream_subproc,
    types::Fail,
  },
  futures::{
    future::ready,
    stream::{Stream, StreamExt, TryStreamExt},
  },
  std::{ffi::OsString, path::PathBuf, sync::Arc},
  tokio::io::{self, AsyncWrite, AsyncWriteExt, BufWriter},
};

fn stream_into<'a, W>(
  path: PathBuf,
  writer: BufWriter<W>,
  stream: impl Stream<Item = Result<OsString, Fail>> + 'a,
) -> impl Stream<Item = Result<(), Fail>> + 'a
where
  W: AsyncWrite + Send + Unpin + 'a,
{
  stream.scan((writer, path), |s, line| async {
    match line {
      Err(e) => Some(Err(e)),
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
        //let written = s
        //  .0
        //  .write_all(bytes)
        //  .await
        //  .map_err(|e| Fail::IO(s.1.clone(), e.kind()));
        //Some(written)
        Some(Ok(()))
      }
    }
  })
}

pub fn stream_out<'a>(
  opts: &Options,
  stream: impl Stream<Item = Result<OsString, Fail>> + 'a,
) -> impl Stream<Item = Fail> + 'a {
  let pipe = match (&opts.action, &opts.printer) {
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
      stream_into(PathBuf::from("/dev/stdout"), stdout, stream)
    }
  };

  pipe.filter_map(|r| {
    ready(match r {
      Ok(()) => None,
      Err(e) => Some(e),
    })
  })
}
