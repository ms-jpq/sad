use argparse::{Arguments, Options};
use async_std::{
  io,
  path::PathBuf,
  prelude::*,
  sync::{channel, Receiver},
  task::{self, JoinHandle},
};
use clap::Clap;
use displace::Displaced;
use errors::*;
use futures::future::{join, join3, JoinAll};
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
  Ok(PathBuf::from(path.as_str()))
}

fn stream_displace(
  opts: Options,
  receiver: Receiver<Vec<u8>>,
) -> (JoinHandle<SadResult<()>>, Receiver<Displaced>) {
  let (s, r) = channel::<Displaced>(1);
  let handle = task::spawn(async move {
    while let Some(name) = receiver.recv().await {
      let path = p_path(name)?;
      let displaced = displace::displace(path, &opts);
    }
    return SadResult::Ok(());
  });
  (handle, r)
}

fn stream_stdout(receiver: Receiver<Displaced>) -> JoinHandle<()> {
  task::spawn(async move { while let Some(diff) = receiver.recv().await {} })
}

fn main() {
  let args = Arguments::parse();
  let (reader, path_receiver) = stream_stdin(&args);
  match Options::new(args) {
    Ok(opts) => {
      let (intermediary, displaced_receiver) = stream_displace(opts, path_receiver);
      let writer = stream_stdout(displaced_receiver);

      task::block_on(async {
        let joined = join3(reader, writer, intermediary);
        let _ = joined.await;
      })
    }
    Err(e) => {
      eprintln!("{}", e);
      process::exit(1);
    }
  }
}
