use std::{
  error::Error,
  fmt::{self, Display, Formatter},
};
use tokio::sync::broadcast::Sender;

#[derive(Debug)]
pub enum Failure {
  Interrupt,
  Sucks(String),
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

impl Error for Failure {}

impl Display for Failure {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "Error:\n{:#?}", self)
  }
}

pub type Abort = Sender<Box<dyn Error>>;
