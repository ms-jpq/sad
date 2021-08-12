use std::{
  error::Error,
  fmt::{self, Display, Formatter},
};

/*
 * Consolidate Error Handling
 * ==========================
 */

#[derive(Debug)]
pub enum Failure {
  Interrupt,
  Sucks(String),
}
impl Error for Failure {}

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
