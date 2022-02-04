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
