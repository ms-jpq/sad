use async_std::io;
use std::{fmt, string};

/*
 * Consolidate Error Handling
 */

pub enum Failure {
  Simple(String),
  IO(io::Error),
  Str(string::FromUtf8Error),
  Regex(regex::Error),
}

impl fmt::Display for Failure {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self)
  }
}

pub type SadResult<T> = Result<T, Failure>;

pub trait Sadness {
  fn cry(self) -> Failure;
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
  fn cry(self) -> Failure {
    Failure::IO(self)
  }
}

impl Sadness for string::FromUtf8Error {
  fn cry(self) -> Failure {
    Failure::Str(self)
  }
}

impl Sadness for regex::Error {
  fn cry(self) -> Failure {
    Failure::Regex(self)
  }
}
