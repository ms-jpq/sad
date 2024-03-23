use {
  super::types::Die,
  std::{borrow::ToOwned, fs::Metadata, path::Path},
  tokio::{
    fs::{rename, File, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
  },
  uuid::Uuid,
};

pub struct Slurpee {
  pub meta: Metadata,
  pub content: Vec<String>,
}

pub async fn slurp(path: &Path) -> Result<Slurpee, Die> {
  let fd = File::open(path)
    .await
    .map_err(|e| Die::IO(path.to_owned(), e.kind()))?;

  let meta = fd
    .metadata()
    .await
    .map_err(|e| Die::IO(path.to_owned(), e.kind()))?;

  let mut content = Vec::new();
  if meta.is_file() {
    let mut reader = BufReader::new(fd);
    loop {
      let mut buf = Vec::new();
      match reader.read_until(b'\n', &mut buf).await {
        Err(err) => return Err(Die::IO(path.to_owned(), err.kind())),
        Ok(0) => break,
        Ok(_) => match String::from_utf8(buf) {
          Ok(s) => content.push(s),
          Err(_) => {
            return Ok(Slurpee {
              meta,
              content: Vec::new(),
            })
          }
        },
      }
    }
  };

  Ok(Slurpee { meta, content })
}

pub async fn spit(
  canonical: &Path,
  meta: &Metadata,
  text: Vec<impl AsRef<[u8]> + Send>,
) -> Result<(), Die> {
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
    .map_err(|e| Die::IO(tmp.clone(), e.kind()))?;
  fd.set_permissions(meta.permissions())
    .await
    .map_err(|e| Die::IO(tmp.clone(), e.kind()))?;

  let mut writer = BufWriter::new(fd);
  for t in text {
    writer
      .write_all(t.as_ref())
      .await
      .map_err(|e| Die::IO(tmp.clone(), e.kind()))?;
  }

  writer
    .flush()
    .await
    .map_err(|e| Die::IO(tmp.clone(), e.kind()))?;

  rename(&tmp, &canonical)
    .await
    .map_err(|e| Die::IO(canonical.to_owned(), e.kind()))?;

  Ok(())
}
