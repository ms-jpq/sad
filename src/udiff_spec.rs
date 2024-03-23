#[cfg(test)]
mod spec {
  use super::super::udiff::{apply_patches, patches, pure_diffs, udiff};
  use difflib::unified_diff;
  use regex::Regex;
  use std::{
    collections::HashSet,
    fs::{read_dir, read_to_string},
    path::PathBuf,
  };

  fn read_files() -> Vec<String> {
    let mut source = read_dir(PathBuf::from("src"))
      .unwrap()
      .map(|entry| {
        let path = entry.unwrap().path();
        read_to_string(path).unwrap()
      })
      .collect::<Vec<_>>();
    let tests = read_dir(PathBuf::from("tests"))
      .unwrap()
      .map(|entry| {
        let path = entry.unwrap().path();
        read_to_string(path).unwrap()
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
    .map(|(s1, s2)| (Regex::new(s1).unwrap(), s2.to_owned()))
    .collect::<_>()
  }

  fn diffs() -> Vec<(Vec<String>, Vec<String>)> {
    let texts = read_files();
    let regexes = regexes();
    let mut acc = Vec::new();
    for text in texts {
      for re in &regexes {
        let before = text
          .clone()
          .split_inclusive('\n')
          .map(String::from)
          .collect::<Vec<_>>();
        let after = re
          .0
          .replace_all(text.as_str(), re.1.as_str())
          .to_string()
          .split_inclusive('\n')
          .map(String::from)
          .collect::<Vec<_>>();
        acc.push((before, after));
      }
    }
    acc
  }

  #[test]
  fn patch() {
    let diffs = diffs();
    for (unified, (before, after)) in diffs.into_iter().enumerate() {
      let ranges = pure_diffs(unified, &before, &after);
      let rangeset = ranges.into_iter().collect::<HashSet<_>>();

      let ps = patches(unified, &before, &after);
      let patched = apply_patches(ps, &rangeset, &before);
      let imp = patched.into_iter().map(String::from).collect::<Vec<_>>();
      assert_eq!(imp, after);
    }
  }

  #[test]
  fn unified() {
    let diffs = diffs();
    for (unified, (before, after)) in diffs.into_iter().enumerate() {
      let canon = unified_diff(&before, &after, "", "", "", "", unified)
        .iter()
        .skip(2)
        .map(|s| {
          if s.starts_with("@@") {
            "@@".to_owned()
          } else {
            s.clone()
          }
        })
        .collect::<Vec<_>>();
      let imp = udiff(None, unified, Default::default(), &before, &after)
        .to_string_lossy()
        .split_inclusive('\n')
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
    }
  }
}
