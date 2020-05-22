use difflib::sequencematcher::SequenceMatcher;


// pub struct Hunk {
//   pub name: String,
//   pub range: (usize, usize),
//   pub lines: Vec<String>,
// }


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
    let (first, last) = (group.first().unwrap(), group.last().unwrap());
    let r1 = format_range_unified(first.first_start, last.first_end);
    let r2 = format_range_unified(first.second_start, last.second_end);
    print.push(format!("@@ -{} +{} @@", r1, r2));
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
