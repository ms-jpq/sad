use ansi_term::Colour;
use argparse::{Arguments, Options};
use async_std::{
  io,
  path::PathBuf,
  prelude::*,
  sync::{channel, Arc, Receiver},
  task::{self, JoinHandle},
};
use clap::Clap;
use errors::*;
use futures::future::{join3, join_all, JoinAll};
use std::process;

mod argparse;
mod displace;
mod errors;
mod udiff;

fn stream_stdin(args: &Arguments) -> (JoinHandle<()>, Receiver<SadResult<Vec<u8>>>) {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let (s, r) = channel::<SadResult<Vec<u8>>>(1);
  let mut reader = io::BufReader::new(io::stdin());

  let handle = task::spawn(async move {
    loop {
      let mut buf = Vec::new();
      let read = reader.read_until(delim, &mut buf).await.halp();
      match read {
        Ok(0) => return,
        Ok(_) => {
          buf.pop();
          s.send(SadResult::Ok(buf)).await;
        }
        Err(e) => s.send(SadResult::Err(e)).await,
      }
    }
  });

  (handle, r)
}

fn p_path(name: Vec<u8>) -> SadResult<PathBuf> {
  let path = String::from_utf8(name).halp()?;
  Ok(PathBuf::from(path.as_str()))
}

fn stream_displace(
  opts: Options,
  receiver: Receiver<SadResult<Vec<u8>>>,
) -> (JoinAll<JoinHandle<()>>, Receiver<SadResult<String>>) {
  let (s, r) = channel::<SadResult<String>>(1);
  let rr = Arc::new(receiver);
  let ss = Arc::new(s);
  let oo = Arc::new(opts);

  let threads = num_cpus::get() * 2;
  let handles = (1..=threads)
    .map(|_| {
      let receiver = Arc::clone(&rr);
      let sender = Arc::clone(&ss);
      let opts = Arc::clone(&oo);

      task::spawn(async move {
        while let Some(name) = receiver.recv().await {
          let path = name.and_then(p_path);
          match path {
            Ok(val) => {
              let displaced = displace::displace(val, &opts).await;
              sender.send(displaced).await;
            }
            Err(e) => {
              sender.send(SadResult::Err(e)).await;
            }
          }
        }
      })
    })
    .collect::<Vec<JoinHandle<()>>>();

  let handle = join_all(handles);
  (handle, r)
}

fn stream_stdout(receiver: Receiver<SadResult<String>>) -> JoinHandle<()> {
  let mut stdout = io::BufWriter::new(io::stdout());
  task::spawn(async move {
    while let Some(res) = receiver.recv().await {
      match res {
        Ok(print) => match stdout.write(print.as_bytes()).await {
          Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => process::exit(1),
          _ => {}
        },
        Err(err) => err_exit(err),
      };
    }
    stdout.flush().await.unwrap();
  })
}

fn err_exit(err: Failure) -> ! {
  eprintln!("{}", Colour::Red.paint(format!("{:#?}", err)));
  process::exit(1)
}

fn main() {
  let args = Arguments::parse();
  let (reader, path_receiver) = stream_stdin(&args);
  match Options::new(args) {
    Ok(opts) => {
      let (intermediary, displaced_receiver) = stream_displace(opts, path_receiver);
      let writer = stream_stdout(displaced_receiver);
      task::block_on(async {
        join3(reader, writer, intermediary).await;
      })
    }
    Err(e) => err_exit(e),
  }
}
