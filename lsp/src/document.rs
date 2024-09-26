use lsp_types::Position;
use lsp_types::Range;
use lsp_types::Uri;
use mf2_parser::ast::AnyNode;
use mf2_parser::ast::Message;
use mf2_parser::AnyNodeVisitor;
use mf2_parser::LineColUtf16;
use mf2_parser::SourceTextInfo;
use mf2_parser::Span;
use mf2_parser::Spanned;
use mf2_parser::Visit;
use yoke::Yoke;
use yoke::Yokeable;

use crate::diagnostics::Diagnostic;
use crate::scope::ScopeVisitor;

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
}

impl Document {
  pub fn new(uri: Uri, version: i32, text: Box<str>) -> Document {
    let parsed = Yoke::attach_to_cart(text, |text| {
      let (ast, parser_diagnostics, info) = mf2_parser::parse(text);

      let diagnostics = parser_diagnostics
        .into_iter()
        .map(Diagnostic::Parser)
        .collect();

      let diagnostics = {
        let mut scope_visitor = ScopeVisitor::new(diagnostics);
        scope_visitor.visit_message(&ast);
        scope_visitor.diagnostics
      };

      ParsedDocument {
        ast,
        info,
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

  pub fn find_node(&self, pos: Position) -> Option<AnyNode> {
    let location = self.pos_to_loc(pos);
    let ast = &self.parsed.get().ast;

    let mut result = None;

    let mut visitor = AnyNodeVisitor::new(|node| {
      if node.span().contains_loc(location) {
        result = Some(node);
      }
    });

    visitor.visit_message(ast);

    result
  }
}
