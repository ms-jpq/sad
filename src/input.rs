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
      stream_preview(preview)
    } else if let Some(patch) = &self.internal_patch {
      stream_patch(patch)
    } else if self.input.is_empty() {
      stream_stdin(self.nul_delim)
    } else {
      stream_input(&self.input)
    }
  }
}

fn p_path(name: &[u8]) -> SadResult<PathBuf> {
  String::from_utf8(name.to_vec())
    .map(|p| PathBuf::from(p.as_str()))
    .into_sadness()
}

struct DiffLine(PathBuf, DiffRange);

impl TryFrom<&str> for DiffLine {
  type Error = Failure;

  fn try_from(candidate: &str) -> SadResult<Self> {
    let preg = "\n\n\n\n@@ -(\\d+),(\\d+) \\+(\\d+),(\\d+) @@$";
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

    let range = DiffRange {
      before: (before_start - 1, before_inc),
      after: (after_start - 1, after_inc),
    };
    let name = re.replace(candidate, "");
    let buf = p_path(&name.as_bytes())?;
    Ok(DiffLine(buf, range))
  }
}

fn stream_preview(preview: &str) -> (Task, Receiver<SadResult<Payload>>) {
  let line = DiffLine::try_from(preview);
  let (tx, rx) = channel::<SadResult<Payload>>(1);
  let handle = task::spawn(async move {
    let step = line.map(|line| {
      let mut ranges = HashSet::new();
      ranges.insert(line.1);
      Payload::Piecewise(line.0, ranges)
    });
    tx.send(step).await;
  });
  (handle, rx)
}

fn stream_patch(patch: &str) -> (Task, Receiver<SadResult<Payload>>) {
  let lines = patch
    .split_terminator('\0')
    .map(DiffLine::try_from)
    .collect::<Vec<SadResult<DiffLine>>>();
  let (tx, rx) = channel::<SadResult<Payload>>(1);
  let handle = task::spawn(async move {
    for line in lines {
      let step = line.map(|line| {
        let mut ranges = HashSet::new();
        ranges.insert(line.1);
        Payload::Piecewise(line.0, ranges)
      });
      tx.send(step).await
    }
  });
  (handle, rx)
}

fn stream_stdin(use_nul: bool) -> (Task, Receiver<SadResult<Payload>>) {
  let delim = if use_nul { b'\0' } else { b'\n' };
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

fn stream_input(paths: &[PathBuf]) -> (Task, Receiver<SadResult<Payload>>) {
  let paths = paths.to_vec();
  let (tx, rx) = channel::<SadResult<Payload>>(1);
  let handle = task::spawn(async move {
    for path in paths {
      tx.send(Ok(Payload::Entire(path))).await;
    }
  });
  (handle, rx)
}
