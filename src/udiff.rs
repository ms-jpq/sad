use difflib::{sequencematcher::Opcode, sequencematcher::SequenceMatcher};
use std::{
  collections::HashSet,
  fmt::{self, Display, Formatter},
};

#[derive(Eq, Hash, PartialEq)]
pub struct DiffRange {
  pub before: (usize, usize),
  pub after: (usize, usize),
}

impl DiffRange {
  pub fn new(ops: &[Opcode]) -> Option<DiffRange> {
    match (ops.first(), ops.last()) {
      (Some(first), Some(last)) => Some(DiffRange {
        before: (first.first_start, last.first_end - first.first_start),
        after: (first.second_start, last.second_end - first.second_start),
      }),
      _ => None,
    }
  }
}

impl Display for DiffRange {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(
      f,
      "@@ -{},{} +{},{} @@",
      self.before.0 + 1,
      self.before.1,
      self.after.0 + 1,
      self.after.1,
    )
  }
}

pub type DiffRanges = Vec<DiffRange>;

pub trait Picker {
  fn new(unified: usize, before: &str, after: &str) -> Self;
}

impl Picker for DiffRanges {
  fn new(unified: usize, before: &str, after: &str) -> Self {
    let before = before.split_terminator('\n').collect::<Vec<&str>>();
    let after = after.split_terminator('\n').collect::<Vec<&str>>();
    let mut ret = Vec::new();
    let mut matcher = SequenceMatcher::new(&before, &after);
    for group in &matcher.get_grouped_opcodes(unified) {
      let range = DiffRange::new(group).unwrap();
      ret.push(range);
    }
    ret
  }
}

pub struct Diff {
  range: DiffRange,
  new_lines: Vec<String>,
}

pub type Diffs = Vec<Diff>;

pub trait Patchable {
  fn new(unified: usize, before: &str, after: &str) -> Self;
  fn patch(&self, ranges: &HashSet<DiffRange>, before: &str) -> String;
}

impl Patchable for Diffs {
  fn new(unified: usize, before: &str, after: &str) -> Self {
    let before = before.split_terminator('\n').collect::<Vec<&str>>();
    let after = after.split_terminator('\n').collect::<Vec<&str>>();

    let mut ret = Vec::new();
    let mut matcher = SequenceMatcher::new(&before, &after);

    for group in &matcher.get_grouped_opcodes(unified) {
      let mut new_lines = Vec::new();
      for code in group {
        if code.tag == "equal" {
          for line in before.iter().take(code.first_end).skip(code.first_start) {
            new_lines.push((*line).to_string());
          }
          continue;
        }
        if code.tag == "replace" || code.tag == "insert" {
          for line in after.iter().take(code.second_end).skip(code.second_start) {
            new_lines.push((*line).to_string());
          }
        }
      }
      let diff = Diff {
        range: DiffRange::new(group).unwrap(),
        new_lines,
      };
      ret.push(diff);
    }
    ret
  }

  fn patch(&self, ranges: &HashSet<DiffRange>, before: &str) -> String {
    let before = before.split_terminator('\n').collect::<Vec<&str>>();
    let mut ret = String::new();
    let mut prev = 0;

    for diff in self.iter() {
      let (before_start, before_inc) = diff.range.before;
      let before_end = before_start + before_inc;
      for i in prev..before_start {
        before.get(i).map(|b| ret.push_str(b)).unwrap();
        ret.push('\n');
      }
      if ranges.contains(&diff.range) {
        for line in diff.new_lines.iter() {
          ret.push_str(line);
          ret.push('\n')
        }
      } else {
        for i in before_start..before_end {
          before.get(i).map(|b| ret.push_str(b)).unwrap();
          ret.push('\n')
        }
      }
      prev = before_end;
    }
    for i in prev..before.len() {
      before.get(i).map(|b| ret.push_str(b)).unwrap();
      ret.push('\n')
    }
    ret
  }
}

pub fn udiff(
  ranges: Option<&HashSet<DiffRange>>,
  unified: usize,
  name: &str,
  before: &str,
  after: &str,
) -> String {
  let before = before.split_terminator('\n').collect::<Vec<&str>>();
  let after = after.split_terminator('\n').collect::<Vec<&str>>();

  let mut ret = String::new();
  ret.push_str(&format!("\ndiff --git {} {}", name, name));
  ret.push_str(&format!("\n--- {}", name));
  ret.push_str(&format!("\n+++ {}", name));

  let mut matcher = SequenceMatcher::new(&before, &after);
  for group in &matcher.get_grouped_opcodes(unified) {
    let range = DiffRange::new(group).unwrap();
    if let Some(ranges) = &ranges {
      if !ranges.contains(&range) {
        continue;
      }
    };
    ret.push_str(&format!("\n{}", range));
    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push_str(&format!("\n {}", *line))
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "delete" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push_str(&format!("\n-{}", *line))
        }
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          ret.push_str(&format!("\n+{}", *line))
        }
      }
    }
  }
  ret
}
