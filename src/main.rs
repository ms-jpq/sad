use ansi_term::Colour;
use argparse::{Arguments, Options};
use clap::Clap;
use errors::*;
use futures::future::{join3, join_all, JoinAll};
use std::{path::PathBuf, process, sync::Arc};
use tokio::{
  io,
  prelude::*,
  runtime,
  sync::{mpsc, Mutex},
  task::{self, JoinHandle},
};

mod argparse;
mod displace;
mod errors;
mod udiff;

type Task = JoinHandle<()>;

fn stream_list(paths: Vec<PathBuf>) -> (Task, mpsc::Receiver<SadResult<PathBuf>>) {
  let (mut tx, rx) = mpsc::channel::<SadResult<PathBuf>>(1);
  let handle = task::spawn(async move {
    for path in paths {
      tx.send(Ok(path)).await.unwrap();
    }
  });
  (handle, rx)
}

fn p_path(name: &[u8]) -> SadResult<PathBuf> {
  String::from_utf8(name.to_vec())
    .map(|p| PathBuf::from(p.as_str()))
    .into_sadness()
}

fn stream_stdin(args: &Arguments) -> (Task, mpsc::Receiver<SadResult<PathBuf>>) {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let (mut tx, rx) = mpsc::channel::<SadResult<PathBuf>>(1);
  let mut reader = io::BufReader::new(io::stdin());
  let mut buf = Vec::new();

  let handle = task::spawn(async move {
    loop {
      let line = reader.read_until(delim, &mut buf).await.into_sadness();
      match line {
        Ok(0) => return,
        Ok(_) => {
          buf.pop();
          let path = p_path(&buf);
          buf.clear();
          tx.send(path).await.unwrap();
        }
        Err(err) => tx.send(Err(err)).await.unwrap(),
      }
    }
  });

  (handle, rx)
}

fn choose_input(args: &Arguments) -> (Task, mpsc::Receiver<SadResult<PathBuf>>) {
  if args.input.is_empty() {
    stream_stdin(&args)
  } else {
    stream_list(args.input.clone())
  }
}

fn stream_process(
  opts: Options,
  stream: mpsc::Receiver<SadResult<PathBuf>>,
) -> (JoinAll<Task>, mpsc::Receiver<SadResult<String>>) {
  let sx = Arc::new(Mutex::new(stream));
  let oo = Arc::new(opts);
  let (tx, rx) = mpsc::channel::<SadResult<String>>(1);

  let threads = num_cpus::get() * 2;
  let handles = (1..=threads)
    .map(|_| {
      let stream = Arc::clone(&sx);
      let opts = Arc::clone(&oo);
      let mut sender = mpsc::Sender::clone(&tx);

      task::spawn(async move {
        while let Some(path) = stream.lock().await.recv().await {
          match path {
            Ok(val) => {
              let displaced = displace::displace(val, &opts).await;
              sender.send(displaced).await.unwrap()
            }
            Err(err) => sender.send(Err(err)).await.unwrap(),
          }
        }
      })
    })
    .collect::<Vec<Task>>();
  let handle = join_all(handles);
  (handle, rx)
}

fn stream_stdout(mut stream: mpsc::Receiver<SadResult<String>>) -> Task {
  let mut stdout = io::BufWriter::new(io::stdout());
  task::spawn(async move {
    while let Some(print) = stream.recv().await {
      match print {
        Ok(val) => match stdout.write(val.as_bytes()).await {
          Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => process::exit(1),
          Err(e) => err_exit(e.into()),
          _ => {}
        },
        Err(e) => err_exit(e),
      }
    }
    stdout.flush().await.unwrap()
  })
}

fn err_exit(err: Failure) -> ! {
  eprintln!("{}", Colour::Red.paint(format!("{:#?}", err)));
  process::exit(1)
}

fn main() {
  let mut rt = runtime::Builder::new()
    .threaded_scheduler()
    .enable_io()
    .build()
    .unwrap();
  rt.block_on(async {
    let args = Arguments::parse();
    let (reader, receiver) = choose_input(&args);
    let end = match Options::new(args) {
      Ok(opts) => {
        let (steps, rx) = stream_process(opts, receiver);
        let writer = stream_stdout(rx);
        join3(reader, steps, writer).await
      }
      Err(e) => err_exit(e),
    };
    println!("{:#?}", end)
  })
}
