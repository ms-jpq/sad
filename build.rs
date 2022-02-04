use uuid::Uuid;

fn main() {
  let under_the_sea = "SAD_ARGS_ENV";
  let sea_id = Uuid::new_v4().to_string();
  println!("cargo:rustc-env={under_the_sea}={sea_id}");
}
