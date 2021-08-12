use super::argparse::Arguments;
use super::errors::Failure;
use super::types::{Abort, Task};
use super::udiff::DiffRange;
use async_channel::{bounded, Receiver};
use regex::Regex;
use std::os::unix::ffi::OsStringExt;
use std::{
  collections::{HashMap, HashSet},
  convert::TryFrom,
  error::Error,
  ffi::OsString,
  path::PathBuf,
};
use tokio::{
  fs::{canonicalize, File},
  io::{self, AsyncBufReadExt, BufReader},
  select, task,
};

#[derive(Debug)]
pub enum Payload {
  Entire(PathBuf),
  Piecewise(PathBuf, HashSet<DiffRange>),
}

fn p_path(name: Vec<u8>) -> PathBuf {
  PathBuf::from(OsString::from_vec(name))
}

struct DiffLine(PathBuf, DiffRange);

impl TryFrom<&str> for DiffLine {
  type Error = Failure;

  fn try_from(candidate: &str) -> Result<Self, Box<dyn Error>> {
    let preg = "\n\n\n\n@@ -(\\d+),(\\d+) \\+(\\d+),(\\d+) @@$";
    let re = Regex::new(preg)?;
    let captures = re.captures(candidate).ok_or_else(|| Failure::Sucks(String::new()))?;
    let before_start = captures
      .get(1)
      .ok_or_else(|| Failure::Sucks(String::new()))?
      .as_str()
      .parse::<usize>()
      ?;
    let before_inc = captures
      .get(2)
      .ok_or_else(|| Failure::Sucks(String::new()))?
      .as_str()
      .parse::<usize>()
      ?;
    let after_start = captures
      .get(3)
      .ok_or_else(|| Failure::Sucks(String::new()))?
      .as_str()
      .parse::<usize>()
      ?;
    let after_inc = captures
      .get(4)
      .ok_or_else(|| Failure::Sucks(String::new()))?
      .as_str()
      .parse::<usize>()
      ?;

    let range = DiffRange {
      before: (before_start - 1, before_inc),
      after: (after_start - 1, after_inc),
    };
    let name = re.replace(candidate, "").as_bytes().to_vec();
    let buf = p_path(name);
    Ok(DiffLine(buf, range))
  }
}

async fn read_patches(
  path: &PathBuf,
) -> Result<HashMap<PathBuf, HashSet<DiffRange>>, Box<dyn Error>> {
  let mut acc: HashMap<PathBuf, HashSet<DiffRange>> = HashMap::new();
  let fd = File::open(path).await?;
  let mut reader = BufReader::new(fd);

  loop {
    let mut buf = Vec::new();
    let n = reader.read_until(b'\0', &mut buf).await?;
    match n {
      0 => break,
      _ => {
        buf.pop();
        let line = String::from_utf8(buf)?;
        let patch = DiffLine::try_from(line.as_str())?;
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

fn stream_patch(abort: Abort, patch: PathBuf) -> (Task, Receiver<Payload>) {
  let (tx, rx) = bounded::<Payload>(1);
  let handle = task::spawn(async move {
    match read_patches(&patch).await {
      Ok(patches) => {
        for patch in patches {
          tx.send(Payload::Piecewise(patch.0, patch.1))
            .await
            .expect("<CHAN>")
        }
      }
      Err(err) => abort.tx.send(err).expect("<CHAN>"),
    }
  });
  (handle, rx)
}

fn stream_stdin(abort: Abort, use_nul: bool) -> (Task, Receiver<Payload>) {
  let (tx, rx) = bounded::<Payload>(1);
  let handle = task::spawn(async move {
    let delim = if use_nul { b'\0' } else { b'\n' };
    let mut reader = BufReader::new(io::stdin());
    if atty::is(atty::Stream::Stdin) {
      abort
        .tx
        .send(Box::new(Failure::Sucks(String::new())))
        .expect("<CHAN>")
    } else {
      let mut seen = HashSet::new();
      loop {
        let mut buf = Vec::new();
        select! {
          _ = abort.rx.changed() => break,
          n = reader.read_until(delim, &mut buf) => {

        match n {
          Ok(0) => break,
          Ok(_) => {
            buf.pop();
            let path = p_path(buf);
            if let Ok(canonical) = canonicalize(&path).await {
              if seen.insert(canonical.clone()) {
                tx.send(Payload::Entire(canonical))
                  .await
                  .expect("<CHAN>")
              }
            }
          }
          Err(err) => {
            abort.tx.send(Box::new(err)).expect("<CHAN>");
            break;
          }
        }
          }
        }
      }
    }
  });
  (handle, rx)
}

impl Arguments {
  pub fn stream(&self, abort: Abort) -> (Task, Receiver<Payload>) {
    if let Some(preview) = &self.internal_preview {
      stream_patch(abort, preview.clone())
    } else if let Some(patch) = &self.internal_patch {
      stream_patch(abort, patch.clone())
    } else {
      stream_stdin(abort, self.nul_delim)
    }
  }
}
