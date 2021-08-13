use regex::Error as RegexError;
use std::{
  clone::Clone,
  error::Error,
  fmt::{self, Display, Formatter},
  io::ErrorKind,
  path::PathBuf,
};
use tokio::{
  sync::{Mutex, Notify},
  task::JoinError,
};

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

pub struct Abort {
  errors: Mutex<Vec<Fail>>,
  pub rx: Notify,
}

impl Abort {
  pub fn new() -> Self {
    Abort {
      errors: Mutex::new(Vec::new()),
      rx: Notify::new(),
    }
  }

  pub fn fin(self: Self) -> Vec<Fail> {
    self.errors.into_inner()
  }

  pub async fn send(self: &Self, fail: Fail) {
    let mut errors = self.errors.lock().await;
    errors.push(fail);
    self.rx.notify_waiters()
  }
}
