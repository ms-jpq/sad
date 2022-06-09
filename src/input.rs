use {
  super::{
    argparse::{Arguments, Mode},
    types::{Abort, Fail},
    udiff::DiffRange,
  },
  async_channel::{bounded, Receiver},
  futures::{
    future::{select, Either},
    pin_mut,
  },
  regex::Regex,
  std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
  },
  tokio::{
    fs::{canonicalize, File},
    io::{stdin, AsyncBufReadExt, BufReader},
    task::{spawn, JoinHandle},
  },
};

#[derive(Debug)]
pub enum Payload {
  Entire(PathBuf),
  Piecewise(PathBuf, HashSet<DiffRange>),
}

struct DiffLine(PathBuf, DiffRange);

fn p_line(line: &str) -> Result<DiffLine, Fail> {
  let f = Fail::ArgumentError(Default::default());
  let preg = "\n\n\n\n@@ -(\\d+),(\\d+) \\+(\\d+),(\\d+) @@$";
  let re = Regex::new(preg).map_err(Fail::RegexError)?;
  let captures = re.captures(line).ok_or_else(|| f.clone())?;

  let before_start = captures
    .get(1)
    .ok_or_else(|| f.clone())?
    .as_str()
    .parse::<usize>()
    .map_err(|_| f.clone())?;
  let before_inc = captures
    .get(2)
    .ok_or_else(|| f.clone())?
    .as_str()
    .parse::<usize>()
    .map_err(|_| f.clone())?;
  let after_start = captures
    .get(3)
    .ok_or_else(|| f.clone())?
    .as_str()
    .parse::<usize>()
    .map_err(|_| f.clone())?;
  let after_inc = captures
    .get(4)
    .ok_or_else(|| f.clone())?
    .as_str()
    .parse::<usize>()
    .map_err(|_| f.clone())?;

  let range = DiffRange {
    before: (before_start - 1, before_inc),
    after: (after_start - 1, after_inc),
  };
  let path = PathBuf::from(String::from(re.replace(line, "")));
  Ok(DiffLine(path, range))
}

async fn read_patches(path_file: &Path) -> Result<HashMap<PathBuf, HashSet<DiffRange>>, Fail> {
  let fd = File::open(path_file)
    .await
    .map_err(|e| Fail::IO(path_file.to_owned(), e.kind()))?;
  let mut reader = BufReader::new(fd);
  let mut acc = HashMap::<_, HashSet<_>>::new();

  loop {
    let mut buf = Default::default();
    let n = reader
      .read_until(b'\0', &mut buf)
      .await
      .map_err(|e| Fail::IO(path_file.to_owned(), e.kind()))?;

    if n == 0 {
      break;
    }

    buf.pop();
    let line =
      String::from_utf8(buf).map_err(|_| Fail::IO(path_file.to_owned(), ErrorKind::InvalidData))?;
    let patch = p_line(&line)?;
    if let Some(ranges) = acc.get_mut(&patch.0) {
      ranges.insert(patch.1);
    } else {
      let mut ranges = HashSet::new();
      ranges.insert(patch.1);
      acc.insert(patch.0, ranges);
    }
  }

  Ok(acc)
}

fn stream_patch(abort: &Arc<Abort>, patch: &Path) -> (JoinHandle<()>, Receiver<Payload>) {
  let abort = abort.clone();
  let patch = patch.to_owned();
  let (tx, rx) = bounded::<Payload>(1);

  let handle = spawn(async move {
    match read_patches(&patch).await {
      Ok(patches) => {
        for patch in patches {
          if tx.send(Payload::Piecewise(patch.0, patch.1)).await.is_err() {
            break;
          }
        }
      }
      Err(err) => {
        abort.send(err).await;
      }
    }
  });
  (handle, rx)
}

fn u8_pathbuf(v8: Vec<u8>) -> PathBuf {
  #[cfg(target_family = "unix")]
  {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(OsString::from_vec(v8))
  }
  #[cfg(target_family = "windows")]
  {
    use std::{convert::TryInto, os::windows::ffi::OsStringExt};
    let mut buf = Vec::new();
    for chunk in v8.chunks_exact(2) {
      let c: [u8; 2] = chunk.try_into().expect("exact chunks");
      let b = u16::from_ne_bytes(c);
      buf.push(b)
    }
    PathBuf::from(OsString::from_wide(&buf))
  }
}

fn stream_stdin(abort: &Arc<Abort>, use_nul: bool) -> (JoinHandle<()>, Receiver<Payload>) {
  let (tx, rx) = bounded::<Payload>(1);

  let abort = abort.clone();
  let handle = spawn(async move {
    if atty::is(atty::Stream::Stdin) {
      abort
        .send(Fail::ArgumentError(
          "/dev/stdin connected to tty".to_owned(),
        ))
        .await;
    } else {
      let delim = if use_nul { b'\0' } else { b'\n' };
      let mut reader = BufReader::new(stdin());
      let mut seen = HashSet::new();

      loop {
        let mut buf = Default::default();
        let f1 = abort.notified();
        let f2 = reader.read_until(delim, &mut buf);

        pin_mut!(f1);
        pin_mut!(f2);
        match select(f1, f2).await {
          Either::Left(_) | Either::Right((Ok(0), _)) => break,
          Either::Right((Err(err), _)) => {
            abort
              .send(Fail::IO(PathBuf::from("/dev/stdin"), err.kind()))
              .await;
            break;
          }
          Either::Right((Ok(_), _)) => {
            buf.pop();
            let path = u8_pathbuf(buf);
            match canonicalize(&path).await {
              Ok(canonical) => {
                if seen.insert(canonical.clone())
                  && tx.send(Payload::Entire(canonical)).await.is_err()
                {
                  break;
                }
              }
              Err(err) if err.kind() == ErrorKind::NotFound => (),
              Err(err) => {
                abort.send(Fail::IO(path, err.kind())).await;
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

pub fn stream_in(
  abort: &Arc<Abort>,
  mode: &Mode,
  args: &Arguments,
) -> (JoinHandle<()>, Receiver<Payload>) {
  match mode {
    Mode::Initial => stream_stdin(abort, args.read0),
    Mode::Preview(path) | Mode::Patch(path) => stream_patch(abort, path),
  }
}
