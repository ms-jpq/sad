use super::argparse::Arguments;
use super::types::{Abort, Failure};
use super::udiff::DiffRange;
use async_channel::{bounded, Receiver};
use regex::Regex;
use std::{
  collections::{HashMap, HashSet},
  convert::TryFrom,
  error::Error,
  ffi::OsString,
  os::unix::ffi::OsStringExt,
  path::PathBuf,
};
use tokio::{
  fs::{canonicalize, File},
  io::{self, AsyncBufReadExt, BufReader},
  select,
  task::{spawn, JoinHandle},
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
    let captures = re
      .captures(candidate)
      .ok_or_else(|| Failure::Sucks(String::new()))?;
    let before_start = captures
      .get(1)
      .ok_or_else(|| Failure::Sucks(String::new()))?
      .as_str()
      .parse::<usize>()?;
    let before_inc = captures
      .get(2)
      .ok_or_else(|| Failure::Sucks(String::new()))?
      .as_str()
      .parse::<usize>()?;
    let after_start = captures
      .get(3)
      .ok_or_else(|| Failure::Sucks(String::new()))?
      .as_str()
      .parse::<usize>()?;
    let after_inc = captures
      .get(4)
      .ok_or_else(|| Failure::Sucks(String::new()))?
      .as_str()
      .parse::<usize>()?;

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
  let fd = File::open(path).await?;
  let mut reader = BufReader::new(fd);
  let mut acc = HashMap::new();

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

fn stream_patch(abort: &Abort, patch: PathBuf) -> (JoinHandle<()>, Receiver<Payload>) {
  let (tx, rx) = bounded::<Payload>(1);
  let handle = spawn(async move {
    match read_patches(&patch).await {
      Ok(patches) => {
        for patch in patches {
          match tx.send(Payload::Piecewise(patch.0, patch.1)).await {
            Err(err) => {
              abort.send(Box::new(err));
              break;
            }
            _ => (),
          }
        }
      }
      Err(err) => abort.send(Box::new(err)),
    }
  });
  (handle, rx)
}

fn stream_stdin(abort: &Abort, use_nul: bool) -> (JoinHandle<()>, Receiver<Payload>) {
  let (tx, rx) = bounded::<Payload>(1);

  let handle = spawn(async move {
    if atty::is(atty::Stream::Stdin) {
      abort.send(Box::new(Failure::Sucks(String::new())));
    } else {
      let delim = if use_nul { b'\0' } else { b'\n' };
      let mut on_abort = abort.subscribe();
      let mut reader = BufReader::new(io::stdin());
      let mut seen = HashSet::new();
      loop {
        let mut buf = Vec::new();
        select! {
          _ = on_abort.recv() => break,
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
                abort.send(Box::new(err));
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

pub fn stream_input(abort: &Abort, args: &Arguments) -> (JoinHandle<()>, Receiver<Payload>) {
  if let Some(preview) = &args.internal_preview {
    stream_patch(abort, preview.clone())
  } else if let Some(patch) = &args.internal_patch {
    stream_patch(abort, patch.clone())
  } else {
    stream_stdin(abort, args.nul_delim)
  }
}
