use super::argparse::{Action, Options};
use super::errors::*;
use async_std::{fs, path::PathBuf};
use diff;
use either::Either::*;

async fn replace(path: &PathBuf, opts: &Options) -> SadResult<(String, String)> {
  let before = fs::read_to_string(path).await.halp()?;
  let after = match &opts.pattern {
    Left(ac) => ac.replace_all(&before, &[opts.replace.as_str()]),
    Right(re) => String::from(re.replace_all(&before, opts.replace.as_str())),
  };
  Ok((before, after))
}

fn rdiff(before: &str, after: &str) -> String {
  let changes = diff::lines(before, after);
  let mut diff = String::new();
  for line in changes {
    match line {
      diff::Result::Left(l) => {
        diff.push_str(format!("-{}\n", l).as_str());
      }
      diff::Result::Right(r) => {
        diff.push_str(format!("+{}\n", r).as_str());
      }
      diff::Result::Both(l, _) => {
        // diff.push_str(format!(" {}\n", l).as_str());
      }
    };
  }
  diff
}

pub async fn displace(path: PathBuf, opts: &Options) -> SadResult<String> {
  let (before, after) = replace(&path, opts).await?;
  let print = match opts.action {
    Action::Diff => rdiff(&before, &after),
    Action::Write => {
      fs::write(&path, after).await.halp()?;
      String::from(path.to_string_lossy())
    }
  };
  Ok(print)
}
