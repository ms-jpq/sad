use {
  super::{
    argparse::{Arguments, Mode},
    types::Die,
    udiff::DiffRange,
  },
  futures::{
    future::{ready, Either},
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
pub enum RowIn {
  Entire(PathBuf),
  Piecewise(PathBuf, HashSet<DiffRange>),
}

#[derive(Debug)]
struct DiffRow(PathBuf, DiffRange);

fn p_row(row: &str) -> Result<DiffRow, Die> {
  let f = || Die::ArgumentError(String::new());
  let ff = |_| f();
  let preg = "\n\n\n\n@@ -(\\d+),(\\d+) \\+(\\d+),(\\d+) @@$";
  let re = Regex::new(preg).map_err(Die::RegexError)?;
  let captures = re.captures(row).ok_or_else(f)?;

  let before_start = captures
    .get(1)
    .ok_or_else(f)?
    .as_str()
    .parse::<usize>()
    .map_err(ff)?;
  let before_inc = captures
    .get(2)
    .ok_or_else(f)?
    .as_str()
    .parse::<usize>()
    .map_err(ff)?;
  let after_start = captures
    .get(3)
    .ok_or_else(f)?
    .as_str()
    .parse::<usize>()
    .map_err(ff)?;
  let after_inc = captures
    .get(4)
    .ok_or_else(f)?
    .as_str()
    .parse::<usize>()
    .map_err(ff)?;

  let range = DiffRange {
    before: (before_start - 1, before_inc),
    after: (after_start - 1, after_inc),
  };
  let path = PathBuf::from(String::from(re.replace(row, "")));
  Ok(DiffRow(path, range))
}

async fn stream_patch(patches: &Path) -> impl Stream<Item = Result<RowIn, Die>> {
  let patches = patches.to_owned();

  let fd = match File::open(&patches).await {
    Err(e) => {
      let err = Die::IO(patches.clone(), e.kind());
      return Either::Left(once(ready(Err(err))));
    }
    Ok(fd) => fd,
  };
  let reader = BufReader::new(fd).split(b'\0');
  let acc = HashSet::new();

  let stream = try_unfold(
    (reader, patches, PathBuf::new(), acc),
    move |mut s| async move {
      let next = s
        .0
        .next_segment()
        .await
        .map_err(|e| Die::IO(s.1.clone(), e.kind()))?;

      match next {
        None if s.3.is_empty() => Ok(None),
        None => {
          let path = s.2;
          let ranges = s.3;
          s.2 = PathBuf::new();
          s.3 = HashSet::new();
          Ok(Some((Some(RowIn::Piecewise(path, ranges)), s)))
        }
        Some(buf) => {
          let row =
            String::from_utf8(buf).map_err(|_| Die::IO(s.1.clone(), ErrorKind::InvalidData))?;
          let parsed = p_row(&row)?;
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
              Ok(Some((Some(RowIn::Piecewise(path, ranges)), s)))
            }
          }
        }
      }
    },
  );

  Either::Right(stream.try_filter_map(|x| ready(Ok(x))))
}

fn u8_pathbuf(v8: Vec<u8>) -> PathBuf {
  #[cfg(target_family = "unix")]
  {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(OsString::from_vec(v8))
  }
  #[cfg(target_family = "windows")]
  {
    use std::os::windows::ffi::OsStringExt;
    let mut buf = Vec::new();
    for chunk in v8.chunks_exact(2) {
      let c: [u8; 2] = chunk.try_into().expect("exact chunks");
      let b = u16::from_ne_bytes(c);
      buf.push(b)
    }
    PathBuf::from(OsString::from_wide(&buf))
  }
}

fn stream_stdin(use_nul: bool) -> impl Stream<Item = Result<RowIn, Die>> {
  if io::stdin().is_terminal() {
    let err = Die::ArgumentError("/dev/stdin connected to tty".to_owned());
    return Either::Left(once(ready(Err(err))));
  }
  let delim = if use_nul { b'\0' } else { b'\n' };
  let reader = BufReader::new(stdin()).split(delim);
  let seen = HashSet::new();

  let stream = try_unfold((reader, seen), |mut s| async {
    let next = s
      .0
      .next_segment()
      .await
      .map_err(|e| Die::IO(PathBuf::from("/dev/stdin"), e.kind()))?;
    match next {
      None => Ok(None),
      Some(buf) => {
        let path = u8_pathbuf(buf);
        match canonicalize(&path).await {
          Err(e) if e.kind() == ErrorKind::NotFound => Ok(Some((None, s))),
          Err(e) => Err(Die::IO(path, e.kind())),
          Ok(canonical) => Ok(Some({
            if s.1.insert(canonical.clone()) {
              (Some(RowIn::Entire(canonical)), s)
            } else {
              (None, s)
            }
          })),
        }
      }
    }
  });

  Either::Right(stream.try_filter_map(|x| ready(Ok(x))))
}

pub async fn stream_in(mode: &Mode, args: &Arguments) -> impl Stream<Item = Result<RowIn, Die>> {
  match mode {
    Mode::Initial => Either::Left(stream_stdin(args.read0)),
    Mode::Preview(path) | Mode::Patch(path) => Either::Right(stream_patch(path).await),
  }
}
