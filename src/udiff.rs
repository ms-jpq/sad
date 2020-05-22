use super::errors::*;
use difflib::{sequencematcher::Opcode, sequencematcher::SequenceMatcher};
use regex::Regex;
use std::{
  convert::TryFrom,
  fmt::{self, Display, Formatter},
};

pub struct DiffRange {
  before: (usize, usize),
  after: (usize, usize),
}

impl DiffRange {
  pub fn new(ops: &[Opcode]) -> Option<DiffRange> {
    match (ops.first(), ops.last()) {
      (Some(first), Some(last)) => Some(DiffRange {
        before: (first.first_start, last.first_end),
        after: (first.second_start, last.second_end),
      }),
      _ => None,
    }
  }
}

impl Display for DiffRange {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(
      f,
      "@@ -{} +{} @@",
      format_range_unified(self.before),
      format_range_unified(self.after)
    )
  }
}

fn format_range_unified((start, end): (usize, usize)) -> String {
  let length = end - start;
  let beginning = if length == 0 { start } else { start + 1 };
  format!("{},{}", beginning, length)
}

impl TryFrom<&str> for DiffRange {
  type Error = Failure;

  fn try_from(candidate: &str) -> SadResult<Self> {
    let preg = r"^@@ -(\d+),(\d+) \+(\d+),(\d+) @@$";
    let re = Regex::new(preg).into_sadness()?;
    let captures = re
      .captures(candidate)
      .ok_or_else(|| Failure::Parse(candidate.into()))?;
    let before_start = captures
      .get(1)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let before_inc = captures
      .get(2)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let after_start = captures
      .get(3)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let after_inc = captures
      .get(4)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;

    Ok(DiffRange {
      before: (before_start, before_inc),
      after: (after_start, after_inc),
    })
  }
}

pub fn udiff(unified: usize, name: &str, before: &str, after: &str) -> String {
  let before = before.split_terminator('\n').collect::<Vec<&str>>();
  let after = after.split_terminator('\n').collect::<Vec<&str>>();
  let mut ret = String::new();
  ret.push_str(&format!("\ndiff --git {} {}", name, name));
  ret.push_str(&format!("\n--- {}", name));
  ret.push_str(&format!("\n+++ {}", name));

  let mut matcher = SequenceMatcher::new(&before, &after);
  for group in &matcher.get_grouped_opcodes(unified) {
    ret.push_str(&format!("\n{}", DiffRange::new(group).unwrap()));
    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push_str(&format!("\n {}", line))
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "delete" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push_str(&format!("\n-{}", line))
        }
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          ret.push_str(&format!("\n+{}", line))
        }
      }
    }
  }
  ret
}

