use super::argparse::{Action, Engine, Options};
use super::errors::*;
use super::input::Payload;
use super::udiff::{udiff, DiffRanges, Diffs, Patchable, Picker};
use ansi_term::Colour;
use std::{fs::Metadata, path::PathBuf};
use tokio::fs;
use uuid::Uuid;

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

async fn read_meta(path: &PathBuf) -> SadResult<(PathBuf, Metadata)> {
  let canonical = fs::canonicalize(&path).await.into_sadness()?;
  let meta = fs::metadata(&canonical).await.into_sadness()?;
  if !meta.is_file() {
    let msg = format!("Not a file - {}", canonical.to_string_lossy());
    return Err(Failure::Simple(msg));
  }
  Ok((canonical, meta))
}

async fn safe_write(canonical: &PathBuf, meta: &Metadata, text: &str) -> SadResult<()> {
  let uuid = Uuid::new_v4().to_simple().to_string();
  let mut file_name = canonical
    .file_name()
    .and_then(|s| s.to_str())
    .map(String::from)
    .ok_or_else(|| Failure::Simple(format!("Bad file name - {}", canonical.to_string_lossy())))?;
  file_name.push_str("___");
  file_name.push_str(&uuid);

  let backup = canonical.with_file_name(file_name);
  fs::rename(&canonical, &backup).await.into_sadness()?;
  fs::write(&canonical, text).await.into_sadness()?;
  fs::set_permissions(&canonical, meta.permissions())
    .await
    .into_sadness()?;
  fs::remove_file(&backup).await.into_sadness()?;

  Ok(())
}

async fn displace_impl(opts: &Options, payload: &Payload) -> SadResult<String> {
  let path = payload.path().clone();
  let name = path.to_string_lossy();
  let (canonical, meta) = read_meta(&path).await?;
  let before = fs::read_to_string(&canonical).await.into_sadness()?;
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
        safe_write(&canonical, &meta, &after).await?;
        format!("{}\n", name)
      }
      (Action::Commit, Payload::Piecewise(_, ranges)) => {
        let diffs: Diffs = Patchable::new(opts.unified, &before, &after);
        let after = diffs.patch(&ranges, &before);
        safe_write(&canonical, &meta, &after).await?;
        format!("{}\n", name)
      }
      (Action::Fzf, _) => {
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

pub async fn displace(opts: &Options, payload: Payload) -> SadResult<String> {
  match displace_impl(opts, &payload).await {
    Ok(ret) => Ok(ret),
    Err(err) => Err(Failure::Displace(format!("{:#?}", payload), Box::new(err))),
  }
}

