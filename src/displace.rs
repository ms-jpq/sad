use super::argparse::{Action, Engine, Options};
use super::fs_pipe::{slurp, spit};
use super::input::Payload;
use super::types::Fail;
use super::udiff::{udiff, DiffRanges, Diffs, Patchable, Picker};
use ansi_term::Colour;
use pathdiff::diff_paths;
use std::{error::Error, path::PathBuf};

impl Engine {
  fn replace(&self, before: &str) -> String {
    match self {
      Engine::AhoCorasick(ac, replace) => ac.replace_all(&before, &[replace.as_str()]),
      Engine::Regex(re, replace) => re.replace_all(&before, replace.as_str()).into(),
    }
  }
}

impl Payload {
  fn path(&self) -> &PathBuf {
    match self {
      Payload::Entire(path) => path,
      Payload::Piecewise(path, _) => path,
    }
  }
}

async fn displace_impl(opts: &Options, payload: &Payload) -> Result<String, Fail> {
  let path = payload.path().clone();
  let slurped = slurp(&path).await?;
  let rel_path = opts
    .cwd
    .as_ref()
    .and_then(|cwd| diff_paths(&path, cwd))
    .map(|p| p)
    .unwrap_or(path);

  let name = rel_path.display();
  let (canonical, meta, before) = (slurped.path, slurped.meta, slurped.content);
  let after = opts.engine.replace(&before);

  if before == after {
    Ok(String::new())
  } else {
    let print = match (&opts.action, &payload) {
      (Action::Preview, Payload::Entire(_)) => udiff(None, opts.unified, &name, &before, &after),
      (Action::Preview, Payload::Piecewise(_, ranges)) => {
        udiff(Some(ranges), opts.unified, &name, &before, &after)
      }
      (Action::Commit, Payload::Entire(_)) => {
        spit(&canonical, &meta, &after).await?;
        format!("{}\n", name)
      }
      (Action::Commit, Payload::Piecewise(_, ranges)) => {
        let diffs: Diffs = Patchable::new(opts.unified, &before, &after);
        let after = diffs.patch(&ranges, &before);
        spit(&canonical, &meta, &after).await?;
        format!("{}\n", name)
      }
      (Action::Fzf(_, _), _) => {
        let ranges: DiffRanges = Picker::new(opts.unified, &before, &after);
        let mut fzf_lines = String::new();
        for range in ranges {
          let repr = Colour::Red.paint(format!("{}", range));
          let line = format!("{}\n\n\n\n{}\0", &name, repr);
          fzf_lines.push_str(&line);
        }
        fzf_lines
      }
    };
    Ok(print)
  }
}

pub async fn displace(opts: &Options, payload: Payload) -> Result<String, Fail> {
  match displace_impl(opts, &payload).await {
    Ok(ret) => Ok(ret),
    Err(err) => Err(Fail::Sucks(format!("{:#?}{:#?}", payload, err))),
  }
}
