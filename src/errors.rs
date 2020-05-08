use async_std::io;
use std::{fmt, string};

/*
 * Consolidate Error Handling
 */

pub enum Failure {
  IO(String),
  Str(String),
  Regex(String),
}

impl fmt::Display for Failure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let message = match self {
      Failure::IO(txt) => txt,
      Failure::Str(txt) => txt,
      Failure::Regex(txt) => txt,
    };
    write!(f, "{}", message)
  }
}

pub type SadResult<T> = Result<T, Failure>;

pub trait Sadness {
  fn cry(&self) -> Failure;
}

pub trait Depression {
  type Wry;
  fn halp(self) -> SadResult<Self::Wry>;
}

impl<T, E: Sadness> Depression for Result<T, E> {
  type Wry = T;
  fn halp(self) -> SadResult<Self::Wry> {
    match self {
      Ok(val) => Ok(val),
      Err(err) => Err(err.cry()),
    }
  }
}

/*
 * Consolidate Error Handling
 */

impl Sadness for io::Error {
  fn cry(&self) -> Failure {
    Failure::IO(format!("{}", self))
  }
}

impl Sadness for string::FromUtf8Error {
  fn cry(&self) -> Failure {
    Failure::Str(format!("{}", self))
  }
}

impl Sadness for regex::Error {
  fn cry(&self) -> Failure {
    Failure::Regex(format!("{}", self))
  }
}
