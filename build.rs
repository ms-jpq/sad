#![deny(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata, clippy::wildcard_dependencies)]

use std::{
  error::Error,
  process::{Command, Stdio},
};

fn uuid() -> Result<String, Box<dyn Error>> {
  let proc = Command::new("uuidgen").stdout(Stdio::piped()).output()?;
  assert!(proc.status.success());
  let uuid = String::from_utf8(proc.stdout)?.trim().into();
  Ok(uuid)
}

fn main() -> Result<(), Box<dyn Error>> {
  println!("cargo:rustc-env=SAD_ARGV_UUID={uuid}", uuid = uuid()?);

  println!("cargo:rustc-env=SAD_PREVIEW_UUID={uuid}", uuid = uuid()?);

  println!("cargo:rustc-env=SAD_PATCH_UUID={uuid}", uuid = uuid()?);

  Ok(())
}
