use super::errors::{Failure, SadResult, SadnessFrom};
use super::subprocess::SubprocessCommand;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use regex::{Regex, RegexBuilder};
use std::{collections::HashMap, env, path::PathBuf};
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
  Fzf(PathBuf, Vec<String>),
}

#[derive(Clone, Debug)]
pub enum Printer {
  Stdout,
  Pager(SubprocessCommand),
}

#[derive(Clone, Debug)]
pub struct Options {
  pub cwd: Option<PathBuf>,
  pub action: Action,
  pub engine: Engine,
  pub printer: Printer,
  pub unified: usize,
}

impl Options {
  pub fn new(args: Arguments) -> SadResult<Options> {
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

    let action = if args.commit || args.internal_patch != None {
      Action::Commit
    } else {
      match (args.internal_preview, p_fzf(args.fzf)) {
        (Some(_), _) => Action::Preview,
        (_, None) => Action::Preview,
        (_, Some((bin, args))) => Action::Fzf(bin, args),
      }
    };

    let printer = match p_pager(&args.pager) {
      Some(cmd) => Printer::Pager(cmd),
      None => Printer::Stdout,
    };

    Ok(Options {
      cwd: env::current_dir().ok(),
      action,
      engine,
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

fn p_fzf(fzf: Option<String>) -> Option<(PathBuf, Vec<String>)> {
  match (which("fzf"), atty::is(atty::Stream::Stdout)) {
    (Ok(p), true) => match fzf {
      Some(v) if v == "never" => None,
      Some(val) => Some((p, val.split_whitespace().map(String::from).collect())),
      None => Some((p, Vec::new())),
    },
    _ => None,
  }
}

fn p_pager(pager: &Option<String>) -> Option<SubprocessCommand> {
  let (prog, arguments) = match pager {
    Some(val) => match val as &str {
      "never" => (None, Vec::new()),
      _ => (Some(PathBuf::from(val)), Vec::new()),
    },
    None => match env::var("GIT_PAGER") {
      Ok(val) => {
        let less_less = val.split('|').next().unwrap_or(&val).trim();
        let mut commands = less_less.split_whitespace().map(String::from);
        (commands.next().map(PathBuf::from), commands.collect())
      }
      Err(_) => (
        which("delta").or_else(|_| which("diff-so-fancy")).ok(),
        Vec::new(),
      ),
    },
  };

  prog.map(|program| SubprocessCommand {
    arguments,
    program,
    env: HashMap::new(),
  })
}
