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
mod output;
mod subprocess;
mod types;
mod udiff;
mod udiff_spec;

use {
  ansi_term::Colour,
  argparse::{parse_args, parse_opts},
  displace::displace,
  futures::{
    sink::unfold,
    stream::{once, select, BoxStream, Stream, StreamExt, TryStreamExt},
  },
  input::stream_in,
  output::stream_sink,
  std::{
    convert::Into,
    io,
    path::PathBuf,
    process::{ExitCode, Termination},
    sync::Arc,
    thread::available_parallelism,
  },
  tokio::{runtime::Builder, signal::ctrl_c},
  types::Fail,
};

async fn consume(stream: impl Stream<Item = Result<(), Fail>> + Unpin) -> Result<(), Fail> {
  let int = once(async {
    ctrl_c()
      .await
      .map_err(|e| Fail::IO(PathBuf::new(), e.kind()))?;
    Err::<(), Fail>(Fail::Interrupt)
  });
  let sink = unfold(io::stderr(), |mut s, line| async move {
    s.write_all(&line)
      .await
      .map_err(|e| Fail::IO(PathBuf::from("/dev/stderr"), e.kind()))?;
    Ok::<(), Fail>(Some(s))
  });
  let out = select(stream, int);

  out.forward(sink).await
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
