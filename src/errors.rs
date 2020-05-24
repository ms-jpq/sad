use std::{io, num, string};
use tokio::task::JoinError;

/*
 * Consolidate Error Handling
 * ==========================
 */

#[derive(Debug)]
pub enum Failure {
  Compound(Box<Failure>, Box<Failure>),
  Displace(String, Box<Failure>),
  Fzf(String),
  Interrupt,
  IO(io::Error),
  JoinError,
  Pager(String),
  Parse(String),
  Regex(regex::Error),
  Simple(String),
  Str(string::FromUtf8Error),
}

impl Failure {
  pub fn silent_exit(&self) -> bool {
    match self {
      Failure::Interrupt => true,
      _ => false,
    }
  }
}

pub type SadResult<T> = Result<T, Failure>;

pub trait SadnessFrom<T> {
  fn into_sadness(self) -> SadResult<T>;
}

impl<T, E: Into<Failure>> SadnessFrom<T> for Result<T, E> {
  fn into_sadness(self) -> SadResult<T> {
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

impl From<num::ParseIntError> for Failure {
  fn from(err: num::ParseIntError) -> Self {
    Failure::Parse(format!("{:#?}", err))
  }
}

impl From<regex::Error> for Failure {
  fn from(err: regex::Error) -> Self {
    Failure::Regex(err)
  }
}

impl From<JoinError> for Failure {
  fn from(_: JoinError) -> Self {
    Failure::JoinError
  }
}
