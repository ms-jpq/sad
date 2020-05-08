use argparse::{Arguments, Options};
use async_std::{
  io,
  path::PathBuf,
  prelude::*,
  sync::{channel, Receiver},
  task::{self, JoinHandle},
};
use clap::Clap;
use errors::*;
use std::process;
mod argparse;
mod displace;
mod errors;

fn stream_stdin(args: &Arguments) -> (JoinHandle<SadResult<()>>, Receiver<Vec<u8>>) {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let (s, r) = channel::<Vec<u8>>(1);
  let mut reader = io::BufReader::new(io::stdin());
  let handle = task::spawn(async move {
    loop {
      let mut buf = Vec::new();
      let n = reader.read_until(delim, &mut buf).await.halp()?;
      if n == 0 {
        return SadResult::Ok(());
      } else {
        buf.pop();
        s.send(buf).await;
      }
    }
  });
  (handle, r)
}

fn p_path(name: Vec<u8>) -> SadResult<PathBuf> {
  let path = String::from_utf8(name).halp()?;
  Ok(PathBuf::from(&path[..]))
}

fn stream_stdout(receiver: Receiver<String>) -> JoinHandle<()> {
  task::spawn(async move {
    while let Some(diff) = receiver.recv().await {
      println!("{}", diff);
    }
  })
}

fn main() {
  let args = Arguments::parse();
  let (reader, receiver) = stream_stdin(&args);
  // let writer = stream_stdout(receiver);

  match Options::new(args) {
    Ok(opts) => {
      println!("123 - 123 - 123");
    }
    Err(e) => {
      eprintln!("{}", e);
      process::exit(1);
    }
  }

  task::block_on(async {
    // let _owo = reader.join(writer).await;
  })
}
