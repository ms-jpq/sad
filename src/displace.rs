use super::{
  argparse::{Action, Engine, Options},
  fs_pipe::{slurp, spit},
  input::Payload,
  types::Fail,
  udiff::{apply_patches, patches, pure_diffs, udiff},
};
use ansi_term::Colour;
use pathdiff::diff_paths;
use std::{path::PathBuf, sync::Arc};
use tokio::task::spawn_blocking;

impl Engine {
  fn replace(&self, before: &str) -> String {
    match self {
      Engine::AhoCorasick(ac, replace) => ac.replace_all(before, &[replace.as_str()]),
      Engine::Regex(re, replace) => re.replace_all(before, replace.as_str()).into(),
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

pub async fn displace(opts: &Arc<Options>, payload: Payload) -> Result<String, Fail> {
  let path = payload.path().clone();
  let rel_path = opts
    .cwd
    .as_ref()
    .and_then(|cwd| diff_paths(&path, cwd))
    .unwrap_or_else(|| path.clone());

  let name = format!("{}", rel_path.display());
  let slurped = slurp(&path).await?;
  let before = Arc::new(slurped.content);

  let o = opts.clone();
  let o2 = opts.clone();
  let b = before.clone();
  let after = spawn_blocking(move || o.engine.replace(&b)).await?;

  if *before == after {
    Ok(String::new())
  } else {
    let print = match (&opts.action, payload) {
      (Action::Preview, Payload::Entire(_)) => {
        spawn_blocking(move || udiff(None, o2.unified, &name, &before, &after)).await?
      }
      (Action::Preview, Payload::Piecewise(_, ranges)) => {
        spawn_blocking(move || udiff(Some(&ranges), o2.unified, &name, &before, &after)).await?
      }
      (Action::Commit, Payload::Entire(_)) => {
        spit(&path, &slurped.meta, &after).await?;
        format!("{name}\n")
      }
      (Action::Commit, Payload::Piecewise(_, ranges)) => {
        let after = spawn_blocking(move || {
          let patches = patches(o2.unified, &before, &after);
          apply_patches(patches, &ranges, &before)
        })
        .await?;

        spit(&path, &slurped.meta, &after).await?;
        format!("{name}\n")
      }
      (Action::FzfPreview(_, _), _) => {
        spawn_blocking(move || {
          let ranges = pure_diffs(o2.unified, &before, &after);
          let mut fzf_lines = String::new();
          for range in ranges {
            let repr = Colour::Red.paint(format!("{range}"));
            let line = format!("{name}\n\n\n\n{repr}\0");
            fzf_lines.push_str(&line);
          }
          fzf_lines
        })
        .await?
      }
    };
    Ok(print)
  }
}
