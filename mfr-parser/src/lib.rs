use mf2_parser::{Location, SourceTextInfo, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceDiagnosticCategory {
  Structure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceDiagnosticSeverity {
  Error,
}

#[derive(Debug, Clone)]
pub struct ResourceDiagnostic {
  pub code: &'static str,
  pub message: String,
  pub span: Span,
  pub severity: ResourceDiagnosticSeverity,
  pub category: ResourceDiagnosticCategory,
}

#[derive(Debug)]
pub struct ResourceValue {
  pub text: String,
  /// Maps each byte index in `text` back to a byte offset in the original document.
  pub source_offsets: Vec<u32>,
  pub span: Span,
}

#[derive(Debug)]
pub struct Entry<'text> {
  pub id: &'text str,
  pub id_span: Span,
  pub value: ResourceValue,
  pub span: Span,
  pub section_path: Vec<&'text str>,
  pub attached_metadata: Vec<usize>,
  pub attached_comments: Vec<usize>,
}

#[derive(Debug)]
pub struct Metadata<'text> {
  pub name: &'text str,
  pub name_span: Span,
  pub value: Option<ResourceValue>,
  pub span: Span,
}

#[derive(Debug)]
pub struct Section<'text> {
  pub id: &'text str,
  pub id_span: Span,
  pub span: Span,
}

#[derive(Debug)]
pub struct Frontmatter {
  pub span: Span,
}

#[derive(Debug)]
pub struct Comment {
  pub span: Span,
}

#[derive(Debug)]
pub enum ResourceLine {
  Frontmatter { span: Span },
  SectionHead { span: Span, section_idx: usize },
  Entry { span: Span, entry_idx: usize },
  Metadata { span: Span, metadata_idx: usize },
  Comment { span: Span, comment_idx: usize },
  Empty { span: Span },
  Invalid { span: Span },
}

#[derive(Debug, Default)]
pub struct ResourceDoc<'text> {
  pub lines: Vec<ResourceLine>,
  pub entries: Vec<Entry<'text>>,
  pub sections: Vec<Section<'text>>,
  pub metadata: Vec<Metadata<'text>>,
  pub comments: Vec<Comment>,
  pub frontmatter: Option<Frontmatter>,
}

#[derive(Debug)]
struct RawLine<'text> {
  start: usize,
  content_end: usize,
  newline_len: usize,
  text: &'text str,
}

impl<'text> RawLine<'text> {
  fn span(&self) -> Span {
    Span::new(
      Location::new_for_test(self.start as u32)
        ..Location::new_for_test(self.content_end as u32),
    )
  }

  fn content_no_cr(&self) -> &'text str {
    self.text.strip_suffix('\r').unwrap_or(self.text)
  }

  fn is_empty(&self) -> bool {
    self.content_no_cr().trim_matches([' ', '\t']).is_empty()
  }

  fn newline_offset_for_mapping(&self) -> Option<u32> {
    match self.newline_len {
      0 => None,
      1 => Some(self.content_end as u32),
      2 => Some((self.content_end + 1) as u32),
      _ => None,
    }
  }
}

