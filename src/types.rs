use regex::Error as RegexError;
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
  Join,
  Interrupt,
  ArgumentError(String),
  RegexError(RegexError),
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
