use {
  super::{
    argparse::{Action, Engine, Options},
    fs_pipe::{slurp, spit},
    input::RowIn,
    types::Die,
    udiff::{apply_patches, patches, pure_diffs, udiff},
  },
  ansi_term::Style,
  std::{borrow::ToOwned, ffi::OsString, path::PathBuf, sync::Arc},
  tokio::task::spawn_blocking,
};

impl Engine {
  fn replace(&self, before: &str) -> String {
    match self {
      Self::AhoCorasick(ac, replace) => ac.replace_all(before, &[replace.as_str()]),
      Self::Regex(re, replace) => re.replace_all(before, replace.as_str()).into(),
    }
  }
}

impl RowIn {
  const fn path(&self) -> &PathBuf {
    match self {
      Self::Entire(path) | Self::Piecewise(path, _) => path,
    }
  }
}

fn diff(opts: &Options, before: &[String]) -> Vec<String> {
  let b = before.iter().cloned().collect::<String>();
  opts
    .engine
    .replace(&b)
    .split_inclusive('\n')
    .map(ToOwned::to_owned)
    .collect::<Vec<_>>()
}

pub async fn displace(opts: Arc<Options>, input: RowIn) -> Result<OsString, Die> {
  let path = input.path().clone();
  let slurped = slurp(&path).await?;
  let before = Arc::new(slurped.content);
  if before.is_empty() {
    Ok(OsString::new())
  } else {
    let mut name = opts
      .cwd
      .as_ref()
      .and_then(|cwd| path.strip_prefix(cwd).ok())
      .unwrap_or_else(|| path.as_ref())
      .as_os_str()
      .to_owned();

    let (b, o) = (before.clone(), opts.clone());
    let after = spawn_blocking(move || diff(&o, &b)).await?;

    let print = match (&opts.action, input) {
      (Action::Preview, RowIn::Entire(_)) => {
        spawn_blocking(move || udiff(None, opts.unified, &name, &before, &after)).await?
      }
      (Action::Preview, RowIn::Piecewise(_, ranges)) => {
        spawn_blocking(move || udiff(Some(&ranges), opts.unified, &name, &before, &after)).await?
      }
      (Action::Commit, RowIn::Entire(_)) => {
        spit(&path, &slurped.meta, after).await?;
        name.push("\n");
        name
      }
      (Action::Commit, RowIn::Piecewise(_, ranges)) => {
        let applied = spawn_blocking(move || {
          let patches = patches(opts.unified, &before, &after);
          let applied = apply_patches(patches, &ranges, &before);
          applied.iter().map(ToString::to_string).collect::<Vec<_>>()
        })
        .await?;
        spit(&path, &slurped.meta, applied).await?;
        name.push("\n");
        name
      }
      (Action::FzfPreview(_, _), _) => {
        let ranges = spawn_blocking(move || pure_diffs(opts.unified, &before, &after)).await?;
        if ranges.is_empty() {
          OsString::new()
        } else {
          let mut fzf_lines = OsString::new();
          for range in ranges {
            let repr = Style::new().dimmed().paint(format!("{range}"));
            fzf_lines.push(&name);
            let line = format!("\x04 {repr}\0");
            fzf_lines.push(&line);
          }
          fzf_lines
        }
      }
    };
    Ok(print)
  }
}
