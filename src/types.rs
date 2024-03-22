use {
  aho_corasick::BuildError,
  //futures::stream::Stream,
  regex::Error as RegexError,
  std::{
    clone::Clone,
    error::Error,
    fmt::{self, Display, Formatter},
    io::ErrorKind,
    path::PathBuf,
    //pin::{pin, Pin},
    //task::{Context, Poll},
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

//pub enum E3<A, B, C> {
//  A(A),
//  B(B),
//  C(C),
//}

//impl<A, B, C> Stream for E3<A, B, C>
//where
//  A: Stream + Unpin,
//  B: Stream<Item = A::Item> + Unpin,
//  C: Stream<Item = A::Item> + Unpin,
//{
//  type Item = A::Item;

//  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//    match *self {
//      E3::A(ref mut a) => {
//        let a = pin!(a);
//        a.poll_next(cx)
//      }
//      E3::B(ref mut b) => {
//        let b = pin!(b);
//        b.poll_next(cx)
//      }
//      E3::C(ref mut c) => {
//        let c = pin!(c);
//        c.poll_next(cx)
//      }
//    }
//  }
//}
