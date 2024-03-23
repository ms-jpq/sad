use {
  difflib::sequencematcher::{Opcode, SequenceMatcher},
  std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    fmt::{self, Display, Formatter},
  },
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct DiffRange {
  pub before: (usize, usize),
  pub after: (usize, usize),
}

impl DiffRange {
  pub const fn new(ops: &[Opcode]) -> Option<Self> {
    match (ops.first(), ops.last()) {
      (Some(first), Some(last)) => Some(Self {
        before: (first.first_start, last.first_end - first.first_start),
        after: (first.second_start, last.second_end - first.second_start),
      }),
      _ => None,
    }
  }
}

impl Display for DiffRange {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let (before_lo, before_hi) = (self.before.0 + 1, self.before.1);
    let (after_lo, after_hi) = (self.after.0 + 1, self.after.1);

    write!(f, "@@ -{before_lo},{before_hi} +{after_lo},{after_hi} @@")
  }
}

pub struct Patch<S> {
  range: DiffRange,
  new_lines: Vec<S>,
}

pub fn pure_diffs(unified: usize, before: &[String], after: &[String]) -> Vec<DiffRange> {
  let mut ret = Vec::new();
  let mut matcher = SequenceMatcher::new(before, after);
  for group in &matcher.get_grouped_opcodes(unified) {
    let range = DiffRange::new(group).expect("algo failure");
    ret.push(range);
  }
  ret
}

pub fn patches<'a>(
  unified: usize,
  before: &'a [String],
  after: &'a [String],
) -> Vec<Patch<&'a str>> {
  let mut ret = Vec::new();
  let mut matcher = SequenceMatcher::new(before, after);

  for group in &matcher.get_grouped_opcodes(unified) {
    let mut new_lines = Vec::new();
    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          new_lines.push(line.as_str());
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          new_lines.push(line);
        }
      }
    }
    let diff = Patch {
      range: DiffRange::new(group).expect("algo failure"),
      new_lines,
    };
    ret.push(diff);
  }
  ret
}

pub fn apply_patches<'a>(
  patches: Vec<Patch<&'a str>>,
  ranges: &HashSet<DiffRange>,
  before: &'a [String],
) -> Vec<&'a str> {
  let mut ret = Vec::new();
  let mut prev = 0;

  for diff in patches {
    let (before_start, before_inc) = diff.range.before;
    let before_end = before_start + before_inc;
    for i in prev..before_start {
      before
        .get(i)
        .map(|b| ret.push(b.as_str()))
        .expect("algo failure");
    }
    if ranges.contains(&diff.range) {
      for line in &diff.new_lines {
        ret.push(line);
      }
    } else {
      for i in before_start..before_end {
        before.get(i).map(|b| ret.push(b)).expect("algo failure");
      }
    }
    prev = before_end;
  }
  for i in prev..before.len() {
    before.get(i).map(|b| ret.push(b)).expect("algo failure");
  }
  ret
}

pub fn udiff(
  ranges: Option<&HashSet<DiffRange>>,
  unified: usize,
  name: &OsStr,
  before: &[String],
  after: &[String],
) -> OsString {
  let mut ret = OsString::new();

  ret.push("diff --git ");
  ret.push(name);
  ret.push(" ");
  ret.push(name);
  ret.push("\n");

  ret.push("--- ");
  ret.push(name);
  ret.push("\n");

  ret.push("+++ ");
  ret.push(name);
  ret.push("\n");

  let mut matcher = SequenceMatcher::new(before, after);
  for group in &matcher.get_grouped_opcodes(unified) {
    let range = DiffRange::new(group).expect("algo failure");
    if let Some(ranges) = ranges {
      if !ranges.contains(&range) {
        continue;
      }
    };

    ret.push(&format!("{range}\n"));

    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push(" ");
          ret.push(line);
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "delete" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push("-");
          ret.push(line);
        }
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          ret.push("+");
          ret.push(line);
        }
      }
    }
  }
  ret
}
