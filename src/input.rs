use super::argparse::Arguments;
use super::errors::{Failure, SadResult, SadnessFrom};
use super::types::Task;
use super::udiff::DiffRange;
use async_channel::{bounded, Receiver};
use regex::Regex;
use std::{
  collections::{HashMap, HashSet},
  convert::TryFrom,
  path::PathBuf,
};
use tokio::{
  fs::File,
  io::{self, AsyncBufReadExt, BufReader},
  task,
};

#[derive(Debug)]
pub enum Payload {
  Entire(PathBuf),
  Piecewise(PathBuf, HashSet<DiffRange>),
}

impl Arguments {
  pub fn stream(&self) -> (Task, Receiver<SadResult<Payload>>) {
    if let Some(preview) = &self.internal_preview {
      stream_patch(preview.clone())
    } else if let Some(patch) = &self.internal_patch {
      stream_patch(patch.clone())
    } else {
      stream_stdin(self.nul_delim)
    }
  }
}

fn p_path(name: Vec<u8>) -> SadResult<PathBuf> {
  String::from_utf8(name)
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
    let name = re.replace(candidate, "").as_bytes().to_vec();
    let buf = p_path(name)?;
    Ok(DiffLine(buf, range))
  }
}

async fn read_patches(path: &PathBuf) -> SadResult<HashMap<PathBuf, HashSet<DiffRange>>> {
  let mut acc: HashMap<PathBuf, HashSet<DiffRange>> = HashMap::new();
  let fd = File::open(path).await.into_sadness()?;
  let mut reader = BufReader::new(fd);

  loop {
    let mut buf = Vec::new();
    let n = reader.read_until(b'\0', &mut buf).await.into_sadness()?;
    match n {
      0 => break,
      _ => {
        buf.pop();
        let line = String::from_utf8(buf).into_sadness()?;
        let patch = DiffLine::try_from(line.as_str()).into_sadness()?;
        match acc.get_mut(&patch.0) {
          Some(ranges) => {
            ranges.insert(patch.1);
          }
          None => {
            let mut ranges = HashSet::new();
            ranges.insert(patch.1);
            acc.insert(patch.0, ranges);
          }
        }
      }
    }
  }

  Ok(acc)
}

fn stream_patch(patch: PathBuf) -> (Task, Receiver<SadResult<Payload>>) {
  let (tx, rx) = bounded::<SadResult<Payload>>(1);
  let handle = task::spawn(async move {
    match read_patches(&patch).await {
      Ok(patches) => {
        for patch in patches {
          tx.send(Ok(Payload::Piecewise(patch.0, patch.1))).await
        }
      }
      Err(err) => tx.send(Err(err)).await,
    }
  });
  (handle, rx)
}

fn stream_stdin(use_nul: bool) -> (Task, Receiver<SadResult<Payload>>) {
  let (tx, rx) = bounded::<SadResult<Payload>>(1);
  let handle = task::spawn(async move {
    let delim = if use_nul { b'\0' } else { b'\n' };
    let mut reader = BufReader::new(io::stdin());
    if atty::is(atty::Stream::Stdin) {
      tx.send(Err(Failure::NilStdin)).await
    }
    loop {
      let mut buf = Vec::new();
      let n = reader.read_until(delim, &mut buf).await.into_sadness();
      match n {
        Ok(0) => return,
        Ok(_) => {
          buf.pop();
          let path = p_path(buf);
          let step = path.map(Payload::Entire);
          tx.send(step).await;
        }
        Err(err) => tx.send(Err(err)).await,
      }
    }
  });
  (handle, rx)
}
