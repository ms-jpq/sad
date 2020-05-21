pub fn udiff(hunk_size: usize, name: &str, before: &str, after: &str) -> String {
  let delim = '\n';
  let before_split = before.split_terminator(delim).collect::<Vec<&str>>();
  let after_split = after.split_terminator(delim).collect::<Vec<&str>>();
  let diff = difflib::unified_diff(&before_split, &after_split, name, name, "", "", hunk_size);
  let prefix = vec![
    format!("\ndiff --git {} {}", name, name),
    format!("--- {}", name),
    format!("+++ {}", name),
  ];
  let body = diff
    .into_iter()
    .map(|line| match line.chars().next() {
      Some(ch) if ch == '@' => line.trim().into(),
      _ => line,
    })
    .skip(2);
  let end = vec!["\n".into()];
  let print = itertools::chain(itertools::chain(prefix, body), end);
  itertools::join(print, "\n")
}
