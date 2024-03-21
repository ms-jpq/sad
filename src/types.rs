use {
  aho_corasick::BuildError,
  futures::lock::Mutex,
  regex::Error as RegexError,
  std::{
    clone::Clone,
    error::Error,
    fmt::{self, Display, Formatter},
    io::ErrorKind,
    path::PathBuf,
    sync::Arc,
  },
  tokio::task::JoinError,
};

#[derive(Clone, Debug)]
pub enum Fail {
  Join,
  Interrupt,
  RegexError(RegexError),
  BuildError(BuildError),
  ArgumentError(String),
  IO(PathBuf, ErrorKind),
  BadExit(PathBuf, i32),
}

impl Error for Fail {}

impl Display for Fail {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "Error:\n{self:#?}")
  }
}

impl From<JoinError> for Fail {
  fn from(e: JoinError) -> Self {
    if e.is_cancelled() {
      Self::Interrupt
    } else {
      Self::Join
    }
  }
}

impl From<RegexError> for Fail {
  fn from(e: RegexError) -> Self {
    Self::RegexError(e)
  }
}

impl From<BuildError> for Fail {
  fn from(e: BuildError) -> Self {
    Self::BuildError(e)
  }
}
