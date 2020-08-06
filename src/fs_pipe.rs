use super::errors::{Failure, SadResult, SadnessFrom};
use std::{fs::Metadata, path::PathBuf};
use tokio::fs::{
  canonicalize, metadata, read_to_string, remove_file, rename, set_permissions, write,
};
use uuid::Uuid;

pub struct Slurpee {
  pub canonical: PathBuf,
  pub meta: Metadata,
  pub content: String,
}

pub async fn slurp(path: &PathBuf) -> SadResult<Slurpee> {
  let canonical = canonicalize(&path).await.into_sadness()?;
  let meta = metadata(&canonical).await.into_sadness()?;
  let content = if meta.is_file() {
    match read_to_string(&canonical).await {
      Ok(text) => text,
      Err(err) => Err(err).into_sadness()?,
    }
  } else {
    String::new()
  };
  let slurpee = Slurpee {
    canonical,
    meta,
    content,
  };
  Ok(slurpee)
}

pub async fn spit(canonical: &PathBuf, meta: &Metadata, text: &str) -> SadResult<()> {
  let uuid = Uuid::new_v4().to_simple().to_string();
  let mut file_name = canonical
    .file_name()
    .and_then(|s| s.to_str())
    .map(String::from)
    .ok_or_else(|| Failure::Simple(format!("Bad file name - {}", canonical.to_string_lossy())))?;
  file_name.push_str("___");
  file_name.push_str(&uuid);

  let backup = canonical.with_file_name(file_name);
  rename(&canonical, &backup).await.into_sadness()?;
  write(&canonical, text).await.into_sadness()?;
  set_permissions(&canonical, meta.permissions())
    .await
    .into_sadness()?;
  remove_file(&backup).await.into_sadness()?;

  Ok(())
}
