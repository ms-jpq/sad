use super::argparse::{Action, Options};
use super::errors::*;
use super::udiff::{udiff, Diffs, Patchable};
use either::Either::*;
use std::{fs::Metadata, path::PathBuf};
use tokio::fs;
use uuid::Uuid;

fn replace(before: &str, opts: &Options) -> String {
  match &opts.pattern {
    Left(ac) => ac.replace_all(&before, &[opts.replace.as_str()]),
    Right(re) => re.replace_all(&before, opts.replace.as_str()).into(),
  }
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

pub async fn displace(path: PathBuf, opts: &Options) -> SadResult<String> {
  let name = path.to_string_lossy();
  let canonical = fs::canonicalize(&path).await.into_sadness()?;
  let meta = fs::metadata(&canonical).await.into_sadness()?;
  if !meta.is_file() {
    let msg = format!("Not a file - {}", canonical.to_string_lossy());
    return Err(Failure::Simple(msg));
  }
  let before = fs::read_to_string(&canonical).await.into_sadness()?;
  let after = replace(&before, opts);
  if before == after {
    Ok(String::new())
  } else {
    let print = match opts.action {
      Action::Diff => udiff(opts.unified, &name, &before, &after),
      Action::Write => {
        safe_write(&canonical, &meta, &after).await?;
        format!("{}\n", name)
      }
    };
    Ok(print)
  }
}
