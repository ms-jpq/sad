use async_std::io;

pub enum Failure {
  IO(String),
  Str(String),
}

pub type SadResult<T> = Result<T, Failure>;

pub trait Sadness {
  fn cry(&self) -> Failure;
}

pub trait Depression {
  type Wry;
  fn halp(self) -> SadResult<Self::Wry>;
}

impl<T, E: Sadness> Depression for Result<T, E> {
  type Wry = T;
  fn halp(self) -> SadResult<Self::Wry> {
    match self {
      Ok(val) => Ok(val),
      Err(err) => Err(err.cry()),
    }
  }
}

impl Sadness for io::Error {
  fn cry(&self) -> Failure {
    Failure::IO(format!("{}", self))
  }
}

impl Sadness for std::string::FromUtf8Error {
  fn cry(&self) -> Failure {
    Failure::Str(format!("{}", self))
  }
}
