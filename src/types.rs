use regex::RegexError;
use std::{
  clone::Clone,
  error::Error,
  fmt::{self, Display, Formatter},
  io::ErrorKind,
  path::PathBuf,
};
use tokio::sync::broadcast::Sender;

#[derive(Clone, Debug)]
pub enum Fail {
  Interrupt,
  ArgumentError(String),
  RegexError(RegexError),
  IO(PathBuf, ErrorKind),
}

impl Fail {
  pub fn exit_message(&self) -> Option<String> {
    match self {
      Fail::Interrupt => None,

      _ => Some(format!("{}", self)),
    }
  }

  pub fn exit_code(&self) -> i32 {
    match self {
      Fail::Interrupt => 130,
      _ => 1,
    }
  }
}

impl Error for Fail {}

impl Display for Fail {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "Error:\n{:#?}", self)
  }
}

pub type Abort = Sender<Fail>;
