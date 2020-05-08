use super::errors::*;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use async_std::path::PathBuf;
use clap::Clap;
use either::Either::{self, *};
use regex::{Regex, RegexBuilder};

#[derive(Clap)]
pub struct Arguments {
  pub pattern: String,
  pub replace: String,

  #[clap(short, long)]
  pub input: Vec<PathBuf>,

  #[clap(short = "0")]
  pub nul_delim: bool,

  #[clap(short, long)]
  pub commit: bool,

  #[clap(short, long)]
  pub exact: bool,

  #[clap(short, long)]
  pub flags: Option<String>,
}

pub enum Action {
  Diff,
  Write,
}

pub struct Options {
  pub pattern: Either<AhoCorasick, Regex>,
  pub replace: String,
  pub commit: Action,
}

impl Options {
  pub fn new(args: Arguments) -> SadResult<Options> {
    let flagset = args
      .flags
      .unwrap_or_default()
      .split_terminator("")
      .skip(1)
      .map(String::from)
      .collect::<Vec<String>>();

    let pattern = {
      if args.exact {
        Left(p_aho_corasick(&args.pattern, &flagset)?)
      } else {
        Right(p_regex(&args.pattern, &flagset)?)
      }
    };

    let commit = if args.commit {
      Action::Write
    } else {
      Action::Diff
    };

    Ok(Options {
      pattern,
      replace: args.replace,
      commit,
    })
  }
}

fn p_aho_corasick(pattern: &str, flags: &[String]) -> SadResult<AhoCorasick> {
  let mut ac = AhoCorasickBuilder::new();
  for flag in flags {
    match flag.as_str() {
      "i" => ac.ascii_case_insensitive(true),
      _ => return Err(Failure::Regex(String::from("Invalid flags"))),
    };
  }
  Ok(ac.build(&[pattern]))
}

fn p_regex(pattern: &str, flags: &[String]) -> SadResult<Regex> {
  let mut re = RegexBuilder::new(pattern);
  for flag in flags {
    match flag.as_str() {
      "i" => re.case_insensitive(true),
      "m" => re.multi_line(true),
      "s" => re.dot_matches_new_line(true),
      "U" => re.swap_greed(true),
      "x" => re.ignore_whitespace(true),
      _ => return Err(Failure::Regex(String::from("Invalid flags"))),
    };
  }
  re.build().halp()
}
