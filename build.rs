use uuid::Uuid;

fn main() {
  println!(
    "cargo:rustc-env={env}={uuid}",
    env = "SAD_PREVIEW_UUID",
    uuid = Uuid::new_v4()
  );

  println!(
    "cargo:rustc-env={env}={uuid}",
    env = "SAD_PATCH_UUID",
    uuid = Uuid::new_v4()
  );
}
