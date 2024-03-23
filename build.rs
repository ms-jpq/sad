#!/usr/bin/env -S -- bash -Eeuo pipefail
// || rustc --edition=2021 -o "${T:="$(mktemp)"}" -- "$0" && exec -a "$0" -- "$T" "$0" "$@"
#![deny(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata, clippy::wildcard_dependencies)]

use std::{
  error::Error,
  process::{Command, Stdio},
};

fn uuid() -> Result<String, Box<dyn Error>> {
  #[cfg(target_family = "windows")]
  let py = "python.exe";
  #[cfg(target_family = "unix")]
  let py = "python3";
  let proc = Command::new(py)
    .arg("-c")
    .arg("import uuid; print(uuid.uuid4())")
    .stdout(Stdio::piped())
    .output()?;
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
