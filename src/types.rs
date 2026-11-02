use {
  aho_corasick::BuildError,
  regex::Error as RegexError,
  std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::ErrorKind,
    path::PathBuf,
  },
  tokio::task::JoinError,
};

#[derive(Debug)]
pub enum Die {
  ArgumentError(String),
  BadExit(PathBuf, i32),
  BuildError(BuildError),
  IO(PathBuf, ErrorKind),
  Interrupt,
  Join(JoinError),
  RegexError(RegexError),
}

impl Error for Die {}

impl Display for Die {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "Error: {self:?}")
  }
}

impl From<RegexError> for Die {
  fn from(e: RegexError) -> Self {
    Self::RegexError(e)
  }
}

impl From<BuildError> for Die {
  fn from(e: BuildError) -> Self {
    Self::BuildError(e)
  }
}

impl From<JoinError> for Die {
  fn from(e: JoinError) -> Self {
    Self::Join(e)
  }
}
