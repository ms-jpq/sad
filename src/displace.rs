use super::argparse::{Action, Options};
use super::errors::*;
use async_std::{fs, path::PathBuf};
use difference::{Changeset, Difference};
use either::Either::*;

async fn replace(path: &PathBuf, opts: &Options) -> SadResult<(String, String)> {
  let before = fs::read_to_string(path).await.halp()?;
  let after = match &opts.pattern {
    Left(ac) => ac.replace_all(&before, &[opts.replace.as_str()]),
    Right(re) => String::from(re.replace_all(&before, opts.replace.as_str())),
  };
  Ok((before, after))
}

fn diff(before: &str, after: &str) -> String {
  let changes = Changeset::new(before, after, "\n");
  let mut diff = String::new();
  for line in changes.diffs {
    match line {
      Difference::Add(l) => &diff.push_str(format!("+{}", l).as_str()),
      Difference::Rem(l) => &diff.push_str(format!("-{}", l).as_str()),
      Difference::Same(l) => &diff.push_str(format!("{}", l).as_str()),
    };
  }
  diff
}

pub async fn displace(path: PathBuf, opts: &Options) -> SadResult<String> {
  let (before, after) = replace(&path, opts).await?;
  let print = match opts.action {
    Action::Diff => diff(&before, &after),
    Action::Write => {
      fs::write(&path, after).await.halp()?;
      String::from(path.to_string_lossy())
    }
  };
  Ok(print)
}