pub fn parse_resource<'text>(
  text: &'text str,
) -> (
  ResourceDoc<'text>,
  Vec<ResourceDiagnostic>,
  SourceTextInfo<'text>,
) {
  let (_, _, info) = mf2_parser::parse(text);
  let lines = split_lines(text);
  let mut doc = ResourceDoc::default();
  let mut diagnostics = Vec::new();

  let mut idx = 0usize;
  let mut seen_frontmatter = false;
  let mut seen_section_or_entry = false;
  let mut section_path = Vec::<&'text str>::new();
  let mut pending_metadata = Vec::<usize>::new();
  let mut pending_comments = Vec::<usize>::new();

  while idx < lines.len() {
    let line = &lines[idx];
    let content = line.content_no_cr();

    if line.is_empty() {
      doc.lines.push(ResourceLine::Empty { span: line.span() });
      flush_pending_metadata(&mut diagnostics, &doc, &mut pending_metadata);
      pending_comments.clear();
      idx += 1;
      continue;
    }

    if starts_with_ws(content) {
      let span = line.span();
      diagnostics.push(diag(
        "invalid-line-indented",
        "Indented line is only valid as a value continuation after entry or metadata.",
        span,
      ));
      doc.lines.push(ResourceLine::Invalid { span });
      flush_pending_metadata(&mut diagnostics, &doc, &mut pending_metadata);
      pending_comments.clear();
      idx += 1;
      continue;
    }

    if content.starts_with('#') {
      let comment_idx = doc.comments.len();
      doc.comments.push(Comment { span: line.span() });
      doc.lines.push(ResourceLine::Comment {
        span: line.span(),
        comment_idx,
      });
      flush_pending_metadata(&mut diagnostics, &doc, &mut pending_metadata);
      pending_comments.push(comment_idx);
      idx += 1;
      continue;
    }

    if is_frontmatter(content) {
      let span = line.span();
      if seen_frontmatter {
        diagnostics.push(diag(
          "multiple-frontmatter",
          "A resource may contain at most one frontmatter line.",
          span,
        ));
      }
      if seen_section_or_entry {
        diagnostics.push(diag(
          "frontmatter-order",
          "Frontmatter must appear before any section headers or entries.",
          span,
        ));
      }

      seen_frontmatter = true;
      if doc.frontmatter.is_none() {
        doc.frontmatter = Some(Frontmatter { span });
      }
      doc.lines.push(ResourceLine::Frontmatter { span });
      pending_metadata.clear();
      pending_comments.clear();
      idx += 1;
      continue;
    }

    if content.starts_with('[') {
      let span = line.span();
      match parse_section(content) {
        Some((id, id_rel_start, id_rel_end)) => {
          let id_span = absolute_span(line.start, id_rel_start, id_rel_end);
          if !is_valid_id(id) {
            diagnostics.push(diag(
              "invalid-section-id",
              "Section identifier is not a valid resource id.",
              id_span,
            ));
          }
          section_path = id
            .split('.')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
          let section_idx = doc.sections.len();
          doc.sections.push(Section { id, id_span, span });
          doc
            .lines
            .push(ResourceLine::SectionHead { span, section_idx });
          seen_section_or_entry = true;
          pending_metadata.clear();
          pending_comments.clear();
        }
        None => {
          diagnostics.push(diag(
            "invalid-section-syntax",
            "Section header must use [id] syntax.",
            span,
          ));
          doc.lines.push(ResourceLine::Invalid { span });
          flush_pending_metadata(&mut diagnostics, &doc, &mut pending_metadata);
          pending_comments.clear();
        }
      }
      idx += 1;
      continue;
    }

    if content.starts_with('@') {
      let span = line.span();
      match parse_metadata_header(content) {
        Some((name, name_start, name_end, value_start)) => {
          let name_span = absolute_span(line.start, name_start, name_end);
          if !is_valid_id_part(name) {
            diagnostics.push(diag(
              "invalid-metadata-name",
              "Metadata key must be a valid id-part.",
              name_span,
            ));
          }
          let (value, consumed) = if let Some(value_start) = value_start {
            parse_multiline_value(&lines, idx, value_start)
          } else {
            (None, 0)
          };
          let metadata_idx = doc.metadata.len();
          doc.metadata.push(Metadata {
            name,
            name_span,
            value,
            span,
          });
          doc
            .lines
            .push(ResourceLine::Metadata { span, metadata_idx });
          pending_metadata.push(metadata_idx);
          idx += consumed + 1;
          continue;
        }
        None => {
          diagnostics.push(diag(
            "invalid-metadata-syntax",
            "Metadata line must start with @key and optional value.",
            span,
          ));
          doc.lines.push(ResourceLine::Invalid { span });
          flush_pending_metadata(&mut diagnostics, &doc, &mut pending_metadata);
          pending_comments.clear();
          idx += 1;
          continue;
        }
      }
    }

    if let Some((id, id_rel_start, id_rel_end, value_start)) =
      parse_entry_header(content)
    {
      seen_section_or_entry = true;
      let id_span = absolute_span(line.start, id_rel_start, id_rel_end);
      if !is_valid_id(id) {
        diagnostics.push(diag(
          "invalid-entry-id",
          "Entry identifier is not a valid resource id.",
          id_span,
        ));
      }
      let (value, consumed) = parse_multiline_value(&lines, idx, value_start);
      let Some(value) = value else {
        let span = absolute_span(line.start, value_start, value_start);
        diagnostics.push(diag(
          "missing-entry-value",
          "Entry is missing a value after '='.",
          span,
        ));
        doc.lines.push(ResourceLine::Invalid { span: line.span() });
        flush_pending_metadata(&mut diagnostics, &doc, &mut pending_metadata);
        pending_comments.clear();
        idx += 1;
        continue;
      };

      let span = Span::new(
        Location::new_for_test(line.start as u32)
          ..Location::new_for_test(lines[idx + consumed].content_end as u32),
      );
      let attached_metadata = std::mem::take(&mut pending_metadata);
      let attached_comments = std::mem::take(&mut pending_comments);
      let entry_idx = doc.entries.len();
      doc.entries.push(Entry {
        id,
        id_span,
        value,
        span,
        section_path: section_path.clone(),
        attached_metadata,
        attached_comments,
      });
      doc.lines.push(ResourceLine::Entry { span, entry_idx });
      idx += consumed + 1;
      continue;
    }

    let span = line.span();
    diagnostics.push(diag(
      "invalid-line",
      "Line is not valid frontmatter, section, entry, metadata, comment, or empty line.",
      span,
    ));
    doc.lines.push(ResourceLine::Invalid { span });
    flush_pending_metadata(&mut diagnostics, &doc, &mut pending_metadata);
    pending_comments.clear();
    idx += 1;
  }

  flush_pending_metadata(&mut diagnostics, &doc, &mut pending_metadata);

  (doc, diagnostics, info)
}

