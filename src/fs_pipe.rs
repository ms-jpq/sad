use super::types::Fail;
use std::{ffi::OsString, fs::Metadata, io::ErrorKind, path::PathBuf};
use tokio::{
  fs::{rename, File, OpenOptions},
  io::{AsyncReadExt, AsyncWriteExt},
};
use uuid::Uuid;

pub struct Slurpee {
  pub meta: Metadata,
  pub content: String,
}

pub async fn slurp(path: &PathBuf) -> Result<Slurpee, Fail> {
  let mut fd = File::open(path)
    .await
    .map_err(|e| Fail::IO(path.clone(), e.kind()))?;

  let meta = fd
    .metadata()
    .await
    .map_err(|e| Fail::IO(path.clone(), e.kind()))?;

  let content = if meta.is_file() {
    let mut s = String::new();
    match fd.read_to_string(&mut s).await {
      Ok(_) => s,
      Err(err) if err.kind() == ErrorKind::InvalidData => s,
      Err(err) => Err(Fail::IO(path.clone(), err.kind()))?,
    }
  } else {
    String::new()
  };

  Ok(Slurpee { meta, content })
}

pub async fn spit(canonical: &PathBuf, meta: &Metadata, text: &str) -> Result<(), Fail> {
  let uuid = Uuid::new_v4().to_simple().to_string();
  let mut file_name = canonical
    .file_name()
    .map(|n| n.to_owned())
    .unwrap_or_else(|| OsString::from(""));
  file_name.push("___");
  file_name.push(uuid);
  let tmp = canonical.with_file_name(file_name);

  let mut fd = OpenOptions::new()
    .create_new(true)
    .write(true)
    .open(&tmp)
    .await
    .map_err(|e| Fail::IO(tmp.clone(), e.kind()))?;

  fd.set_permissions(meta.permissions())
    .await
    .map_err(|e| Fail::IO(tmp.clone(), e.kind()))?;
  fd.write_all(text.as_bytes());

  rename(&tmp, &canonical)
    .await
    .map_err(|e| Fail::IO(canonical.clone(), e.kind()))?;

  Ok(())
}
