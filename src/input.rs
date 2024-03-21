use {
  super::{
    argparse::{Arguments, Mode},
    types::{Abort, Fail},
    udiff::DiffRange,
  },
  async_channel::{bounded, Receiver},
  futures::{
    future::{ready, select, Either},
    pin_mut,
    stream::{empty, once, try_unfold, Stream, StreamExt, TryStream, TryStreamExt},
  },
  regex::Regex,
  std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    io::{self, ErrorKind, IsTerminal},
    marker::Unpin,
    path::{Path, PathBuf},
    pin::{pin, Pin},
    task::{Context, Poll},
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
  let f = Fail::ArgumentError(String::default());
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

async fn stream_patch(patch: &Path) -> Box<dyn Stream<Item = Result<Payload, Fail>>> {
  let patch = patch.to_owned();

  let fd = match File::open(&patch).await {
    Err(e) => {
      let err = Fail::IO(patch.to_owned(), e.kind());
      return Box::new(once(ready(Err(err))));
    }
    Ok(fd) => fd,
  };
  //let mut reader = BufReader::new(fd);
  //let mut acc = HashMap::<_, HashSet<_>>::new();

  //loop {
  //  let mut buf = Vec::default();
  //  let n = reader
  //    .read_until(b'\0', &mut buf)
  //    .await
  //    .map_err(|e| Fail::IO(path_file.to_owned(), e.kind()))?;

  //  if n == 0 {
  //    break;
  //  }

  //  buf.pop();
  //  let line =
  //    String::from_utf8(buf).map_err(|_| Fail::IO(path_file.to_owned(), ErrorKind::InvalidData))?;
  //  let patch = p_line(&line)?;
  //  if let Some(ranges) = acc.get_mut(&patch.0) {
  //    ranges.insert(patch.1);
  //  } else {
  //    let mut ranges = HashSet::new();
  //    ranges.insert(patch.1);
  //    acc.insert(patch.0, ranges);
  //  }
  //}

  let stream = try_unfold(0, |_| async { Ok(None) });
  return Box::new(stream);
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

fn stream_stdin(use_nul: bool) -> impl Stream<Item = Result<Payload, Fail>> {
  let delim = if use_nul { b'\0' } else { b'\n' };
  let reader = BufReader::new(stdin());
  let seen = HashSet::new();

  let stream = try_unfold((reader, seen), move |mut s| async move {
    let mut buf = Vec::default();
    match s.0.read_until(delim, &mut buf).await {
      Err(err) => Err(Fail::IO(PathBuf::from("/dev/stdin"), err.kind())),
      Ok(0) => Ok(Some((None, s))),
      Ok(_) => {
        buf.pop();
        let path = u8_pathbuf(buf);
        match canonicalize(&path).await {
          Err(err) if err.kind() == ErrorKind::NotFound => Ok(Some((None, s))),
          Err(err) => Err(Fail::IO(path, err.kind())),
          Ok(canonical) => Ok(Some({
            if s.1.insert(canonical.clone()) {
              (Some(Payload::Entire(canonical)), s)
            } else {
              (None, s)
            }
          })),
        }
      }
    }
  });

  return stream.try_filter_map(|x| async { Ok(x) });
}

pub async fn stream_in(
  mode: &Mode,
  args: &Arguments,
) -> Box<dyn Stream<Item = Result<Payload, Fail>>> {
  match mode {
    Mode::Initial if io::stdin().is_terminal() => {
      let err = Fail::ArgumentError("/dev/stdin connected to tty".to_owned());
      Box::new(once(ready(Err(err))))
    }
    Mode::Initial => Box::new(stream_stdin(args.read0)),
    Mode::Preview(path) | Mode::Patch(path) => stream_patch(path).await,
  }
}
