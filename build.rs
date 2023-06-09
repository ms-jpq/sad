#![deny(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata, clippy::wildcard_dependencies)]

use std::{
  error::Error,
  process::{Command, Stdio},
};

fn uuid() -> Result<String, Box<dyn Error>> {
  let out = Command::new("uuidgen").stdout(Stdio::piped()).output()?;
  assert!(out.status.success());
  let uuid = String::from_utf8(out.stdout)?.trim().into();
  Ok(uuid)
}

fn main() -> Result<(), Box<dyn Error>> {
  println!("cargo:rustc-env=SAD_ARGV_UUID={uuid}", uuid = uuid()?);

  println!("cargo:rustc-env=SAD_PREVIEW_UUID={uuid}", uuid = uuid()?);

  println!("cargo:rustc-env=SAD_PATCH_UUID={uuid}", uuid = uuid()?);
  Ok(())
}
