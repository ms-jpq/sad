use std::{io, string};
use tokio::sync::mpsc::error::SendError;

/*
 * Consolidate Error Handling
 * ==========================
 */

#[derive(Debug)]
pub enum Failure {
  Simple(String),
  IO(io::Error),
  Str(string::FromUtf8Error),
  Regex(regex::Error),
  SendError,
}

pub type SadResult<T> = Result<T, Failure>;

pub trait SadnessFrom<T> {
  fn halp(self) -> SadResult<T>;
}

impl<T, E: Into<Failure>> SadnessFrom<T> for Result<T, E> {
  fn halp(self) -> SadResult<T> {
    match self {
      Ok(val) => Ok(val),
      Err(e) => Err(e.into()),
    }
  }
}

/* ==========================
 * Consolidate Error Handling
 */

impl From<io::Error> for Failure {
  fn from(err: io::Error) -> Self {
    Failure::IO(err)
  }
}

impl From<string::FromUtf8Error> for Failure {
  fn from(err: string::FromUtf8Error) -> Self {
    Failure::Str(err)
  }
}

impl From<regex::Error> for Failure {
  fn from(err: regex::Error) -> Self {
    Failure::Regex(err)
  }
}

impl<T> From<SendError<T>> for Failure {
  fn from(err: SendError<T>) -> Self {
    Failure::SendError
  }
}

