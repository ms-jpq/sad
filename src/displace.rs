use {
  super::{
    argparse::{Action, Engine, Options},
    fs_pipe::{slurp, spit},
    input::LineIn,
    types::Die,
    udiff::{apply_patches, patches, pure_diffs, udiff},
  },
  ansi_term::Colour,
  std::{ffi::OsString, path::PathBuf, sync::Arc},
};

impl Engine {
  fn replace(&self, before: &str) -> String {
    match self {
      Self::AhoCorasick(ac, replace) => ac.replace_all(before, &[replace.as_str()]),
      Self::Regex(re, replace) => re.replace_all(before, replace.as_str()).into(),
    }
  }
}

impl LineIn {
  const fn path(&self) -> &PathBuf {
    match self {
      Self::Entire(path) | Self::Piecewise(path, _) => path,
    }
  }
}

pub async fn displace(opts: &Arc<Options>, input: LineIn) -> Result<OsString, Die> {
  let path = input.path().clone();
  let name = opts
    .cwd
    .as_ref()
    .and_then(|cwd| path.strip_prefix(cwd).ok())
    .unwrap_or_else(|| path.as_ref())
    .as_os_str()
    .to_owned();

  let slurped = slurp(&path).await?;
  let before = slurped.content;
  let after = opts.engine.replace(&before);

  if *before == after {
    Ok(OsString::default())
  } else {
    let print = match (&opts.action, input) {
      (Action::Preview, LineIn::Entire(_)) => udiff(None, opts.unified, &name, &before, &after),
      (Action::Preview, LineIn::Piecewise(_, ranges)) => {
        udiff(Some(&ranges), opts.unified, &name, &before, &after)
      }
      (Action::Commit, LineIn::Entire(_)) => {
        spit(&path, &slurped.meta, &after).await?;
        let mut out = name;
        out.push("\n");
        out
      }
      (Action::Commit, LineIn::Piecewise(_, ranges)) => {
        let patches = patches(opts.unified, &before, &after);
        let after = apply_patches(patches, &ranges, &before);
        spit(&path, &slurped.meta, &after).await?;
        let mut out = name;
        out.push("\n");
        out
      }
      (Action::FzfPreview(_, _), _) => {
        let ranges = pure_diffs(opts.unified, &before, &after);
        let mut fzf_lines = OsString::new();
        for range in ranges {
          let repr = Colour::Red.paint(format!("{range}"));
          fzf_lines.push(&name);
          let line = format!("\n\n\n\n{repr}\0");
          fzf_lines.push(&line);
        }
        fzf_lines
      }
    };
    Ok(print)
  }
}
