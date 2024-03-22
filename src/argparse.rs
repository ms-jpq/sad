use {
  super::{subprocess::SubprocCommand, types::Die},
  aho_corasick::{AhoCorasick, AhoCorasickBuilder},
  clap::Parser,
  regex::{Regex, RegexBuilder},
  shlex::split,
  std::{
    collections::HashMap,
    env::{args_os, current_dir, var_os},
    ffi::OsString,
    io::{stderr, stdout, IsTerminal},
    path::PathBuf,
  },
  which::which,
};

#[derive(Debug)]
pub enum Mode {
  Initial,
  Preview(PathBuf),
  Patch(PathBuf),
}

impl Mode {
  pub const ARGV: &'static str = env!("SAD_ARGV_UUID");
  pub const PREVIEW: &'static str = env!("SAD_PREVIEW_UUID");
  pub const PATCH: &'static str = env!("SAD_PATCH_UUID");
}

#[derive(Debug, Parser)]
#[clap(about, version)]
pub struct Arguments {
  /// Search pattern
  #[clap()]
  pub pattern: String,

  /// Replacement pattern, empty = delete
  #[clap()]
  pub replace: Option<String>,

  /// Use \0 as stdin delimiter
  #[clap(short = '0', long)]
  pub read0: bool,

  /// No preview, write changes to file
  #[clap(short = 'k', long)]
  pub commit: bool,

  /// String literal mode
  #[clap(short, long)]
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
  #[clap(short, long)]
  pub flags: Option<String>,

  /// Colourizing program, disable = never, default = $GIT_PAGER
  ///
  /// Uses bash shell syntax for splitting
  #[clap(short, long)]
  pub pager: Option<String>,

  /// Additional Fzf options, disable = never
  ///
  /// Uses bash shell syntax for splitting
  #[clap(long)]
  pub fzf: Option<String>,

  /// Same as in GNU diff --unified={size}, affects aggregate size
  ///
  /// ie. a higher {size} will leader to more changes grouped together
  #[clap(short, long)]
  pub unified: Option<usize>,
}

fn parse_fzf_mode(argv: &OsString) -> Option<Mode> {
  let exec = argv
    .to_str()
    .unwrap_or_default()
    .split('\x04')
    .collect::<Vec<_>>();
  match exec[..] {
    [Mode::PREVIEW, path] => Some(Mode::Preview(PathBuf::from(path))),
    [Mode::PATCH, path] => Some(Mode::Patch(PathBuf::from(path))),
    _ => None,
  }
}

pub fn parse_args() -> (Mode, Arguments) {
  let args = args_os().collect::<Vec<_>>();
  match (
    args.get(1).and_then(|a| a.to_str()),
    args.get(2).and_then(parse_fzf_mode),
    var_os(Mode::ARGV).and_then(|a| a.into_string().ok()),
  ) {
    (Some("-c"), Some(mode), Some(arg_list)) => {
      (mode, Arguments::parse_from(arg_list.split('\x04')))
    }
    _ => (Mode::Initial, Arguments::parse_from(args)),
  }
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Engine {
  AhoCorasick(AhoCorasick, String),
  Regex(Regex, String),
}

#[derive(Clone, Debug)]
pub enum Action {
  Preview,
  Commit,
  FzfPreview(PathBuf, Vec<String>),
}

#[derive(Clone, Debug)]
pub enum Printer {
  Stdout,
  Pager(SubprocCommand),
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
    flags.push("m".to_owned());
  }
  for c in pattern.chars() {
    if c.is_uppercase() {
      flags.push("I".to_owned());
      break;
    }
  }
  flags
}

fn p_aho_corasick(pattern: &str, flags: Vec<String>) -> Result<AhoCorasick, Die> {
  let mut ac = AhoCorasickBuilder::new();
  for flag in flags {
    match flag.as_str() {
      "i" => ac.ascii_case_insensitive(true),
      "I" => ac.ascii_case_insensitive(false),
      _ => {
        return Err(Die::ArgumentError(format!(
          "Invalid regex flag, see `--help` :: {flag}"
        )))
      }
    };
  }
  Ok(ac.build([pattern])?)
}

fn p_regex(pattern: &str, flags: Vec<String>) -> Result<Regex, Die> {
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
        return Err(Die::ArgumentError(format!(
          "Invalid regex flag, see `--help` :: {flag}"
        )))
      }
    };
  }
  Ok(re.build()?)
}

fn p_fzf(fzf: &Option<String>) -> Option<(PathBuf, Vec<String>)> {
  match (which("fzf"), stdout().is_terminal(), stderr().is_terminal()) {
    (Ok(p), true, true) => match fzf.as_deref() {
      Some("never") => None,
      Some(val) => Some((p, split(val).unwrap_or_default())),
      None => Some((p, Vec::default())),
    },
    _ => None,
  }
}

fn p_pager(pager: &Option<String>) -> Option<SubprocCommand> {
  let norm = || which("delta").or_else(|_| which("diff-so-fancy")).ok();

  let (prog, arguments) = match pager.as_deref() {
    Some("never") => (None, Vec::default()),
    Some(val) => {
      let mut sh = split(val)
        .unwrap_or_else(|| vec![val.to_owned()])
        .into_iter();
      (
        sh.next().and_then(|p| which(p).ok()).or_else(norm),
        sh.collect(),
      )
    }
    None => {
      let val = var_os("GIT_PAGER")
        .and_then(|v| v.into_string().ok())
        .unwrap_or_default();

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

  prog.map(|program| SubprocCommand {
    args: arguments,
    prog: program,
    env: HashMap::new(),
  })
}

pub fn parse_opts(mode: Mode, args: Arguments) -> Result<Options, Die> {
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

  let action = match (args.commit, mode, p_fzf(&args.fzf)) {
    (true, _, _) | (_, Mode::Patch(_), _) => Action::Commit,
    (_, Mode::Initial, Some((bin, args))) => Action::FzfPreview(bin, args),
    _ => Action::Preview,
  };

  let printer = p_pager(&args.pager).map_or(Printer::Stdout, Printer::Pager);

  Ok(Options {
    cwd: current_dir().ok(),
    action,
    engine,
    printer,
    unified: args.unified.unwrap_or(3),
  })
}
