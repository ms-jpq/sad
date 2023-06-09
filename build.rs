#![deny(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata, clippy::wildcard_dependencies)]

use uuid::Uuid;

fn main() {
  println!(
    "cargo:rustc-env=SAD_ARGV_UUID={uuid}",
    uuid = Uuid::new_v4()
  );

  println!(
    "cargo:rustc-env=SAD_PREVIEW_UUID={uuid}",
    uuid = Uuid::new_v4()
  );

  println!(
    "cargo:rustc-env=SAD_PATCH_UUID={uuid}",
    uuid = Uuid::new_v4()
  );
}
