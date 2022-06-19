use {
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
  tokio::{sync::Notify, task::JoinError},
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

pub struct Abort {
  errors: Mutex<Vec<Fail>>,
  rx: Notify,
}

impl Abort {
  pub fn new() -> Arc<Self> {
    Arc::new(Self {
      errors: Mutex::new(Default::default()),
      rx: Notify::new(),
    })
  }

  pub async fn fin(&self) -> Vec<Fail> {
    self.errors.lock().await.to_vec()
  }

  pub async fn send(&self, fail: Fail) {
    let mut errors = self.errors.lock().await;
    errors.push(fail);
    self.rx.notify_waiters();
  }

  pub async fn notified(&self) {
    let errors = self.errors.lock().await;
    if errors.len() > 0 {
    } else {
      self.rx.notified().await;
    }
  }
}
