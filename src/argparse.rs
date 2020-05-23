use super::errors::*;
use super::subprocess::SubprocessCommand;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use regex::{Regex, RegexBuilder};
use std::{env, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "sad", author, about)]
pub struct Arguments {
  #[structopt(about = "Search pattern")]
  pub pattern: String,

  #[structopt(about = "Replacement pattern")]
  pub replace: Option<String>,

  #[structopt(short, long, about = "Skip stdin, supply files to edit")]
  pub input: Vec<PathBuf>,

  #[structopt(short = "0", about = r"Use \0 as stdin delimiter")]
  pub nul_delim: bool,

  #[structopt(short = "k", long, about = "No preview, write changes to file")]
  pub commit: bool,

  #[structopt(short, long, about = "String literal mode")]
  pub exact: bool,

  #[structopt(short, long, about = "Standard regex flags: ie. -f imx")]
  pub flags: Option<String>,

  #[structopt(
    long,
    env = "GIT_PAGER",
    about = "Colourizing program, disable = never, default = $GIT_PAGER"
  )]
  pub pager: Option<String>,

  #[structopt(
    short,
    long,
    about = "Same as in GNU diff --unified={size}, affects hunk size"
  )]
  pub unified: Option<usize>,

  #[structopt(short, long, about = "Pick hunks using fzf")]
  pub pick: bool,

  #[structopt(long, about = "*Internal use only*")]
  pub internal_preview: Option<String>,

  #[structopt(long, about = "*Internal use only*")]
  pub internal_patch: Option<String>,
}

pub enum Engine {
  AhoCorasick(AhoCorasick, String),
  Regex(Regex, String),
}

pub enum Action {
  Preview,
  Commit,
  Pick,
}

#[derive(Clone)]
pub enum Printer {
  Stdout,
  Pager(SubprocessCommand),
  Fzf,
}

pub struct Options {
  pub action: Action,
  pub engine: Engine,
  pub printer: Printer,
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

    let engine = {
      let replace = args.replace.unwrap_or_default();
      if args.exact {
        Engine::AhoCorasick(p_aho_corasick(&args.pattern, &flagset)?, replace)
      } else {
        Engine::Regex(p_regex(&args.pattern, &flagset)?, replace)
      }
    };

    let action = match (args.pick, args.commit) {
      (true, _) => Action::Pick,
      (false, true) => Action::Commit,
      (false, false) => Action::Preview,
    };

    let printer = if args.pick {
      Printer::Fzf
    } else {
      match p_pager(args.pager) {
        Some(cmd) => Printer::Pager(cmd),
        None => Printer::Stdout,
      }
    };

    Ok(Options {
      action,
      engine,
      printer,
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

fn p_pager(pager: Option<String>) -> Option<SubprocessCommand> {
  pager.and_then(|val| {
    let less_less = val.split('|').next().unwrap_or(&val).trim();
    let mut commands = less_less.split(' ').map(String::from);
    commands.next().map(|program| SubprocessCommand {
      program,
      arguments: commands.collect(),
    })
  })
}
