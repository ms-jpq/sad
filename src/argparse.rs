use super::subprocess::SubprocessCommand;
use super::types::Fail;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use regex::{Regex, RegexBuilder};
use shlex::split;
use std::{collections::HashMap, env, path::PathBuf};
use structopt::StructOpt;

use tokio::{fs::File, io::AsyncReadExt};
use which::which;

#[derive(Debug)]
pub enum Mode {
  Initial,
  Preview(PathBuf),
  Patch(PathBuf),
}

impl Mode {
  pub const PREVIEW: &'static str = env!("SAD_PREVIEW_UUID");
  pub const PATCH: &'static str = env!("SAD_PATCH_UUID");
}

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

  /// Regex flags: use `--help` instead of `-h` to see details
  ///
  /// [lowercase on, uppercase off] ie i => on, I => off
  ///
  /// i :: ignore case (works for --exact)
  ///
  /// m :: multiline '^', '$'
  ///
  /// s :: '.' match newlines
  ///
  /// u :: swap the meaning of '*' and '*?' (lazy & greedy matching)
  ///
  /// x :: ignore whitespaces and '#' comments
  #[structopt(short, long)]
  pub flags: Option<String>,

  /// Colourizing program, disable = never, default = $GIT_PAGER
  ///
  /// Uses bash shell syntax for splitting
  #[structopt(short, long)]
  pub pager: Option<String>,

  /// Additional Fzf options, disable = never
  ///
  /// Uses bash shell syntax for splitting
  #[structopt(long)]
  pub fzf: Option<String>,

  /// Same as in GNU diff --unified={size}, affects aggregate size
  ///
  /// ie. a higher {size} will leader to more changes grouped together
  #[structopt(short, long)]
  pub unified: Option<usize>,

  /// *Internal use only*
  #[structopt(long)]
  pub internal_preview: Option<PathBuf>,

  /// *Internal use only*
  #[structopt(long)]
  pub internal_patch: Option<PathBuf>,
}

pub async fn parse_args() -> Result<Arguments, Fail> {
  let args = env::args().collect::<Vec<_>>();
  match (
    args.get(1).map(|s| s.as_str()),
    args.get(2).map(PathBuf::from).map(PathBuf::from),
    env::var_os(Mode::PREVIEW).map(PathBuf::from),
    env::var_os(Mode::PATCH).map(PathBuf::from),
  ) {
    (Some("-c"), Some(files), Some(preview), None) => {
      let mut buf = String::new();
      let mut fd = File::open(preview).await.map_err(|_| Fail::ArgV)?;
      fd.read_to_string(&mut buf).await.map_err(|_| Fail::ArgV)?;
      Ok(Arguments::from_iter(buf.split('\0')))
    }
    (Some("-c"), Some(files), None, Some(patch)) => {
      let mut buf = String::new();
      let mut fd = File::open(patch).await.map_err(|_| Fail::ArgV)?;
      fd.read_to_string(&mut buf).await.map_err(|_| Fail::ArgV)?;
      Ok(Arguments::from_iter(buf.split('\0')))
    }
    (Some("-c"), _, _, _) => Err(Fail::ArgumentError(
      "`-c` is a reserved flag, use --k, or --commit".to_owned(),
    )),
    _ => Ok(Arguments::from_args()),
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

fn p_auto_flags(exact: bool, pattern: &str) -> Vec<String> {
  let mut flags = vec!["i".to_owned()];
  if !exact {
    flags.push("m".to_owned())
  }
  for c in pattern.chars() {
    if c.is_uppercase() {
      flags.push("I".to_owned());
      break;
    }
  }
  flags
}

fn p_aho_corasick(pattern: &str, flags: Vec<String>) -> Result<AhoCorasick, Fail> {
  let mut ac = AhoCorasickBuilder::new();
  for flag in flags {
    match flag.as_str() {
      "i" => ac.ascii_case_insensitive(true),
      "I" => ac.ascii_case_insensitive(false),
      _ => {
        return Err(Fail::ArgumentError(format!(
          "Invaild regex flag, see `--help` :: {}",
          flag
        )))
      }
    };
  }
  Ok(ac.build(&[pattern]))
}

fn p_regex(pattern: &str, flags: Vec<String>) -> Result<Regex, Fail> {
  let mut re = RegexBuilder::new(pattern);
  for flag in flags {
    match flag.as_str() {
      "i" => re.case_insensitive(true),
      "I" => re.case_insensitive(false),
      "m" => re.multi_line(true),
      "M" => re.multi_line(false),
      "s" => re.dot_matches_new_line(true),
      "S" => re.dot_matches_new_line(false),
      "u" => re.swap_greed(true),
      "U" => re.swap_greed(false),
      "x" => re.ignore_whitespace(true),
      "X" => re.ignore_whitespace(false),
      _ => {
        return Err(Fail::ArgumentError(format!(
          "Invaild regex flag, see `--help` :: {}",
          flag
        )))
      }
    };
  }
  Ok(re.build()?)
}

fn p_fzf(fzf: Option<String>) -> Option<(PathBuf, Vec<String>)> {
  match (which("fzf"), atty::is(atty::Stream::Stdout)) {
    (Ok(p), true) => match fzf {
      Some(val) if val == "never" => None,
      Some(val) => Some((p, split(&val).unwrap_or_default())),
      None => Some((p, Vec::new())),
    },
    _ => None,
  }
}

fn p_pager(pager: &Option<String>) -> Option<SubprocessCommand> {
  let norm = || which("delta").or_else(|_| which("diff-so-fancy")).ok();
  let (prog, arguments) = match pager {
    Some(val) => match val as &str {
      "never" => (None, Vec::new()),
      _ => {
        let mut sh = split(val)
          .unwrap_or_else(|| vec![val.to_owned()])
          .into_iter();
        (
          sh.next().and_then(|p| which(p).ok()).or_else(norm),
          sh.collect(),
        )
      }
    },
    None => {
      let val = env::var("GIT_PAGER").unwrap_or_default();
      let less_less = val.split('|').next().unwrap_or(&val).trim();
      let mut sh = split(less_less)
        .unwrap_or_else(|| vec![less_less.to_owned()])
        .into_iter();
      (
        sh.next().and_then(|p| which(p).ok()).or_else(norm),
        sh.collect(),
      )
    }
  };

  prog.map(|program| SubprocessCommand {
    args: arguments,
    prog: program,
    env: HashMap::new(),
  })
}

pub fn parse_opts(args: Arguments) -> Result<Options, Fail> {
  let mut flagset = p_auto_flags(args.exact, &args.pattern);
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
      Engine::AhoCorasick(p_aho_corasick(&args.pattern, flagset)?, replace)
    } else {
      Engine::Regex(p_regex(&args.pattern, flagset)?, replace)
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