fn split_lines(text: &str) -> Vec<RawLine<'_>> {
  let mut lines = Vec::new();
  let bytes = text.as_bytes();
  let mut start = 0usize;

  while start <= bytes.len() {
    let mut end = start;
    while end < bytes.len() && bytes[end] != b'\n' {
      end += 1;
    }

    let newline_len = if end < bytes.len() { 1 } else { 0 };
    let content_end = if end > start && bytes[end - 1] == b'\r' {
      end - 1
    } else {
      end
    };

    lines.push(RawLine {
      start,
      content_end,
      newline_len: if content_end < end && newline_len == 1 {
        2
      } else {
        newline_len
      },
      text: &text[start..end],
    });

    if end == bytes.len() {
      break;
    }
    start = end + 1;
  }

  lines
}

fn parse_multiline_value(
  lines: &[RawLine<'_>],
  start_line_idx: usize,
  value_rel_start: usize,
) -> (Option<ResourceValue>, usize) {
  let mut text = String::new();
  let mut source_offsets = Vec::<u32>::new();

  let first_line = &lines[start_line_idx];
  let first_content = first_line.content_no_cr();
  let first_slice = if value_rel_start <= first_content.len() {
    &first_content[value_rel_start..]
  } else {
    ""
  };
  append_slice_with_mapping(
    first_slice,
    first_line.start + value_rel_start,
    &mut text,
    &mut source_offsets,
  );

  let mut consumed = 0usize;
  let mut end_offset = first_line.content_end;

  let mut i = start_line_idx + 1;
  while i < lines.len() {
    let line = &lines[i];
    let content = line.content_no_cr();

    if !starts_with_ws(content) {
      break;
    }

    if let Some(newline_source) =
      lines[start_line_idx + consumed].newline_offset_for_mapping()
    {
      text.push('\n');
      source_offsets.push(newline_source);
    } else {
      text.push('\n');
      source_offsets.push(lines[start_line_idx + consumed].content_end as u32);
    }

    let indent = content
      .chars()
      .take_while(|c| *c == ' ' || *c == '\t')
      .map(char::len_utf8)
      .sum::<usize>();
    let continuation = &content[indent..];
    append_slice_with_mapping(
      continuation,
      line.start + indent,
      &mut text,
      &mut source_offsets,
    );

    consumed += 1;
    end_offset = line.content_end;
    i += 1;
  }

  let span = Span::new(
    Location::new_for_test((first_line.start + value_rel_start) as u32)
      ..Location::new_for_test(end_offset as u32),
  );

  (
    Some(ResourceValue {
      text,
      source_offsets,
      span,
    }),
    consumed,
  )
}

fn append_slice_with_mapping(
  slice: &str,
  source_start: usize,
  out_text: &mut String,
  source_offsets: &mut Vec<u32>,
) {
  out_text.push_str(slice);
  source_offsets.extend((0..slice.len()).map(|i| (source_start + i) as u32));
}

fn parse_section(content: &str) -> Option<(&str, usize, usize)> {
  let closing = content.rfind(']')?;
  if closing == 0 {
    return None;
  }
  let rest = content[closing + 1..].trim();
  if !rest.is_empty() {
    return None;
  }
  let inside = content[1..closing].trim();
  if inside.is_empty() {
    return None;
  }
  let inside_start = content.find(inside)?;
  Some((inside, inside_start, inside_start + inside.len()))
}

fn parse_metadata_header(
  content: &str,
) -> Option<(&str, usize, usize, Option<usize>)> {
  let mut chars = content.char_indices();
  let (_, first) = chars.next()?;
  if first != '@' {
    return None;
  }

  let mut end = 1usize;
  for (idx, ch) in chars {
    if ch == ' ' || ch == '\t' {
      break;
    }
    end = idx + ch.len_utf8();
  }

  if end <= 1 {
    return None;
  }

  let name = &content[1..end];
  let value_start = content[end..]
    .char_indices()
    .find(|(_, ch)| *ch != ' ' && *ch != '\t')
    .map(|(idx, _)| end + idx);

  Some((name, 1, end, value_start))
}

fn parse_entry_header(content: &str) -> Option<(&str, usize, usize, usize)> {
  let eq_idx = content.find('=')?;
  let id = content[..eq_idx].trim_end();
  if id.is_empty() {
    return None;
  }
  let id_start = content[..eq_idx]
    .char_indices()
    .find(|(_, ch)| *ch != ' ' && *ch != '\t')
    .map(|(idx, _)| idx)
    .unwrap_or(0);
  let value_start = content[eq_idx + 1..]
    .char_indices()
    .find(|(_, ch)| *ch != ' ' && *ch != '\t')
    .map(|(idx, _)| eq_idx + 1 + idx)
    .unwrap_or(eq_idx + 1);
  Some((id, id_start, id_start + id.len(), value_start))
}

fn is_frontmatter(content: &str) -> bool {
  let trimmed_end = content.trim_end_matches([' ', '\t']);
  trimmed_end == "---"
}

fn starts_with_ws(content: &str) -> bool {
  content.starts_with(' ') || content.starts_with('\t')
}

fn is_valid_id(id: &str) -> bool {
  id.split('.')
    .map(str::trim)
    .all(|part| !part.is_empty() && is_valid_id_part(part))
}

fn is_valid_id_part(part: &str) -> bool {
  if part.is_empty() {
    return false;
  }

  let mut chars = part.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\\' {
      if chars.next().is_none() {
        return false;
      }
      continue;
    }

    if ch == '-'
      || ch == '_'
      || ch.is_ascii_alphanumeric()
      || ch.is_alphabetic()
    {
      continue;
    }

    return false;
  }

  true
}

