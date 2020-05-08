use argparse::{Arguments, Options};
use async_std::{
  io,
  path::PathBuf,
  prelude::*,
  sync::{channel, Receiver},
  task::{self, JoinHandle},
};
use clap::Clap;
use std::process;
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

fn p_path(name: Vec<u8>) -> Result<PathBuf, std::string::FromUtf8Error> {
  match String::from_utf8(name) {
    Ok(path) => Ok(PathBuf::from(&path[..])),
    Err(e) => Err(e),
  }
}

fn stream_stdout(receiver: Receiver<Vec<u8>>) -> JoinHandle<()> {
  task::spawn(async move {
    while let Some(buf) = receiver.recv().await {
      let path = p_path(buf).unwrap();
      println!("{}", path.to_str().unwrap());
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
    let _owo = reader.join(writer).await;
  })
}
