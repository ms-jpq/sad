use ansi_term::Colour;
use argparse::{Arguments, Options};
use clap::Clap;
use errors::*;
use std::future::Future;
use std::{path::PathBuf, process};
use tokio::{
  io,
  prelude::*,
  runtime,
  stream::{Stream, StreamExt},
  sync::mpsc,
  task::{self, JoinHandle},
};

mod argparse;
mod displace;
mod errors;
mod udiff;

fn stream_list(
  paths: Vec<PathBuf>,
) -> (
  JoinHandle<SadResult<()>>,
  mpsc::Receiver<SadResult<PathBuf>>,
) {
  let (mut tx, rx) = mpsc::channel::<SadResult<PathBuf>>(1);
  let handle = task::spawn(async move {
    for path in paths {
      tx.send(Ok(path)).await?;
    }
    Ok(())
  });
  (handle, rx)
}

fn p_path(name: &[u8]) -> SadResult<PathBuf> {
  String::from_utf8(name.to_vec())
    .map(|p| PathBuf::from(p.as_str()))
    .into_sadness()
}

fn stream_stdin(
  args: &Arguments,
) -> (
  JoinHandle<SadResult<()>>,
  mpsc::Receiver<SadResult<PathBuf>>,
) {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let (mut tx, rx) = mpsc::channel::<SadResult<PathBuf>>(1);
  let mut reader = io::BufReader::new(io::stdin());
  let mut buf = Vec::new();

  let handle = task::spawn(async move {
    loop {
      let line = reader.read_until(delim, &mut buf).await.into_sadness()?;
      match line {
        0 => return Ok(()),
        _ => {
          buf.pop();
          let path = p_path(&buf);
          tx.send(path).await?;
        }
      }
    }
  });

  (handle, rx)
}

fn choose_input(
  args: &Arguments,
) -> (
  JoinHandle<SadResult<()>>,
  mpsc::Receiver<SadResult<PathBuf>>,
) {
  if args.input.is_empty() {
    stream_stdin(&args)
  } else {
    stream_list(args.input.clone())
  }
}

fn stream_process(stream: mpsc::Receiver<SadResult<PathBuf>>) {}

fn stream_stdout(mut stream: mpsc::Receiver<SadResult<String>>) -> JoinHandle<SadResult<()>> {
  let mut stdout = io::BufWriter::new(io::stdout());
  task::spawn(async move {
    while let Some(res) = stream.next().await {
      match res {
        Ok(print) => match stdout.write(print.as_bytes()).await {
          Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => process::exit(1),
          Err(e) => err_exit(e.into()),
          _ => {}
        },
        Err(err) => err_exit(err),
      };
    }
    stdout.flush().await.into_sadness()?;
    Ok(())
  })
}

fn err_exit(err: Failure) -> ! {
  eprintln!("{}", Colour::Red.paint(format!("{:#?}", err)));
  process::exit(1)
}

fn main() {
  // let mut rt = runtime::Builder::new().build().unwrap();
  // let args = Arguments::parse();
  // let (reader, receiver) = choose_input(&args);
  // match Options::new(args) {
  //   Ok(opts) => {
  //     let writer = stream_stdout(receiver);
  //     rt.block_on(async {
  //       let _lmao = join(reader, writer).await;
  //     })
  //   }
  //   Err(e) => err_exit(e),
  // }
}
