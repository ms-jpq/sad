use difflib::sequencematcher::{Opcode, Sequence, SequenceMatcher};
use std::fmt::Display;

// pub struct Hunk {
//   pub name: String,
//   pub range: (usize, usize),
//   pub lines: Vec<String>,
// }

fn diff_iter<T: Sequence + Display>(
  before: &[T],
  after: &[T],
  n: usize,
  new_hunk: impl Fn(&Opcode, &Opcode),
  eq: impl Fn(&T),
  plus: impl Fn(&T),
  minus: impl Fn(&T),
) {
  let mut matcher = SequenceMatcher::new(before, after);
  for group in &matcher.get_grouped_opcodes(n) {
    let (first, last) = (group.first().unwrap(), group.last().unwrap());
    new_hunk(first, last);
    for code in group {
      if code.tag == "equal" {
        for item in before.iter().take(code.first_end).skip(code.first_start) {
          eq(item)
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "delete" {
        for item in before.iter().take(code.first_end).skip(code.first_start) {
          minus(item)
        }
      }
      if code.tag == "replace" || code.tag == "insert" {
        for item in after.iter().take(code.second_end).skip(code.second_start) {
          plus(item)
        }
      }
    }
  }
}

fn format_range_unified(start: usize, end: usize) -> String {
  let mut beginning = start + 1;
  let length = end - start;
  if length == 1 {
    return beginning.to_string();
  }
  if length == 0 {
    beginning -= 1;
  }
  format!("{},{}", beginning, length)
}

fn format_unified(first: &Opcode, last: &Opcode) -> String {
  let r1 = format_range_unified(first.first_start, last.first_end);
  let r2 = format_range_unified(first.second_start, last.second_end);
  format!("@@ -{} + {} @@", r1, r2)
}

pub fn udiff(hunk_size: usize, name: &str, before: &str, after: &str) -> String {
  let before_split = before.split_terminator('\n').collect::<Vec<&str>>();
  let after_split = after.split_terminator('\n').collect::<Vec<&str>>();
  let mut print = vec![
    format!("\ndiff --git {} {}", name, name),
    format!("--- {}", name),
    format!("+++ {}", name),
  ];
  let new_hunk = |fst: &Opcode, last: &Opcode| {
    print.push(format_unified(fst, last));
  };
  let eq = |line: &&str| {};
  let plus = |line: &&str| {};
  let minus = |line: &&str| {};
  diff_iter(
    &before_split,
    &after_split,
    hunk_size,
    new_hunk,
    eq,
    plus,
    minus,
  );
  print.join("\n")
}
