use async_channel::{RecvError, SendError};
use std::{
  env::VarError,
  error::Error,
  fmt::{self, Display, Formatter},
  io, num, string,
};
use tokio::task::JoinError;

/*
 * Consolidate Error Handling
 * ==========================
 */

#[derive(Debug)]
pub enum Failure {
  Channel,
  Compound(Box<Failure>, Box<Failure>),
  Displace(String, Box<Failure>),
  Fzf(String),
  Interrupt,
  IO(io::Error),
  JoinError,
  NilStdin,
  Parse(String),
  Regex(regex::Error),
  Simple(String),
  Str(string::FromUtf8Error),
  VarErr,
}

impl Failure {
  pub fn exit_message(&self) -> Option<String> {
    match self {
      Failure::Interrupt => None,
      _ => Some(format!("{}", self)),
    }
  }

  pub fn exit_code(&self) -> i32 {
    match self {
      Failure::Interrupt => 130,
      _ => 1,
    }
  }
}

impl Display for Failure {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "Error:\n{:#?}", self)
  }
}

impl Error for Failure {}

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

impl<T> From<SendError<T>> for Failure {
  fn from(_: SendError<T>) -> Self {
    Failure::Channel
  }
}

impl From<RecvError> for Failure {
  fn from(_: RecvError) -> Self {
    Failure::Channel
  }
}

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

impl From<VarError> for Failure {
  fn from(_: VarError) -> Self {
    Failure::VarErr
  }
}

impl From<JoinError> for Failure {
  fn from(_: JoinError) -> Self {
    Failure::JoinError
  }
}
