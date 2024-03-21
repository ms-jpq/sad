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

pub fn stream_sink(
  opts: &Options,
  stream: impl Stream<Item = Result<OsString, Fail>> + Unpin,
) -> Box<dyn Stream<Item = Result<(), Fail>>> {
  match (&opts.action, &opts.printer) {
    (Action::FzfPreview(fzf_p, fzf_a), _) => {
      //stream_fzf_proc( fzf_p, fzf_a, stream)

      todo!()
    }
    (_, Printer::Pager(cmd)) => {
      todo!()
      //Box::new(stream_subproc(cmd.clone(), stream))
    }
    (_, Printer::Stdout) => {
      todo!()
      //let stdout = BufWriter::new(io::stdout());
      //Box::new(stream_into(PathBuf::from("/dev/stdout"), stdout, stream))
    }
  }
}
