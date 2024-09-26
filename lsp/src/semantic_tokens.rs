use lsp_types::Position;
use lsp_types::SemanticToken;
use lsp_types::SemanticTokenType;
use lsp_types::SemanticTokensLegend;
use mf2_parser::ast;
use mf2_parser::Span;
use mf2_parser::Spanned;
use mf2_parser::Visit;
use mf2_parser::Visitable as _;

use crate::document::Document;

pub fn legend() -> SemanticTokensLegend {
  SemanticTokensLegend {
    token_types: vec![
      SemanticTokenType::VARIABLE,
      SemanticTokenType::PARAMETER,
      SemanticTokenType::FUNCTION,
      SemanticTokenType::KEYWORD,
      SemanticTokenType::STRING,
      SemanticTokenType::NUMBER,
    ],
    token_modifiers: vec![],
  }
}

pub struct SemanticTokenVisitor<'a> {
  pub document: &'a Document,
  pub tokens: Vec<SemanticToken>,
  pub last_start: Position,
}

impl SemanticTokenVisitor<'_> {
  fn report_token(&mut self, span: Span, token_type: u32) {
    let mut start = self.document.loc_to_pos(span.start);
    let end = self.document.loc_to_pos(span.end);

    while start.line <= end.line {
      let token = SemanticToken {
        delta_line: start.line - self.last_start.line,
        delta_start: if start.line == self.last_start.line {
          start.character - self.last_start.character
        } else {
          start.character
        },
        length: if start.line == end.line {
          end.character - start.character
        } else {
          let start_loc = self.document.pos_to_loc(start);
          let end_loc = self.document.pos_to_loc(Position {
            line: start.line + 1,
            character: 0,
          });
          self.document.span_len(Span::new(start_loc..end_loc))
        },
        token_type,
        token_modifiers_bitset: 0,
      };

      self.tokens.push(token);

      self.last_start = start;

      start.line += 1;
      start.character = 0;
    }
  }
}

impl<'ast, 'text> Visit<'ast, 'text> for SemanticTokenVisitor<'ast> {
  fn visit_function(&mut self, func: &'ast ast::Function<'text>) {
    self.report_token(func.id.span(), 2 /* function */);
    func.apply_visitor_to_children(self);
  }

  fn visit_variable(&mut self, var: &'ast ast::Variable<'text>) {
    self.report_token(var.span(), 0 /* variable */);
    var.apply_visitor_to_children(self);
  }

  fn visit_literal(&mut self, literal: &'ast ast::Literal<'text>) {
    match literal {
      ast::Literal::Text(s) => {
        self.report_token(s.span(), 4 /* string */)
      }
      ast::Literal::Number(n) => {
        self.report_token(n.span(), 5 /* number */)
      }
      ast::Literal::Quoted(n) => {
        self.report_token(n.span(), 4 /* string */)
      }
    }
  }

  fn visit_matcher(&mut self, matcher: &'ast ast::Matcher<'text>) {
    self.report_token(
      Span::new(matcher.start..matcher.start + ".match"),
      3, /* keyword */
    );
    matcher.apply_visitor_to_children(self);
  }

  fn visit_fn_or_markup_option(
    &mut self,
    opt: &'ast ast::FnOrMarkupOption<'text>,
  ) {
    self.report_token(opt.key.span(), 1 /* parameter */);
    opt.apply_visitor_to_children(self);
  }
}
