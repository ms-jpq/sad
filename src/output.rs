use {
  super::{
    argparse::{Action, Options, Printer},
    fzf::stream_fzf_proc,
    subprocess::{stream_into, stream_subproc},
    types::Fail,
  },
  futures::stream::{Stream, StreamExt, TryStreamExt},
  std::{ffi::OsString, marker::Unpin, path::PathBuf, sync::Arc},
  tokio::io::{self, AsyncWrite, AsyncWriteExt, BufWriter},
};

pub fn stream_sink<'a>(
  opts: &Options,
  stream: impl Stream<Item = Result<OsString, Fail>> + Unpin + 'a,
) -> Box<dyn Stream<Item = Result<(), Fail>> + 'a> {
  match (&opts.action, &opts.printer) {
    (Action::FzfPreview(fzf_p, fzf_a), _) => {
      //stream_fzf_proc( fzf_p, fzf_a, stream)

      todo!()
    }
    (_, Printer::Pager(cmd)) => stream_subproc(cmd.clone(), stream),
    (_, Printer::Stdout) => {
      let stdout = BufWriter::new(io::stdout());
      Box::new(stream_into(PathBuf::from("/dev/stdout"), stdout, stream))
    }
  }
}
