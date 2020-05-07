use argparse::{Arguments, Options};
use async_std::prelude::*;
use async_std::{
  io,
  task::{self, JoinHandle},
};
use clap::Clap;
use std::process;
mod argparse;

fn stream_stdin(args: &Arguments) -> JoinHandle<io::Result<()>> {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let handle = task::spawn(async move {
    let mut reader = io::BufReader::new(io::stdin());
    loop {
      let mut buf = Vec::new();
      let n = reader.read_until(delim, &mut buf).await?;
      if n == 0 {
        return io::Result::Ok(());
      }
    }
  });
  handle
}

fn main() {
  let args = Arguments::parse();
  let reader = stream_stdin(&args);

  match Options::new(args) {
    Ok(opts) => {
      println!("{:?}", opts);
    }
    Err(e) => {
      eprintln!("{}", e);
      process::exit(1);
    }
  }

  task::block_on(async { reader.await.unwrap() })
}
