use lsp_types::Range;
use lsp_types::Uri;
use mf2_parser::ast;
use mf2_parser::ast::Message;
use mf2_parser::Diagnostic;
use mf2_parser::LineColUtf16;
use mf2_parser::Location;
use mf2_parser::Scope;
use mf2_parser::SourceTextInfo;
use mf2_parser::Span;
use yoke::Yoke;
use yoke::Yokeable;

use crate::ast_utils::find_node;

pub struct Document {
  pub uri: Uri,
  pub version: i32,
  pub parsed: Yoke<ParsedDocument<'static>, Box<str>>,
}

#[derive(Yokeable)]
pub struct ParsedDocument<'text> {
  pub ast: Message<'text>,
  pub diagnostics: Vec<Diagnostic<'text>>,
  pub info: SourceTextInfo<'text>,
  pub scope: Scope<'text>,
}

impl Document {
  pub fn new(uri: Uri, version: i32, text: Box<str>) -> Document {
    let parsed = Yoke::attach_to_cart(text, |text| {
      let (ast, mut diagnostics, info) = mf2_parser::parse(text);
      let scope = mf2_parser::analyze_semantics(&ast, &mut diagnostics);

      ParsedDocument {
        ast,
        info,
        scope,
        diagnostics,
      }
    });
    Document {
      uri,
      version,
      parsed,
    }
  }

  pub fn loc_to_pos(&self, loc: mf2_parser::Location) -> lsp_types::Position {
    let LineColUtf16 { line, col } = self.parsed.get().info.utf16_line_col(loc);
    lsp_types::Position {
      line,
      character: col,
    }
  }

  pub fn pos_to_loc(&self, pos: lsp_types::Position) -> mf2_parser::Location {
    self.parsed.get().info.utf16_loc(LineColUtf16 {
      line: pos.line,
      col: pos.character,
    })
  }

  pub fn span_to_range(&self, span: Span) -> Range {
    Range {
      start: self.loc_to_pos(span.start),
      end: self.loc_to_pos(span.end),
    }
  }

  pub fn range_to_span(&self, range: Range) -> Span {
    Span {
      start: self.pos_to_loc(range.start),
      end: self.pos_to_loc(range.end),
    }
  }

  pub fn span_len(&self, span: Span) -> u32 {
    self.parsed.get().info.utf16_len(span)
  }

  pub fn ast(&self) -> &Message {
    &self.parsed.get().ast
  }

  pub fn scope(&self) -> &Scope {
    &self.parsed.get().scope
  }

  pub fn info(&self) -> &SourceTextInfo {
    &self.parsed.get().info
  }

  pub fn diagnostics(&self) -> &Vec<Diagnostic> {
    &self.parsed.get().diagnostics
  }

  pub fn find_variable_at(&self, loc: Location) -> Option<&str> {
    match find_node(self.ast(), loc) {
      Some(ast::AnyNode::Variable(node)) => Some(node.name),
      _ => None,
    }
  }
}
