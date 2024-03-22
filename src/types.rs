use {
  aho_corasick::BuildError,
  regex::Error as RegexError,
  std::{
    clone::Clone,
    error::Error,
    fmt::{self, Display, Formatter},
    io::ErrorKind,
    path::PathBuf,
  },
};

#[derive(Clone, Debug)]
pub enum Die {
  Eof,
  Interrupt,
  RegexError(RegexError),
  BuildError(BuildError),
  ArgumentError(String),
  IO(PathBuf, ErrorKind),
  BadExit(PathBuf, i32),
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
