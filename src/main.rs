use ansi_term::Colour;
use argparse::{Arguments, Options};
use clap::Clap;
use errors::*;
use futures::future::{join_all, JoinAll};
use std::{path::PathBuf, process, sync::Arc};
use tokio::{
  io,
  prelude::*,
  runtime,
  stream::{Stream, StreamExt},
  sync::{mpsc, Mutex},
  task::{self, JoinHandle},
};

mod argparse;
mod displace;
mod errors;
mod udiff;

fn stream_list(paths: Vec<PathBuf>) -> (JoinHandle<SadResult<()>>, mpsc::Receiver<PathBuf>) {
  let (mut tx, rx) = mpsc::channel::<PathBuf>(1);
  let handle = task::spawn(async move {
    for path in paths {
      tx.send(path).await?;
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

fn stream_stdin(args: &Arguments) -> (JoinHandle<SadResult<()>>, mpsc::Receiver<PathBuf>) {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let (mut tx, rx) = mpsc::channel::<PathBuf>(1);
  let mut reader = io::BufReader::new(io::stdin());
  let mut buf = Vec::new();

  let handle = task::spawn(async move {
    loop {
      let line = reader.read_until(delim, &mut buf).await.into_sadness()?;
      match line {
        0 => return Ok(()),
        _ => {
          buf.pop();
          let path = p_path(&buf)?;
          tx.send(path).await?;
        }
      }
    }
  });

  (handle, rx)
}

fn choose_input(args: &Arguments) -> (JoinHandle<SadResult<()>>, mpsc::Receiver<PathBuf>) {
  if args.input.is_empty() {
    stream_stdin(&args)
  } else {
    stream_list(args.input.clone())
  }
}

fn stream_process(
  opts: Options,
  stream: mpsc::Receiver<PathBuf>,
) -> (JoinAll<JoinHandle<SadResult<()>>>, mpsc::Receiver<String>) {
  let sx = Arc::new(Mutex::new(stream));
  let oo = Arc::new(opts);
  let (tx, rx) = mpsc::channel::<String>(1);

  let threads = num_cpus::get() * 2;
  let handles = (1..=threads)
    .map(|_| {
      let stream = Arc::clone(&sx);
      let opts = Arc::clone(&oo);
      let mut sender = mpsc::Sender::clone(&tx);

      task::spawn(async move {
        while let Some(path) = stream.lock().await.next().await {
          let displaced = displace::displace(path, &opts).await?;
          sender.send(displaced).await.into_sadness()?;
        }
        Ok(())
      })
    })
    .collect::<Vec<JoinHandle<SadResult<()>>>>();
  let handle = join_all(handles);
  (handle, rx)
}

fn stream_stdout(mut stream: mpsc::Receiver<String>) -> JoinHandle<SadResult<()>> {
  let mut stdout = io::BufWriter::new(io::stdout());
  task::spawn(async move {
    while let Some(print) = stream.next().await {
      match stdout.write(print.as_bytes()).await {
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => process::exit(1),
        Err(e) => return Err(e.into()),
        _ => {}
      }
    }
    stdout.flush().await.into_sadness()
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
