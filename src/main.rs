#![deny(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]
#![allow(
  clippy::cargo_common_metadata,
  clippy::multiple_crate_versions,
  clippy::wildcard_dependencies
)]

mod argparse;
mod displace;
mod fs_pipe;
mod fzf;
mod input;
mod subprocess;
mod types;
mod udiff;
mod udiff_spec;

use {
  ansi_term::Colour,
  argparse::{parse_args, parse_opts, Action, Options, Printer},
  displace::displace,
  futures::{
    future::{ready, Either},
    stream::{Stream, StreamExt, TryStreamExt},
  },
  fzf::stream_fzf_proc,
  input::stream_in,
  std::{
    convert::Into,
    ffi::OsString,
    marker::Unpin,
    path::PathBuf,
    process::{exit, ExitCode, Termination},
    sync::Arc,
    thread::available_parallelism,
  },
  subprocess::{stream_into, stream_subproc},
  tokio::{io, runtime::Builder},
  types::Die,
};

fn stream_sink(
  opts: &Options,
  stream: impl Stream<Item = Result<OsString, Die>> + Unpin,
) -> impl Stream<Item = Result<(), Die>> {
  match (&opts.action, &opts.printer) {
    (Action::FzfPreview(fzf_p, fzf_a), _) => Either::Left(Either::Left(stream_fzf_proc(
      fzf_p.clone(),
      fzf_a.clone(),
      stream,
    ))),
    (_, Printer::Pager(cmd)) => Either::Left(Either::Right(stream_subproc(cmd.clone(), stream))),
    (_, Printer::Stdout) => {
      let stdout = io::stdout();
      Either::Right(stream_into(PathBuf::from("/dev/stdout"), stdout, stream))
    }
  }
}

async fn run(threads: usize) -> Result<(), Die> {
  let (mode, args) = parse_args();
  let input_stream = stream_in(&mode, &args).await;
  let opts = Arc::new(parse_opts(mode, args)?);

  let trans_stream = input_stream
    .map_ok(|input| displace(opts.clone(), input))
    .try_buffer_unordered(threads);
  let out_stream = stream_sink(&opts, trans_stream.boxed());

  out_stream.try_for_each(|()| ready(Ok(()))).await
}

fn main() -> impl Termination {
  let threads = available_parallelism().map_or(6, Into::into);
  let rt = Builder::new_multi_thread()
    .enable_io()
    .build()
    .expect("runtime failure");

  match rt.block_on(run(threads)).err() {
    None => ExitCode::SUCCESS,
    Some(Die::Interrupt) => exit(130),
    Some(e) => {
      eprintln!("{}", Colour::Red.paint(format!("{e}")));
      ExitCode::FAILURE
    }
  }
}
