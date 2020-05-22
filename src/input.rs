use super::argparse::Arguments;
use super::errors::*;
use super::types::Task;
use async_std::sync::{channel, Receiver};
use std::path::PathBuf;
use tokio::{
  io::{self, AsyncBufReadExt, BufReader},
  task,
};

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

pub fn choose_input(args: &Arguments) -> (Task, Receiver<SadResult<PathBuf>>) {
  if args.input.is_empty() {
    stream_stdin(&args)
  } else {
    stream_list(args.input.clone())
  }
}
