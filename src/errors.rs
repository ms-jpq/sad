use std::{io, string};
use tokio::sync::mpsc::errors::SendError;

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
  SendError(SendError<()>),
}

pub type SadResult<T> = Result<T, Failure>;

impl<T, E> From<Result<T, E>> for SadResult<T>
where
  E: Into<Failure>,
{
  fn from(result: Result<T, E>) -> Self {
    match result {
      Ok(val) => Ok(val),
      Err(err) => Err(err.into()),
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
    Failure::Simple(String::from("TODO"))
  }
}

