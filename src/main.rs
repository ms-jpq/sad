use clap::Clap;
use argparse::{Arguments, Options};
use std::process;
mod argparse;

fn stream_files(args: &Arguments) {}

fn main() {
  let args = Arguments::parse();
  let files = stream_files(&args);
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
