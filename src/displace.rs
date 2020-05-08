use super::argparse::{Action, Options};
use super::errors::*;
use super::udiff;
use async_std::{fs, path::PathBuf};
use either::Either::*;

async fn replace(path: &PathBuf, opts: &Options) -> SadResult<(String, String)> {
  let before = fs::read_to_string(path).await.halp()?;
  let after = match &opts.pattern {
    Left(ac) => ac.replace_all(&before, &[opts.replace.as_str()]),
    Right(re) => String::from(re.replace_all(&before, opts.replace.as_str())),
  };
  Ok((before, after))
}

pub async fn displace(path: PathBuf, opts: &Options) -> SadResult<String> {
  let (before, after) = replace(&path, opts).await?;
  let name = String::from(path.to_string_lossy());
  let print = match opts.action {
    Action::Diff => udiff::udiff(&name, &before, &after),
    Action::Write => {
      fs::write(&path, after).await.halp()?;
      name
    }
  };
  Ok(print)
}
