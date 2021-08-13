use super::types::Fail;
use std::{fs::Metadata, io::ErrorKind, path::PathBuf};
use tokio::fs::{metadata, read_to_string, remove_file, rename, set_permissions, write};
use uuid::Uuid::new_v4;

pub struct Slurpee {
  pub meta: Metadata,
  pub content: String,
}

pub async fn slurp(path: &PathBuf) -> Result<Slurpee, Fail> {
  let meta = metadata(&path)
    .await
    .map_err(|e| Fail::IO(path.clone(), e.kind()))?;
  let content = if meta.is_file() {
    match read_to_string(&path).await {
      Ok(text) => text,
      Err(err) if err.kind() == ErrorKind::InvalidData => String::new(),
      Err(err) => Err(err),
    }
  } else {
    String::new()
  };
  let slurpee = Slurpee {
    meta,
    content,
  };
  Ok(slurpee)
}

pub async fn spit(canonical: &PathBuf, meta: &Metadata, text: &str) -> Result<(), Fail> {
  let uuid = new_v4().to_simple().to_string();
  let mut file_name = canonical
    .file_name()
    .and_then(|s| s.to_str())
    .map(String::from)
    .ok_or_else(|| Fail::Simple(format!("Bad file name - {}", canonical.display())))?;
  file_name.push_str("___");
  file_name.push_str(&uuid);

  let backup = canonical.with_file_name(file_name);
  rename(&canonical, &backup).await?;
  write(&canonical, text).await?;
  set_permissions(&canonical, meta.permissions()).await?;
  remove_file(&backup).await?;

  Ok(())
}
