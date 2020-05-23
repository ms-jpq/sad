use super::argparse::Arguments;
use super::errors::*;
use super::types::Task;
use super::udiff::DiffRange;
use async_std::sync::{channel, Receiver};
use regex::Regex;
use std::{collections::HashSet, convert::TryFrom, path::PathBuf};
use tokio::{
  io::{self, AsyncBufReadExt, BufReader},
  task,
};

pub enum Payload {
  Entire(PathBuf),
  Piecewise(PathBuf, HashSet<DiffRange>),
}

impl Arguments {
  pub fn stream(&self) -> (Task, Receiver<SadResult<Payload>>) {
    if let Some(preview) = &self.internal_preview {
      stream_preview(&self)
    }
    else if let Some(patch) = &self.internal_patch {
      stream_patch(&self)
    }
    else if self.input.is_empty() {
      stream_stdin(&self)
    } else {
      stream_input(&self)
    }
  }
}
impl TryFrom<&str> for DiffRange {
  type Error = Failure;

  fn try_from(candidate: &str) -> SadResult<Self> {
    let preg = r"^@@ -(\d+),(\d+) \+(\d+),(\d+) @@$";
    let re = Regex::new(preg).into_sadness()?;
    let captures = re
      .captures(candidate)
      .ok_or_else(|| Failure::Parse(candidate.into()))?;
    let before_start = captures
      .get(1)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let before_inc = captures
      .get(2)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let after_start = captures
      .get(3)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let after_inc = captures
      .get(4)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;

    Ok(DiffRange {
      before: (before_start - 1, before_inc),
      after: (after_start - 1, after_inc),
    })
  }
}


fn stream_preview(args: &Arguments) -> (Task, Receiver<SadResult<Payload>>) {
  let (tx, rx) = channel::<SadResult<Payload>>(1);
  let handle = task::spawn(async move {
    for path in Vec::new() {
      tx.send(Ok(Payload::Entire(path))).await;
    }
  });
  (handle, rx)
}

fn stream_patch(args: &Arguments) -> (Task, Receiver<SadResult<Payload>>) {
  let (tx, rx) = channel::<SadResult<Payload>>(1);
  let handle = task::spawn(async move {
    for path in Vec::new() {
      tx.send(Ok(Payload::Entire(path))).await;
    }
  });
  (handle, rx)
}

fn p_path(name: &[u8]) -> SadResult<PathBuf> {
  String::from_utf8(name.to_vec())
    .map(|p| PathBuf::from(p.as_str()))
    .into_sadness()
}

fn stream_stdin(args: &Arguments) -> (Task, Receiver<SadResult<Payload>>) {
  let delim = if args.nul_delim { b'\0' } else { b'\n' };
  let (tx, rx) = channel::<SadResult<Payload>>(1);
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
          let step = path.map(Payload::Entire);
          tx.send(step).await;
        }
        Err(err) => tx.send(Err(err)).await,
      }
    }
  });
  (handle, rx)
}

fn stream_input(args: &Arguments) -> (Task, Receiver<SadResult<Payload>>) {
  let paths = args.input.clone();
  let (tx, rx) = channel::<SadResult<Payload>>(1);
  let handle = task::spawn(async move {
    for path in paths {
      tx.send(Ok(Payload::Entire(path))).await;
    }
  });
  (handle, rx)
}
