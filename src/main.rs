use argparse::{Arguments, Options};
use async_std::prelude::*;
use async_std::{
  future, io,
  sync::{channel, Receiver},
  task::{self, JoinHandle},
};
use clap::Clap;
use std::process;
use std::time::Duration;
mod argparse;

fn stream_stdin(args: &Arguments) -> (JoinHandle<io::Result<()>>, Receiver<Vec<u8>>) {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let (s, r) = channel::<Vec<u8>>(1);
  let mut reader = io::BufReader::new(io::stdin());
  let handle = task::spawn(async move {
    loop {
      let mut buf = Vec::new();
      let n = reader.read_until(delim, &mut buf).await?;
      if n == 0 {
        return io::Result::Ok(());
      } else {
        buf.pop();
        s.send(buf).await;
      }
    }
  });
  (handle, r)
}

fn stream_stdout(receiver: Receiver<Vec<u8>>) -> JoinHandle<()> {
  task::spawn(async move {
    while let Some(buf) = receiver.recv().await {
      if let Ok(s) = String::from_utf8(buf) {
        println!("{}", s)
      }
    }
  })
}

fn main() {
  let args = Arguments::parse();
  let (reader, receiver) = stream_stdin(&args);
  let writer = stream_stdout(receiver);

  match Options::new(args) {
    Ok(opts) => {
      println!("{:?}", opts);
    }
    Err(e) => {
      eprintln!("{}", e);
      process::exit(1);
    }
  }

  task::block_on(async {
    reader.join(writer).await;
  })
}
