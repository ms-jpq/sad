use {
  super::{
    argparse::{Arguments, Mode},
    types::Fail,
    udiff::DiffRange,
  },
  futures::{
    future::ready,
    stream::{once, try_unfold, Stream, TryStreamExt},
  },
  regex::Regex,
  std::{
    collections::HashSet,
    ffi::OsString,
    io::{self, ErrorKind, IsTerminal},
    path::{Path, PathBuf},
  },
  tokio::{
    fs::{canonicalize, File},
    io::{stdin, AsyncBufReadExt, BufReader},
  },
};

#[derive(Debug)]
pub enum LineIn {
  Entire(PathBuf),
  Piecewise(PathBuf, HashSet<DiffRange>),
}

#[derive(Debug)]
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

async fn stream_patch(patches: &Path) -> Box<dyn Stream<Item = Result<LineIn, Fail>> + Send> {
  let patches = patches.to_owned();

  let fd = match File::open(&patches).await {
    Err(e) => {
      let err = Fail::IO(patches.clone(), e.kind());
      return Box::new(once(ready(Err(err))));
    }
    Ok(fd) => fd,
  };
  let reader = BufReader::new(fd);
  let acc = HashSet::new();

  let stream = try_unfold(
    (reader, patches, PathBuf::new(), acc),
    move |mut s| async move {
      let mut buf = Vec::default();
      match s.0.read_until(b'\0', &mut buf).await {
        Err(err) => Err(Fail::IO(s.1.clone(), err.kind())),
        Ok(0) if s.3.is_empty() => Ok(None),
        Ok(0) => {
          let path = s.2;
          let ranges = s.3;
          s.2 = PathBuf::new();
          s.3 = HashSet::new();
          Ok(Some((Some(LineIn::Piecewise(path, ranges)), s)))
        }
        Ok(_) => {
          buf.pop();
          let line =
            String::from_utf8(buf).map_err(|_| Fail::IO(s.1.clone(), ErrorKind::InvalidData))?;
          let parsed = p_line(&line)?;
          if parsed.0 == s.2 {
            s.3.insert(parsed.1);
            Ok(Some((None, s)))
          } else {
            let path = s.2;
            let ranges = s.3;
            s.2 = parsed.0;
            s.3 = HashSet::new();
            s.3.insert(parsed.1);
            if ranges.is_empty() {
              Ok(Some((None, s)))
            } else {
              Ok(Some((Some(LineIn::Piecewise(path, ranges)), s)))
            }
          }
        }
      }
    },
  );

  Box::new(stream.try_filter_map(|x| ready(Ok(x))))
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

fn stream_stdin(use_nul: bool) -> impl Stream<Item = Result<LineIn, Fail>> {
  let delim = if use_nul { b'\0' } else { b'\n' };
  let reader = BufReader::new(stdin());
  let seen = HashSet::new();

  let stream = try_unfold((reader, seen), move |mut s| async move {
    let mut buf = Vec::default();
    match s.0.read_until(delim, &mut buf).await {
      Err(e) => Err(Fail::IO(PathBuf::from("/dev/stdin"), e.kind())),
      Ok(0) => Ok(None),
      Ok(_) => {
        buf.pop();
        let path = u8_pathbuf(buf);
        match canonicalize(&path).await {
          Err(e) if e.kind() == ErrorKind::NotFound => Ok(Some((None, s))),
          Err(e) => Err(Fail::IO(path, e.kind())),
          Ok(canonical) => Ok(Some({
            if s.1.insert(canonical.clone()) {
              (Some(LineIn::Entire(canonical)), s)
            } else {
              (None, s)
            }
          })),
        }
      }
    }
  });

  stream.try_filter_map(|x| ready(Ok(x)))
}

pub async fn stream_in(
  mode: &Mode,
  args: &Arguments,
) -> Box<dyn Stream<Item = Result<LineIn, Fail>> + Send> {
  match mode {
    Mode::Initial if io::stdin().is_terminal() => {
      let err = Fail::ArgumentError("/dev/stdin connected to tty".to_owned());
      Box::new(once(ready(Err(err))))
    }
    Mode::Initial => Box::new(stream_stdin(args.read0)),
    Mode::Preview(path) | Mode::Patch(path) => stream_patch(path).await,
  }
}
