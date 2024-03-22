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
    future::ready,
    stream::{once, select, BoxStream, Stream, StreamExt, TryStreamExt},
  },
  fzf::stream_fzf_proc,
  input::stream_in,
  std::{
    convert::Into,
    ffi::OsString,
    marker::Unpin,
    path::PathBuf,
    pin::pin,
    process::{ExitCode, Termination},
    sync::Arc,
    thread::available_parallelism,
  },
  subprocess::{stream_into, stream_subproc},
  tokio::{io, runtime::Builder, signal::ctrl_c},
  types::Fail,
};

fn stream_sink<'a>(
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

async fn consume(stream: impl Stream<Item = Result<(), Fail>> + Send + Unpin) -> Result<(), Fail> {
  let int = once(async {
    match ctrl_c().await {
      Err(e) => Fail::IO(PathBuf::new(), e.kind()),
      Ok(()) => Fail::Interrupt,
    }
  });
  let out = select(
    stream
      .filter_map(|row| async { row.err() })
      .chain(once(ready(Fail::EOF))),
    int,
  );
  let mut out = pin!(out);
  loop {
    match out.next().await {
      None | Some(Fail::EOF) => break,
      Some(Fail::Interrupt) => return Err(Fail::Interrupt),
      Some(e) => eprintln!("{}", Colour::Red.paint(format!("{e}"))),
    }
  }
  Ok(())
}

async fn run(threads: usize) -> Result<(), Fail> {
  let (mode, args) = parse_args();
  let input_stream = stream_in(&mode, &args).await;
  let opts = parse_opts(mode, args)?;
  let options = Arc::new(opts);
  let opts = options.clone();
  let trans_stream = BoxStream::from(input_stream)
    .map_ok(move |input| {
      let opts = options.clone();
      async move { displace(&opts, input).await }
    })
    .try_buffer_unordered(threads);

  let out_stream = BoxStream::from(stream_sink(&opts, trans_stream));
  consume(out_stream).await
}

fn main() -> impl Termination {
  let threads = available_parallelism().map(Into::into).unwrap_or(6);
  let rt = Builder::new_multi_thread()
    .enable_io()
    .max_blocking_threads(threads)
    .build()
    .expect("runtime failure");

  match rt.block_on(run(threads)).err() {
    None => ExitCode::SUCCESS,
    Some(Fail::Interrupt) => ExitCode::from(130),
    Some(e) => {
      eprintln!("{}", Colour::Red.paint(format!("{e}")));
      ExitCode::FAILURE
    }
  }
}
