use std::collections::HashMap;

use lsp_types::Uri;
use mf2_parser::ast::SimpleMessage;
use mf2_parser::parse;
use mf2_parser::SourceTextInfo;
use yoke::Yoke;
use yoke::Yokeable;

#[derive(Default)]
pub struct Documents {
  documents: HashMap<Uri, Document>,
}

pub struct Document {
  pub parsed: Yoke<ParsedDocumet<'static>, Box<str>>,
  pub version: i32,
}

#[derive(Yokeable)]
pub struct ParsedDocumet<'a> {
  pub _ast: SimpleMessage<'a>,
  pub text_info: SourceTextInfo<'a>,
  pub diagnostics: Vec<mf2_parser::Diagnostic<'a>>,
}

impl Documents {
  pub fn on_change(
    &mut self,
    uri: Uri,
    version: i32,
    text: String,
  ) -> &Document {
    let text = text.into_boxed_str();
    let parsed = Yoke::attach_to_cart(text, |t| ParsedDocumet::new(t));
    let document = Document { parsed, version };
    match self.documents.entry(uri) {
      std::collections::hash_map::Entry::Occupied(mut entry) => {
        *entry.get_mut() = document;
        entry.into_mut()
      }
      std::collections::hash_map::Entry::Vacant(entry) => {
        &*entry.insert(document)
      }
    }
  }

  pub fn on_close(&mut self, uri: Uri) {
    self.documents.remove(&uri);
  }

  pub fn get(&self, uri: &Uri) -> Option<&Document> {
    self.documents.get(uri)
  }
}

impl Document {
  pub fn diagnostics(&self) -> Vec<lsp_types::Diagnostic> {
    let parsed = self.parsed.get();

    parsed
      .diagnostics
      .iter()
      .map(|diag| {
        let span = diag.span();

        fn loc_to_pos(
          info: &SourceTextInfo,
          loc: mf2_parser::Location,
        ) -> lsp_types::Position {
          let (line, character) = info.utf16_line_col(loc);
          lsp_types::Position { line, character }
        }

        lsp_types::Diagnostic {
          range: lsp_types::Range {
            start: loc_to_pos(&parsed.text_info, span.start),
            end: loc_to_pos(&parsed.text_info, span.end),
          },
          severity: Some(lsp_types::DiagnosticSeverity::ERROR),
          message: diag.to_string(),
          source: Some("mf2".to_string()),
          ..lsp_types::Diagnostic::default()
        }
      })
      .collect()
  }
}

impl ParsedDocumet<'_> {
  pub fn new(text: &str) -> ParsedDocumet {
    let (ast, diagnostics, text_info) = parse(text);
    ParsedDocumet {
      _ast: ast,
      text_info,
      diagnostics,
    }
  }
}
