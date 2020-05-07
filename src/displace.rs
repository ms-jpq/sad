use aho_corasick::AhoCorasick;
use regex::Regex;

mod displace;

pub fn re_displace<'a>(re: &Regex, replace: &str, text: &'a str) -> Cow<'a, str> {
  re.replace_all(text, replace)
}

pub fn ac_displace<'a>(ac: AhoCorasick, replace: &str, text: &str) -> String {
  ac.replace_all(text, &[replace])
}
