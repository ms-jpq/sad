use options::{Arguments, Options};
use std::process;

fn displace(path: String, text: &str) {}

fn main() {
  let args = Arguments::parse();
  let files = if true { 1 } else { 2 };
  match Options::new(args) {
    Ok(opts) => {
      println!("{:?}", opts);
    }
    Err(e) => {
      eprintln!("{}", e);
      process::exit(1);
    }
  }
}
