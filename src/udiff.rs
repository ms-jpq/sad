use diff;

fn parse(lines: &[diff::Result<&str>]) -> String {
  let mut diff = String::new();
  for (idx, line) in lines.into_iter().enumerate() {
    match line {
      diff::Result::Left(l) => {
        diff.push_str(format!("-{}{}\n", idx, l).as_str());
      }
      diff::Result::Right(r) => {
        diff.push_str(format!("+{}{}\n", idx, r).as_str());
      }
      diff::Result::Both(l, _) => {
        diff.push_str(format!(" {}{}\n", idx, l).as_str());
      }
    };
  }
  diff
}

pub fn udiff(before: &str, after: &str) -> String {
  let changes = diff::lines(before, after);
  parse(&changes)
}
