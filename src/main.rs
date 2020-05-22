use ansi_term::Colour;
use argparse::{Arguments, Options};
use async_std::sync::{channel, Receiver, Sender};
use errors::*;
use futures::future::{try_join, try_join3, try_join_all, TryJoinAll};
use std::{path::PathBuf, process, sync::Arc};
use structopt::StructOpt;
use subprocess::SubprocessCommand;
use tokio::{
  io::{self, AsyncWriteExt, BufWriter},
  runtime, task,
};
use types::Task;

mod argparse;
mod displace;
mod errors;
mod input;
mod subprocess;
mod types;
mod udiff;

fn stream_process(
  opts: &Options,
  stream: Receiver<SadResult<PathBuf>>,
) -> (TryJoinAll<Task>, Receiver<SadResult<String>>) {
  let oo = Arc::new(opts.clone());
  let (tx, rx) = channel::<SadResult<String>>(1);

  let handles = (1..=num_cpus::get() * 2)
    .map(|_| {
      let stream = Receiver::clone(&stream);
      let opts = Arc::clone(&oo);
      let sender = Sender::clone(&tx);

      task::spawn(async move {
        while let Some(path) = stream.recv().await {
          match path {
            Ok(val) => {
              let displaced = displace::displace(val, &opts).await;
              sender.send(displaced).await
            }
            Err(err) => sender.send(Err(err)).await,
          }
        }
      })
    })
    .collect::<Vec<Task>>();
  let handle = try_join_all(handles);
  (handle, rx)
}

fn stream_stdout(stream: Receiver<SadResult<String>>) -> Task {
  let mut stdout = BufWriter::new(io::stdout());
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
    stdout.shutdown().await.unwrap()
  })
}

fn stream_output(cmd: Option<SubprocessCommand>, stream: Receiver<SadResult<String>>) -> Task {
  match cmd {
    Some(cmd) => {
      let (child, rx) = cmd.stream(stream);
      let recv = stream_stdout(rx);
      task::spawn(async {
        if let Err(e) = try_join(child, recv).await {
          err_exit(e.into())
        }
      })
    }
    None => stream_stdout(stream),
  }
}

fn main() {
  let mut rt = runtime::Builder::new()
    .threaded_scheduler()
    .enable_io()
    .build()
    .unwrap();
  rt.block_on(async {
    let args = Arguments::from_args();
    let (reader, receiver) = args.stream();
    let end = match Options::new(args) {
      Ok(opts) => {
        let (steps, rx) = stream_process(&opts, receiver);
        let writer = stream_output(opts.pager, rx);
        try_join3(reader, steps, writer).await
      }
      Err(e) => err_exit(e),
    };
    if let Err(err) = end {
      err_exit(err.into())
    }
  })
}

/* Exit */

pub fn err_exit(err: Failure) -> ! {
  eprintln!("{}", Colour::Red.paint(format!("{:#?}", err)));
  process::exit(1)
}
