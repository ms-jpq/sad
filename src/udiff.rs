use difflib::{sequencematcher::Opcode, sequencematcher::SequenceMatcher};
use std::fmt::{self, Display, Formatter};

pub struct DiffRange {
  r1: (usize, usize),
  r2: (usize, usize),
}

impl DiffRange {
  fn new(ops: &[Opcode]) -> Option<DiffRange> {
    match (ops.first(), ops.last()) {
      (Some(first), Some(last)) => Some(DiffRange {
        r1: (first.first_start, last.first_end),
        r2: (first.second_start, last.second_end),
      }),
      _ => None,
    }
  }
}

impl Display for DiffRange {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "@@ -{} +{} @@", format_range_unified(self.r1), format_range_unified(self.r2))
  }
}

fn format_range_unified((start, end): (usize, usize)) -> String {
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

pub fn udiff(hunk_size: usize, name: &str, before: &str, after: &str) -> String {
  let before = before.split_terminator('\n').collect::<Vec<&str>>();
  let after = after.split_terminator('\n').collect::<Vec<&str>>();
  let mut print = vec![
    format!("\ndiff --git {} {}", name, name),
    format!("--- {}", name),
    format!("+++ {}", name),
  ];
  let mut matcher = SequenceMatcher::new(&before, &after);
  for group in &matcher.get_grouped_opcodes(hunk_size) {
    print.push(format!("{}", DiffRange::new(group).unwrap()));
    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          print.push(format!(" {}", line))
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "delete" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          print.push(format!("-{}", line))
        }
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          print.push(format!("+{}", line))
        }
      }
    }
  }
  print.join("\n")
}
