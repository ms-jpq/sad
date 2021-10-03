use difflib::{sequencematcher::Opcode, sequencematcher::SequenceMatcher};
use std::{
  collections::HashSet,
  fmt::{self, Display, Formatter},
};

#[derive(Debug, Eq, Hash, PartialEq)]
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

pub fn pure_diffs(unified: usize, before: &str, after: &str) -> Vec<DiffRange> {
  let before = before.split_inclusive("\n").collect::<Vec<_>>();
  let after = after.split_inclusive("\n").collect::<Vec<_>>();
  let mut ret = Vec::new();
  let mut matcher = SequenceMatcher::new(&before, &after);
  for group in &matcher.get_grouped_opcodes(unified) {
    let range = DiffRange::new(group).expect("algo failure");
    ret.push(range);
  }
  ret
}

pub struct Patch {
  range: DiffRange,
  new_lines: Vec<String>,
}

pub fn patches(unified: usize, before: &str, after: &str) -> Vec<Patch> {
  let before = before.split_inclusive("\n").collect::<Vec<_>>();
  let after = after.split_inclusive("\n").collect::<Vec<_>>();

  let mut ret = Vec::new();
  let mut matcher = SequenceMatcher::new(&before, &after);

  for group in &matcher.get_grouped_opcodes(unified) {
    let mut new_lines = Vec::new();
    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          new_lines.push((*line).to_owned());
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          new_lines.push((*line).to_owned());
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

pub fn apply_patches(patches: Vec<Patch>, ranges: &HashSet<DiffRange>, before: &str) -> String {
  let before = before.split_inclusive("\n").collect::<Vec<_>>();
  let mut ret = String::new();
  let mut prev = 0;

  for diff in patches.iter() {
    let (before_start, before_inc) = diff.range.before;
    let before_end = before_start + before_inc;
    for i in prev..before_start {
      before
        .get(i)
        .map(|b| ret.push_str(b))
        .expect("algo failure");
    }
    if ranges.contains(&diff.range) {
      for line in diff.new_lines.iter() {
        ret.push_str(line);
      }
    } else {
      for i in before_start..before_end {
        before
          .get(i)
          .map(|b| ret.push_str(b))
          .expect("algo failure");
      }
    }
    prev = before_end;
  }
  for i in prev..before.len() {
    before
      .get(i)
      .map(|b| ret.push_str(b))
      .expect("algo failure");
  }
  ret
}

pub fn udiff(
  ranges: Option<&HashSet<DiffRange>>,
  unified: usize,
  name: &str,
  before: &str,
  after: &str,
) -> String {
  let before = before.split_inclusive("\n").collect::<Vec<_>>();
  let after = after.split_inclusive("\n").collect::<Vec<_>>();

  let mut ret = String::new();
  ret.push_str(&format!("diff --git {} {}\n", name, name));
  ret.push_str(&format!("--- {}\n", name));
  ret.push_str(&format!("+++ {}\n", name));

  let mut matcher = SequenceMatcher::new(&before, &after);
  for group in &matcher.get_grouped_opcodes(unified) {
    let range = DiffRange::new(group).expect("algo failure");
    if let Some(ranges) = &ranges {
      if !ranges.contains(&range) {
        continue;
      }
    };
    ret.push_str(&format!("{}\n", range));
    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push_str(&format!(" {}", *line))
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "delete" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push_str(&format!("-{}", *line))
        }
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          ret.push_str(&format!("+{}", *line))
        }
      }
    }
  }
  ret
}

#[cfg(test)]
mod tests {
  use super::*;
  use difflib::unified_diff;
  use regex::Regex;
  use std::{cmp::max, collections::HashSet, fs, path::PathBuf};

  fn read_files() -> Vec<String> {
    let mut source = fs::read_dir(PathBuf::from("src"))
      .unwrap()
      .map(|entry| {
        let path = entry.unwrap().path();
        fs::read_to_string(path).unwrap()
      })
      .collect::<Vec<_>>();
    let tests = fs::read_dir(PathBuf::from("tests"))
      .unwrap()
      .map(|entry| {
        let path = entry.unwrap().path();
        fs::read_to_string(path).unwrap()
      })
      .collect::<Vec<_>>();
    source.extend(tests);
    source
  }

  fn regexes() -> Vec<(Regex, String)> {
    vec![
      (r"std", "owo"),
      (r"<([^\)])>", "\\|$1"),
      (r"\n", r""),
      (r"use [^\n]+\n", ""),
      (r"use [^\n]+\n", "\n\nowo\n\nowo"),
      (r"\n+", ""),
      (r"\n+", "\n"),
    ]
    .into_iter()
    .map(|(s1, s2)| (Regex::new(&s1).unwrap(), s2))
    .collect::<_>()
  }

  fn diffs() -> Vec<(String, String)> {
    let texts = read_files();
    let regexes = regexes();
    let mut acc = Vec::new();
    for text in texts {
      for re in &regexes {
        let before = text.clone();
        let after = re.0.replace_all(text.as_str(), re.1.as_str());
        acc.push((before, after.to_string()))
      }
    }
    acc
  }

  #[test]
  fn patch() {
    let mut unified = 0;
    let diffs = diffs();
    for (before, after) in diffs {
      let ranges = pure_diffs(unified, &before, &after);
      let rangeset = ranges.into_iter().collect::<HashSet<_>>();

      let patches = patches(unified, &before, &after);
      let patched = apply_patches(patches, &rangeset, &before);

      let canon = after.lines().map(String::from).collect::<Vec<_>>();
      let imp = patched.lines().map(String::from).collect::<Vec<_>>();
      let len = max(canon.len(), imp.len());
      for i in 0..len {
        assert_eq!(canon[i], imp[i]);
      }
      assert_eq!(after, patched);
      unified += 1;
    }
  }

  #[test]
  fn unified() {
    let mut unified = 1;
    let diffs = diffs();
    for (before, after) in diffs {
      let bb = before.lines().collect::<Vec<_>>();
      let aa = after.lines().collect::<Vec<_>>();
      let canon = unified_diff(&bb, &aa, "", "", "", "", unified)
        .iter()
        .skip(2)
        .map(|s| {
          if s.starts_with("@@") {
            "@@".to_owned()
          } else {
            s.to_owned()
          }
        })
        .collect::<Vec<_>>();
      let imp = udiff(None, unified, "", &before, &after)
        .lines()
        .skip(3)
        .map(|s| {
          if s.starts_with("@@") {
            "@@".to_owned()
          } else {
            s.to_owned()
          }
        })
        .collect::<Vec<_>>();

      assert_eq!(imp, canon);
      unified += 1;
    }
  }
}
