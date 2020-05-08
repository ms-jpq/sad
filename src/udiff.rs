use difflib;

pub fn udiff(name: &str, before: &str, after: &str) -> String {
  let before_split = before.split_terminator("\n").collect::<Vec<&str>>();
  let after_split = after.split_terminator("\n").collect::<Vec<&str>>();
  let diff = difflib::unified_diff(&before_split, &after_split, name, name, "", "", 3);
  let prefix = vec![
    format!("\ndiff --git {}", name),
    format!("--- {}", name),
    format!("+++ {}", name),
  ];
  let print = itertools::chain(prefix, diff.into_iter().skip(2));
  itertools::join(print, "\n")
}
