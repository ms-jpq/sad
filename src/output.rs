use {
  super::{
    argparse::{Action, Options, Printer},
    fzf::stream_fzf_proc,
    subprocess::{stream_into, stream_subproc},
    types::Fail,
  },
  futures::stream::Stream,
  std::{ffi::OsString, marker::Unpin, path::PathBuf},
  tokio::io,
};

pub fn stream_sink<'a>(
  opts: &Options,
  stream: impl Stream<Item = Result<OsString, Fail>> + Unpin + Send + 'a,
) -> Box<dyn Stream<Item = Result<(), Fail>> + Send + 'a> {
  match (&opts.action, &opts.printer) {
    (Action::FzfPreview(fzf_p, fzf_a), _) => stream_fzf_proc(fzf_p.clone(), fzf_a.clone(), stream),
    (_, Printer::Pager(cmd)) => stream_subproc(cmd.clone(), stream),
    (_, Printer::Stdout) => {
      let stdout = io::stdout();
      Box::new(stream_into(PathBuf::from("/dev/stdout"), stdout, stream))
    }
  }
}
