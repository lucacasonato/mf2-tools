use lsp_types::Diagnostic as LspDiagnostic;
use lsp_types::DiagnosticSeverity;
use lsp_types::Position;
use lsp_types::Range;
use mf2_parser::Location;
use mf2_parser::SourceTextInfo;
use mfr_parser::parse_resource;
use yoke::Yoke;
use yoke::Yokeable;

pub struct Document {
  pub version: i32,
  pub parsed: Yoke<ParsedDocument<'static>, Box<str>>,
}

#[derive(Yokeable)]
pub struct ParsedDocument<'text> {
  pub lsp_diagnostics: Vec<LspDiagnostic>,
  _phantom: std::marker::PhantomData<&'text ()>,
}

impl Document {
  pub fn new(_uri: lsp_types::Uri, version: i32, text: Box<str>) -> Document {
    let parsed = Yoke::attach_to_cart(text, |text| {
      let (resource, structural_diags, info) = parse_resource(text);
      let mut lsp_diagnostics = Vec::<LspDiagnostic>::new();

      for diag in structural_diags {
        lsp_diagnostics.push(to_lsp_diagnostic(&info, diag.span, diag.message));
      }

      for entry in &resource.entries {
        let (ast, mut value_diags, _) = mf2_parser::parse(&entry.value.text);
        let _scope = mf2_parser::analyze_semantics(&ast, &mut value_diags);

        for diag in value_diags {
          if let Some(span) =
            remap_value_span(&entry.value.source_offsets, diag.span())
          {
            lsp_diagnostics.push(to_lsp_diagnostic(
              &info,
              span,
              diag.message(),
            ));
          }
        }
      }

      for meta in &resource.metadata {
        if let Some(value) = &meta.value {
          let (ast, mut value_diags, _) = mf2_parser::parse(&value.text);
          let _scope = mf2_parser::analyze_semantics(&ast, &mut value_diags);

          for diag in value_diags {
            if let Some(span) =
              remap_value_span(&value.source_offsets, diag.span())
            {
              lsp_diagnostics.push(to_lsp_diagnostic(
                &info,
                span,
                diag.message(),
              ));
            }
          }
        }
      }

      ParsedDocument {
        lsp_diagnostics,
        _phantom: std::marker::PhantomData,
      }
    });

    Document { version, parsed }
  }

  pub fn lsp_diagnostics(&self) -> &Vec<LspDiagnostic> {
    &self.parsed.get().lsp_diagnostics
  }
}

fn remap_value_span(
  source_offsets: &[u32],
  span: mf2_parser::Span,
) -> Option<mf2_parser::Span> {
  let start = span.start.inner_byte_index_for_test() as usize;
  let end = span.end.inner_byte_index_for_test() as usize;

  if source_offsets.is_empty() {
    return None;
  }

  let mapped_start = if start < source_offsets.len() {
    source_offsets[start]
  } else {
    *source_offsets.last()? + 1
  };

  let mapped_end = if end == 0 {
    mapped_start
  } else if end - 1 < source_offsets.len() {
    source_offsets[end - 1] + 1
  } else {
    *source_offsets.last()? + 1
  };

  Some(mf2_parser::Span::new(
    Location::new_for_test(mapped_start)..Location::new_for_test(mapped_end),
  ))
}

fn to_lsp_diagnostic(
  info: &SourceTextInfo,
  span: mf2_parser::Span,
  message: String,
) -> LspDiagnostic {
  LspDiagnostic {
    range: Range {
      start: to_position(info, span.start),
      end: to_position(info, span.end),
    },
    severity: Some(DiagnosticSeverity::ERROR),
    code: None,
    code_description: None,
    source: Some("mfr".to_string()),
    message,
    related_information: None,
    tags: None,
    data: None,
  }
}

fn to_position(info: &SourceTextInfo, loc: Location) -> Position {
  let lc = info.utf16_line_col(loc);
  Position {
    line: lc.line,
    character: lc.col,
  }
}

#[cfg(test)]
mod tests {
  use lsp_types::Uri;

  use super::Document;

  #[test]
  fn reports_structural_and_value_diagnostics() {
    let source = "@locale en-US\n\n[item]\nbroken line\nentry = Hello {$name\n";
    let uri: Uri = "file:///sample.mfr".parse().unwrap();
    let document = Document::new(uri, 1, source.to_string().into_boxed_str());
    let diags = document.lsp_diagnostics();

    assert!(diags
      .iter()
      .any(|d| d.message.contains("Metadata must attach")));
    assert!(diags
      .iter()
      .any(|d| d.message.contains("Line is not valid")));
    assert!(diags
      .iter()
      .any(|d| d.message.contains("missing the closing brace")));
  }

  #[test]
  fn remaps_crlf_value_diagnostics() {
    let source = "entry = Hello {$name\r\n";
    let uri: Uri = "file:///sample-crlf.mfr".parse().unwrap();
    let document = Document::new(uri, 1, source.to_string().into_boxed_str());
    let diag = document
      .lsp_diagnostics()
      .iter()
      .find(|d| d.message.contains("missing the closing brace"))
      .expect("expected value diagnostic");

    assert_eq!(diag.range.start.line, 0);
  }
}
