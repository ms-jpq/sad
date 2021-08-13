use regex::Error as RegexError;
use std::{
  clone::Clone,
  error::Error,
  fmt::{self, Display, Formatter},
  io::{Error as IOError, ErrorKind},
  path::PathBuf,
};
use tokio::{sync::broadcast::Sender, task::JoinError};

#[derive(Clone, Debug)]
pub enum Fail {
  Join,
  Interrupt,
  RegexError(RegexError),
  ArgumentError(String),
  IO(PathBuf, ErrorKind),
  BadExit(PathBuf, i32),
}

impl Error for Fail {}

impl Display for Fail {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "Error:\n{:#?}", self)
  }
}

pub type Abort = Sender<Fail>;

impl From<JoinError> for Fail {
  fn from(e: JoinError) -> Self {
    if e.is_cancelled() {
      Fail::Interrupt
    } else {
      Fail::Join
    }
  }
}

impl From<RegexError> for Fail {
  fn from(e: RegexError) -> Self {
    Fail::RegexError(e)
  }
}

impl Fail {
  fn from_io(path: PathBuf, err: IOError) -> Self {
    Fail::IO(path, err.kind())
  }
}