fn absolute_span(line_start: usize, rel_start: usize, rel_end: usize) -> Span {
  Span::new(
    Location::new_for_test((line_start + rel_start) as u32)
      ..Location::new_for_test((line_start + rel_end) as u32),
  )
}

fn diag(code: &'static str, message: &str, span: Span) -> ResourceDiagnostic {
  ResourceDiagnostic {
    code,
    message: message.to_string(),
    span,
    severity: ResourceDiagnosticSeverity::Error,
    category: ResourceDiagnosticCategory::Structure,
  }
}

fn flush_pending_metadata(
  diagnostics: &mut Vec<ResourceDiagnostic>,
  doc: &ResourceDoc,
  pending_metadata: &mut Vec<usize>,
) {
  if pending_metadata.is_empty() {
    return;
  }

  for metadata_idx in pending_metadata.drain(..) {
    let span = doc.metadata[metadata_idx].span;
    diagnostics.push(diag(
      "dangling-metadata",
      "Metadata must attach to a subsequent frontmatter, section header, or entry without comments/empty lines in between.",
      span,
    ));
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::parse_resource;

  #[test]
  fn detects_multiple_frontmatter_and_ordering() {
    let source = "a = hi\n---\n---\n";
    let (_doc, diagnostics, _info) = parse_resource(source);
    assert!(
      diagnostics.iter().any(|d| d.code == "frontmatter-order"),
      "missing frontmatter ordering diagnostic"
    );
    assert!(
      diagnostics.iter().any(|d| d.code == "multiple-frontmatter"),
      "missing multiple-frontmatter diagnostic"
    );
  }

  #[test]
  fn reports_invalid_indented_line() {
    let source = "  orphan\n";
    let (_doc, diagnostics, _info) = parse_resource(source);
    assert!(diagnostics
      .iter()
      .any(|d| d.code == "invalid-line-indented"));
  }

  #[test]
  fn reports_dangling_metadata() {
    let source = "@locale en-US\n\nentry = hi\n";
    let (_doc, diagnostics, _info) = parse_resource(source);
    assert!(diagnostics.iter().any(|d| d.code == "dangling-metadata"));
  }

  #[test]
  fn parses_multiline_entry_value_and_normalizes_crlf() {
    let source = "entry = one\r\n  two\r\n";
    let (doc, diagnostics, _info) = parse_resource(source);
    assert!(diagnostics.is_empty());
    assert_eq!(doc.entries.len(), 1);
    assert_eq!(doc.entries[0].value.text, "one\ntwo");
  }

  #[test]
  fn parses_sections_and_entry_ids() {
    let source = "[main.section]\nkey-name = hello\n";
    let (doc, diagnostics, _info) = parse_resource(source);
    assert!(diagnostics.is_empty());
    assert_eq!(doc.sections.len(), 1);
    assert_eq!(doc.entries.len(), 1);
    assert_eq!(doc.entries[0].section_path, vec!["main", "section"]);
  }

  #[test]
  fn fixture_suite_resource_mfr() {
    let mut fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.push("../test/fixtures/resource_mfr");
    let mut entries = std::fs::read_dir(&fixture_dir)
      .expect("fixture directory should exist")
      .map(|entry| entry.unwrap())
      .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
      let path = entry.path();
      let source = std::fs::read_to_string(&path).expect("fixture should load");
      let (_doc, diagnostics, _info) = parse_resource(&source);
      let filename = path.file_name().unwrap().to_string_lossy();

      if filename.starts_with("valid_") {
        assert!(
          diagnostics.is_empty(),
          "expected no diagnostics for fixture {filename}, got: {:?}",
          diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
      } else if filename.starts_with("invalid_") {
        assert!(
          !diagnostics.is_empty(),
          "expected diagnostics for fixture {filename}"
        );
      } else {
        panic!("unsupported fixture naming in {filename}");
      }
    }
  }
}
