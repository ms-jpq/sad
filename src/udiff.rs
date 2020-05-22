use difflib::sequencematcher::SequenceMatcher;
use std::rc::Rc;

pub struct DiffRange {
  pub r11: usize,
  pub r12: usize,
  pub r21: usize,
  pub r22: usize,
}

pub enum DiffLine {
  Iden(String),
  Plus(String),
  Minus(String),
}

pub struct Hunk {
  pub name: String,
  pub range: DiffRange,
  intern: Vec<DiffLine>,
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

fn diff_iter(
  n: usize,
  before: &str,
  after: &str,
  new_hunk: &mut impl FnMut(DiffRange),
  eq: &mut impl FnMut(&str),
  plus: &mut impl FnMut(&str),
  minus: &mut impl FnMut(&str),
) {
  let before = before.split_terminator('\n').collect::<Vec<&str>>();
  let after = after.split_terminator('\n').collect::<Vec<&str>>();
  let mut matcher = SequenceMatcher::new(&before, &after);
  for group in &matcher.get_grouped_opcodes(n) {
    let (first, last) = (group.first().unwrap(), group.last().unwrap());
    let range = DiffRange {
      r11 : first.first_start,
      r12 : last.first_end,
      r21 : first.second_start,
      r22 : last.second_end
    };
    new_hunk(range);
    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          eq(*line)
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "delete" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          minus(*line)
        }
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          plus(*line)
        }
      }
    }
  }
}

pub fn udiff(hunk_size: usize, name: &str, before: &str, after: &str) -> String {
  let mut print = Rc::new(vec![
    format!("\ndiff --git {} {}", name, name),
    format!("--- {}", name),
    format!("+++ {}", name),
  ]);

  let mut new_hunk = |size| {

  };
  let mut eq = |line: &str| {};
  let mut plus = |line: &str| {};
  let mut minus = |line: &str| {};

  diff_iter(hunk_size, before, after, &mut new_hunk, &mut eq, &mut plus, &mut minus);
  print.join("\n")
}
