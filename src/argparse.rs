use super::errors::*;
use super::subprocess::SubprocessCommand;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use regex::{Regex, RegexBuilder};
use std::{cmp::max, collections::HashMap, env, fs, path::PathBuf};
use structopt::StructOpt;
use which::which;

#[derive(Debug, StructOpt)]
#[structopt(name = "sad", author, about)]
pub struct Arguments {
  /// Search pattern
  #[structopt()]
  pub pattern: String,

  /// Replacement pattern, empty = delete
  #[structopt()]
  pub replace: Option<String>,

  /// Use \0 as stdin delimiter
  #[structopt(short = "0", long = "read0")]
  pub nul_delim: bool,

  /// No preview, write changes to file
  #[structopt(short = "k", long)]
  pub commit: bool,

  /// String literal mode
  #[structopt(short, long)]
  pub exact: bool,

  /// Standard regex flags: ie. -f imx, full list: https://github.com/ms-jpq/sad
  #[structopt(short, long)]
  pub flags: Option<String>,

  /// Colourizing program, disable = never, default = $GIT_PAGER
  #[structopt(short, long)]
  pub pager: Option<String>,

  /// Additional Fzf options, disable = never
  #[structopt(long)]
  pub fzf: Option<String>,

  /// Same as in GNU diff --unified={size}, affects hunk size
  #[structopt(short, long)]
  pub unified: Option<usize>,

  /// *Internal use only*
  #[structopt(long)]
  pub internal_preview: Option<PathBuf>,

  /// *Internal use only*
  #[structopt(long)]
  pub internal_patch: Option<PathBuf>,
}

impl Arguments {
  pub fn new() -> SadResult<Arguments> {
    let args = env::args().collect::<Vec<_>>();
    match (args.get(1), args.get(2)) {
      (Some(lhs), Some(rhs)) if lhs == "-c" => {
        if rhs.contains('\x04') {
          Ok(Arguments::from_iter(rhs.split('\x04')))
        } else {
          Err(Failure::Simple(
            "`-c` is a reserved flag, use --k, or --commit".to_owned(),
          ))
        }
      }
      _ => Ok(Arguments::from_args()),
    }
  }
}

#[derive(Clone, Debug)]
pub enum Engine {
  AhoCorasick(AhoCorasick, String),
  Regex(Regex, String),
}

#[derive(Clone, Debug)]
pub enum Action {
  Preview,
  Commit,
  Fzf,
}

#[derive(Clone, Debug)]
pub enum Printer {
  Stdout,
  Pager(SubprocessCommand),
}

#[derive(Clone, Debug)]
pub struct Options {
  pub name: String,
  pub action: Action,
  pub engine: Engine,
  pub fzf: Option<Vec<String>>,
  pub printer: Printer,
  pub unified: usize,
}

impl Options {
  pub fn new(args: Arguments) -> SadResult<Options> {
    let name = env::args()
      .next()
      .and_then(|s| {
        let path = PathBuf::from(s);
        fs::canonicalize(path).ok()
      })
      .or_else(|| which::which("sad").ok())
      .and_then(|p| p.to_str().map(|p| p.to_owned()))
      .unwrap_or_else(|| "sad".to_owned());

    let mut flagset = p_auto_flags(&args.pattern);
    flagset.extend(
      args
        .flags
        .unwrap_or_default()
        .split_terminator("")
        .skip(1)
        .map(String::from),
    );

    let engine = {
      let replace = args.replace.unwrap_or_default();
      if args.exact {
        Engine::AhoCorasick(p_aho_corasick(&args.pattern, &flagset)?, replace)
      } else {
        Engine::Regex(p_regex(&args.pattern, &flagset)?, replace)
      }
    };

    let fzf = p_fzf(args.fzf);

    let action = if args.commit || args.internal_patch != None {
      Action::Commit
    } else if args.internal_preview != None || fzf == None {
      Action::Preview
    } else {
      Action::Fzf
    };

    let printer = match p_pager(args.pager) {
      Some(cmd) => Printer::Pager(cmd),
      None => Printer::Stdout,
    };

    Ok(Options {
      name,
      action,
      engine,
      fzf,
      printer,
      unified: args.unified.unwrap_or(3),
    })
  }
}

fn p_auto_flags(pattern: &str) -> Vec<String> {
  let mut flags = vec!["m".into(), "i".into()];
  for c in pattern.chars() {
    if c.is_uppercase() {
      flags.push("I".into());
      break;
    }
  }
  flags
}

fn p_aho_corasick(pattern: &str, flags: &[String]) -> SadResult<AhoCorasick> {
  let mut ac = AhoCorasickBuilder::new();
  for flag in flags {
    match flag.as_str() {
      "I" => ac.ascii_case_insensitive(false),
      "i" => ac.ascii_case_insensitive(true),
      "m" => &mut ac,
      _ => return Err(Failure::Simple(format!("Invaild regex flag -{}", flag))),
    };
  }
  Ok(ac.build(&[pattern]))
}

fn p_regex(pattern: &str, flags: &[String]) -> SadResult<Regex> {
  let mut re = RegexBuilder::new(pattern);
  for flag in flags {
    match flag.as_str() {
      "i" => re.case_insensitive(true),
      "I" => re.case_insensitive(false),
      "m" => re.multi_line(true),
      "M" => re.multi_line(false),
      "s" => re.dot_matches_new_line(true),
      "U" => re.swap_greed(true),
      "x" => re.ignore_whitespace(true),
      _ => return Err(Failure::Simple(format!("Invaild regex flag -{}", flag))),
    };
  }
  re.build().into_sadness()
}

fn p_fzf(fzf: Option<String>) -> Option<Vec<String>> {
  match (which("fzf"), atty::is(atty::Stream::Stdout)) {
    (Ok(_), true) => match fzf {
      Some(v) if v == "never" => None,
      Some(val) => Some(val.split_whitespace().map(String::from).collect()),
      None => Some(Vec::new()),
    },
    _ => None,
  }
}

fn p_pager_args(program: &str, commands: Vec<String>) -> Vec<String> {
  let mut cmd = commands;
  if let Ok(width) = env::var("FZF_PREVIEW_COLUMNS")
    .into_sadness()
    .and_then(|w| w.parse::<isize>().into_sadness())
  {
    if program == "delta" {
      cmd.push(format!("--width={}", max(0, width - 1)))
    }
  }
  cmd
}

fn find_exec(exe: &str) -> Option<String> {
  which(exe)
    .ok()
    .and_then(|p| p.to_str().map(|p| p.to_owned()))
}

fn p_pager(pager: Option<String>) -> Option<SubprocessCommand> {
  pager
    .or_else(|| env::var("GIT_PAGER").ok())
    .or_else(|| find_exec("delta"))
    .or_else(|| find_exec("diff-so-fancy"))
    .and_then(|val| {
      if val == "never" {
        None
      } else {
        let less_less = val.split('|').next().unwrap_or(&val).trim();
        let mut commands = less_less.split_whitespace().map(String::from);
        commands.next().map(|program| SubprocessCommand {
          arguments: p_pager_args(&program, commands.collect()),
          program,
          env: HashMap::new(),
        })
      }
    })
}
