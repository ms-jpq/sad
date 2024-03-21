use {
  super::{
    argparse::{Action, Options, Printer},
    fzf::stream_fzf_proc,
    subprocess::stream_subproc,
    types::{Abort, Fail},
  },
  futures::stream::{Stream, TryStreamExt},
  std::{ffi::OsString, path::PathBuf, sync::Arc},
  tokio::io::{self, AsyncWrite, AsyncWriteExt, BufWriter},
};

fn stream_into(
  path: PathBuf,
  writer: &mut BufWriter<impl AsyncWrite + Send + Unpin>,
  stream: impl Stream<Item = Result<OsString, Fail>>,
) -> impl Stream<Item = Result<(), Fail>> {
  let x = stream.map_ok(|line| async {
    #[cfg(target_family = "unix")]
    let bytes = {
      use std::os::unix::ffi::OsStrExt;
      line.as_bytes()
    };
    #[cfg(target_family = "windows")]
    let bytes = {
      let tmp = line.to_string_lossy();
      tmp.as_bytes()
    };

    writer
      .write_all(bytes)
      .await
      .map_err(|e| Fail::IO(path, e.kind()))?;
    Ok(())
  });
  x
}

pub fn stream_out(
  opts: &Options,
  stream: impl Stream<Item = Result<OsString, Fail>>,
) -> impl Stream<Item = Result<(), Fail>> {
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
      let mut stdout = BufWriter::new(io::stdout());
      stream_into(PathBuf::from("/dev/stdout"), &mut stdout, stream)
    }
  }
}
