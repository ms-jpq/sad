use {
  super::{
    argparse::{Action, Options, Printer},
    fzf::stream_fzf_proc,
    subprocess::{stream_into, stream_subproc},
    types::{Abort, Fail},
  },
  std::{ffi::OsString, path::PathBuf, sync::Arc},
  tokio::{
    io::{self, AsyncWriteExt, BufWriter},
    sync::mpsc::Receiver,
    task::{spawn, JoinHandle},
  },
};

fn stream_stdout(abort: &Arc<Abort>, stream: Receiver<OsString>) -> JoinHandle<()> {
  let abort = abort.clone();
  let mut stdout = BufWriter::new(io::stdout());

  spawn(async move {
    stream_into(&abort, PathBuf::from("/dev/stdout"), &mut stdout, stream).await;
    if let Err(err) = stdout.flush().await {
      abort
        .send(Fail::IO(PathBuf::from("/dev/stdout"), err.kind()))
        .await;
    }
  })
}

pub fn stream_out(abort: &Arc<Abort>, opts: &Options, stream: Receiver<OsString>) -> JoinHandle<()> {
  match (&opts.action, &opts.printer) {
    (Action::FzfPreview(fzf_p, fzf_a), _) => {
      stream_fzf_proc(abort, fzf_p.clone(), fzf_a.clone(), stream)
    }
    (_, Printer::Pager(cmd)) => stream_subproc(abort, cmd.clone(), stream),
    (_, Printer::Stdout) => stream_stdout(abort, stream),
  }
}
