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
  argparse::{parse_args, parse_opts, Options},
  displace::displace,
  futures::stream::{BoxStream, StreamExt, TryStreamExt},
  input::{stream_in, LineIn},
  output::stream_out,
  std::{
    convert::Into,
    ffi::OsString,
    process::{ExitCode, Termination},
    sync::Arc,
    thread::available_parallelism,
  },
  tokio::runtime::Builder,
  types::{Abort, Fail},
};

async fn run(threads: usize) -> Result<(), Fail> {
  let (mode, args) = parse_args();
  let input_stream = stream_in(&mode, &args).await;
  let opts = parse_opts(mode, args)?;
  let options = Arc::new(opts);
  let trans_stream = BoxStream::from(input_stream)
    .map_ok(move |input| {
      let opts = options.clone();
      async move { displace(&opts, input).await }
    })
    .try_buffer_unordered(threads);
  //let h_3 = stream_out(abort, &opts, trans_stream);
  //try_join3(h_1, h_2, h_3).await?;
  Ok(())
}

fn main() -> impl Termination {
  let threads = available_parallelism().map(Into::into).unwrap_or(6);
  let rt = Builder::new_multi_thread()
    .enable_io()
    .max_blocking_threads(threads)
    .build()
    .expect("runtime failure");

  let errors = rt.block_on(async {
    let abort = Abort::new();
    if let Err(err) = run(threads).await {
      let mut errs = abort.fin().await;
      errs.push(err);
      errs
    } else {
      abort.fin().await
    }
  });

  match errors[..] {
    [] => ExitCode::SUCCESS,
    [Fail::Interrupt] => ExitCode::from(130),
    _ => {
      for err in errors {
        eprintln!("{}", Colour::Red.paint(format!("{err}")));
      }
      ExitCode::FAILURE
    }
  }
}
