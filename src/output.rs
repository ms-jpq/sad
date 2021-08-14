use super::argparse::{Action, Options, Printer};
use super::fzf::stream_fzf;
use super::subprocess::{stream_into, stream_subprocess};
use super::types::{Abort, Fail};
use std::{path::PathBuf, sync::Arc};
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  sync::mpsc::Receiver,
  task::{spawn, JoinHandle},
};

fn stream_stdout(abort: &Arc<Abort>, stream: Receiver<String>) -> JoinHandle<()> {
  let abort = abort.clone();
  let mut stdout = BufWriter::new(io::stdout());

  spawn(async move {
    stream_into(&abort, PathBuf::from("/dev/stdout"), &mut stdout, stream).await;
    if let Err(err) = stdout.flush().await {
      abort
        .send(Fail::IO(PathBuf::from("/dev/stdout"), err.kind()))
        .await
    }
  })
}

pub fn stream_output(
  abort: &Arc<Abort>,
  opts: &Options,
  stream: Receiver<String>,
) -> JoinHandle<()> {
  match (&opts.action, &opts.printer) {
    (Action::Fzf(fzf_p, fzf_a), _) => stream_fzf(abort, fzf_p.to_owned(), fzf_a.to_owned(), stream),
    (_, Printer::Pager(cmd)) => stream_subprocess(abort, cmd.clone(), stream),
    (_, Printer::Stdout) => stream_stdout(abort, stream),
  }
}
