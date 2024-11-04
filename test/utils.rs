use std::fmt::Write;
use std::iter;

use mf2_parser::Diagnostic;
use unicode_width::UnicodeWidthStr as _;

pub fn parse_fixture<'t>(
  mut raw: &'t str,
  headers: Vec<&str>,
) -> impl Iterator<Item = &'t str> {
  let mut parts = vec![];
  for header in headers {
    let (part, rest) = raw.split_once(header).unwrap_or((raw, ""));
    parts.push(part);
    raw = rest;
  }
  parts.push(raw);
  parts.into_iter()
}

pub fn normalize_message(message: &str) -> String {
  message
    .chars()
    .map(|c| match c {
      '\n' => '↵',
      '\t' => '⇥',
      c => c,
    })
    .collect::<String>()
}

pub fn generate_actual_diagnostics(
  diagnostics: &[Diagnostic],
  input_message: &str,
  normalized_message: &str,
) -> String {
  let mut formatted_diagnostics = "".to_string();
  for (i, diag) in diagnostics.iter().enumerate() {
    let span = diag.span();
    let span_start = span.start.inner_byte_index_for_test() as usize;
    let span_end = span.end.inner_byte_index_for_test() as usize;

    let prefix = &input_message[0..span_start];
    let contents = &input_message[span_start..span_end];

    if i != 0 {
      formatted_diagnostics.push('\n');
    }
    writeln!(formatted_diagnostics, "{}", diag).unwrap();
    formatted_diagnostics.push(' ');
    formatted_diagnostics.push(' ');
    formatted_diagnostics.push_str(normalized_message);
    formatted_diagnostics.push('\n');
    iter::repeat(' ')
      .take(prefix.width_cjk() + 2)
      .chain(iter::repeat('^').take(contents.width_cjk()))
      .for_each(|c| formatted_diagnostics.push(c));
  }
  formatted_diagnostics
}
