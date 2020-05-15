use ansi_term::Colour;
use argparse::{Arguments, Options, SubprocessCommand};
use async_std::sync::{channel, Arc, Receiver, Sender};
use clap::Clap;
use errors::*;
use futures::future::{try_join, try_join3, try_join_all, TryJoin, TryJoinAll};
use std::{
  path::PathBuf,
  process::{self, Stdio},
};
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
  prelude::*,
  process::{Child, Command},
  runtime,
  task::{self, JoinHandle},
};

mod argparse;
mod displace;
mod errors;
mod udiff;

type Task = JoinHandle<()>;

fn stream_list(paths: Vec<PathBuf>) -> (Task, Receiver<SadResult<PathBuf>>) {
  let (tx, rx) = channel::<SadResult<PathBuf>>(1);
  let handle = task::spawn(async move {
    for path in paths {
      tx.send(Ok(path)).await;
    }
  });
  (handle, rx)
}

fn p_path(name: &[u8]) -> SadResult<PathBuf> {
  String::from_utf8(name.to_vec())
    .map(|p| PathBuf::from(p.as_str()))
    .into_sadness()
}

fn stream_stdin(args: &Arguments) -> (Task, Receiver<SadResult<PathBuf>>) {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let (tx, rx) = channel::<SadResult<PathBuf>>(1);
  let mut reader = BufReader::new(io::stdin());
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
          tx.send(path).await;
        }
        Err(err) => tx.send(Err(err)).await,
      }
    }
  });

  (handle, rx)
}

fn choose_input(args: &Arguments) -> (Task, Receiver<SadResult<PathBuf>>) {
  if args.input.is_empty() {
    stream_stdin(&args)
  } else {
    stream_list(args.input.clone())
  }
}

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

fn stream_pager(cmd: &SubprocessCommand, stream: Receiver<SadResult<String>>) -> Task {
  let subprocess = Command::new(&cmd.program)
    .args(&cmd.arguments)
    .stdin(Stdio::piped())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .spawn();

  let child = match subprocess {
    Ok(child) => child,
    Err(err) => err_exit(err.into()),
  };
  let mut stdin = BufWriter::new(child.stdin.unwrap());

  task::spawn(async move {
    while let Some(print) = stream.recv().await {
      match print {
        Ok(val) => {
          if let Err(e) = stdin.write(val.as_bytes()).await {
            err_exit(e.into())
          }
        }
        Err(e) => err_exit(e),
      }
    }
    stdin.flush().await.unwrap();
  })
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
    stdout.flush().await.unwrap()
  })
}

fn stream_output(cmd: Option<SubprocessCommand>, stream: Receiver<SadResult<String>>) -> Task {
  match cmd {
    Some(cmd) => stream_pager(&cmd, stream),
    None => stream_stdout(stream),
  }
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
