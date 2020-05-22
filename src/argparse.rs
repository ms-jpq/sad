use super::errors::*;
use super::subprocess::SubprocessCommand;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use clap::Clap;
use either::Either::{self, *};
use regex::{Regex, RegexBuilder};
use std::{env, path::PathBuf};

#[derive(Clap)]
#[clap(name = "sad", author, about, version)]
pub struct Arguments {
  #[clap(about = "Search pattern")]
  pub pattern: String,

  #[clap(about = "Replacement pattern")]
  pub replace: Option<String>,

  #[clap(short, long, about = "Skip stdin, supply files to edit")]
  pub input: Vec<PathBuf>,

  #[clap(short = "0", about = r"Use \0 as stdin delimiter")]
  pub nul_delim: bool,

  #[clap(short = "k", long, about = "No preview, write changes to file")]
  pub commit: bool,

  #[clap(short, long, about = "String literal mode")]
  pub exact: bool,

  #[clap(short, long, about = "Standard regex flags: ie. -f imx")]
  pub flags: Option<String>,

  #[clap(long, about = "ignore $GIT_PAGER")]
  pub no_pager: bool,

  #[clap(
    short,
    long,
    about = "Same as in GNU diff --unified={size}, affects hunk size"
  )]
  pub unified: Option<usize>,
}

#[derive(Clone)]
pub enum Action {
  Diff,
  Write,
}

#[derive(Clone)]
pub struct Options {
  pub pattern: Either<AhoCorasick, Regex>,
  pub replace: String,
  pub action: Action,
  pub pager: Option<SubprocessCommand>,
  pub unified: usize,
}

impl Options {
  pub fn new(args: Arguments) -> SadResult<Options> {
    let auto_flags = p_auto_flags(&args.pattern);
    let flags = args
      .flags
      .unwrap_or_default()
      .split_terminator("")
      .skip(1)
      .map(String::from)
      .collect::<Vec<String>>();
    let flagset = itertools::chain(auto_flags, flags).collect::<Vec<String>>();

    let pattern = {
      if args.exact {
        Left(p_aho_corasick(&args.pattern, &flagset)?)
      } else {
        Right(p_regex(&args.pattern, &flagset)?)
      }
    };

    let action = if args.commit {
      Action::Write
    } else {
      Action::Diff
    };

    let pager = if args.no_pager { None } else { p_pager() };

    Ok(Options {
      pattern,
      replace: args.replace.unwrap_or_default(),
      action,
      pager,
      unified: args.unified.unwrap_or(3),
    })
  }
}

fn p_auto_flags(pattern: &str) -> Vec<String> {
  for c in pattern.chars() {
    if c.is_uppercase() {
      return vec!["I".into()];
    }
  }
  vec!["i".into()]
}

fn p_aho_corasick(pattern: &str, flags: &[String]) -> SadResult<AhoCorasick> {
  let mut ac = AhoCorasickBuilder::new();
  for flag in flags {
    match flag.as_str() {
      "I" => ac.ascii_case_insensitive(false),
      "i" => ac.ascii_case_insensitive(true),
      _ => return Err(Failure::Simple("Invalid flags".into())),
    };
  }
  Ok(ac.build(&[pattern]))
}

fn p_regex(pattern: &str, flags: &[String]) -> SadResult<Regex> {
  let mut re = RegexBuilder::new(pattern);
  for flag in flags {
    match flag.as_str() {
      "I" => re.case_insensitive(false),
      "i" => re.case_insensitive(true),
      "m" => re.multi_line(true),
      "s" => re.dot_matches_new_line(true),
      "U" => re.swap_greed(true),
      "x" => re.ignore_whitespace(true),
      _ => return Err(Failure::Simple("Invalid flags".into())),
    };
  }
  re.build().into_sadness()
}

fn p_pager() -> Option<SubprocessCommand> {
  env::var("GIT_PAGER").ok().and_then(|val| {
    let less_less = val.split('|').next().unwrap_or(&val).trim();
    let mut commands = less_less.split(' ').map(String::from);
    commands.next().map(|program| SubprocessCommand {
      program,
      arguments: commands.collect(),
    })
  })
}
