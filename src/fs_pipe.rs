use {
  super::types::Fail,
  std::{borrow::ToOwned, fs::Metadata, io::ErrorKind, path::Path},
  tokio::{
    fs::{rename, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
  },
  uuid::Uuid,
};

pub struct Slurpee {
  pub meta: Metadata,
  pub content: String,
}

pub async fn slurp(path: &Path) -> Result<Slurpee, Fail> {
  let mut fd = File::open(path)
    .await
    .map_err(|e| Fail::IO(path.to_owned(), e.kind()))?;

  let meta = fd
    .metadata()
    .await
    .map_err(|e| Fail::IO(path.to_owned(), e.kind()))?;

  let content = if meta.is_file() {
    let mut s = String::new();
    match fd.read_to_string(&mut s).await {
      Ok(_) => s,
      Err(err) if err.kind() == ErrorKind::InvalidData => s,
      Err(err) => return Err(Fail::IO(path.to_owned(), err.kind())),
    }
  } else {
    String::new()
  };

  Ok(Slurpee { meta, content })
}

pub async fn spit(canonical: &Path, meta: &Metadata, text: &str) -> Result<(), Fail> {
  let uuid = Uuid::new_v4().as_simple().to_string();
  let mut file_name = canonical
    .file_name()
    .map(ToOwned::to_owned)
    .unwrap_or_default();
  file_name.push("___");
  file_name.push(uuid);
  let tmp = canonical.with_file_name(file_name);

  let fd = OpenOptions::new()
    .create_new(true)
    .write(true)
    .open(&tmp)
    .await
    .map_err(|e| Fail::IO(tmp.clone(), e.kind()))?;
  fd.set_permissions(meta.permissions())
    .await
    .map_err(|e| Fail::IO(tmp.clone(), e.kind()))?;

  let mut writer = BufWriter::new(fd);
  writer
    .write_all(text.as_bytes())
    .await
    .map_err(|e| Fail::IO(tmp.clone(), e.kind()))?;

  writer
    .flush()
    .await
    .map_err(|e| Fail::IO(tmp.clone(), e.kind()))?;

  rename(&tmp, &canonical)
    .await
    .map_err(|e| Fail::IO(canonical.to_owned(), e.kind()))?;

  Ok(())
}
