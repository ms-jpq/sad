use diff;
use std::collections::VecDeque;

fn parse(lines: &[diff::Result<&str>], unified_lines: isize) -> Vec<String> {
  let mut diffs = Vec::<String>::new();

  let mut old_idx = 1;
  let mut new_idx = 1;

  for line in lines {
    match line {
      diff::Result::Left(l) => {
        let line = format!("-{}\n", l);
        diffs.push(line);
        old_idx += 1;
      }
      diff::Result::Right(r) => {
        let line = format!("+{}\n", r);
        diffs.push(line);
        new_idx += 1;
      }
      diff::Result::Both(l, _) => {
        let line = format!(" {}\n", l);
        diffs.push(line);
        old_idx += 1;
        new_idx += 1;
      }
    };
  }
  diffs
}

pub fn udiff(name: &str, before: &str, after: &str) -> String {
  let headers = vec![
    String::from(format!("diff --git {} {}\n", name, name)),
    String::from(format!("--- {}\n", name)),
    String::from(format!("+++ {}\n", name)),
  ];
  let changes = diff::lines(before, after);
  let diffs = parse(&changes, 3);
  let print = itertools::chain(headers, diffs);
  itertools::join(print, "")
}
