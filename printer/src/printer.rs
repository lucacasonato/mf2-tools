use std::borrow::Cow;

use mf2_parser::ast::Literal;
use mf2_parser::ast::*;
use mf2_parser::LineColUtf8;
use mf2_parser::Location;
use mf2_parser::SourceTextInfo;
use mf2_parser::Span;
use mf2_parser::Spanned;
use mf2_parser::Visit;
use mf2_parser::Visitable;
use unicode_bidi::format_chars::FSI;
use unicode_bidi::format_chars::LRI;
use unicode_bidi::format_chars::PDI;

pub struct Printer<'ast, 'text> {
  ast: &'ast Message<'text>,
  info: Option<&'text SourceTextInfo<'text>>,
  out: String,
  text_last_isolated: usize,
}

impl<'ast, 'text> Printer<'ast, 'text> {
  pub fn new(
    ast: &'ast Message<'text>,
    info: Option<&'text SourceTextInfo<'text>>,
  ) -> Self {
    Self {
      ast,
      info,
      out: String::new(),
      text_last_isolated: 0,
    }
  }

  pub fn print(mut self) -> String {
    self.ast.apply_visitor(&mut self);
    self.out
  }

  fn push(&mut self, ch: char) {
    self.out.push(ch);
  }

  fn push_n(&mut self, ch: char, count: usize) {
    for _ in 0..count {
      self.push(ch);
    }
  }

  fn push_str(&mut self, str: &str) {
    self.out.push_str(str);
  }

  fn push_isolated_str(&mut self, str: &str) {
    let has_rtl = has_rtl(str);
    if has_rtl {
      self.push(FSI);
    }
    self.push_str(str);
    if has_rtl {
      self.push(PDI);
    }
  }

  fn helper_visit_expression<T, F>(
    &mut self,
    body: T,
    annotation: Option<&'ast Annotation<'text>>,
    attributes: &'ast Vec<Attribute<'text>>,
    cb: F,
  ) where
    F: FnOnce(&mut Self, T),
  {
    let needs_isolation = has_rtl(&self.out[self.text_last_isolated..]);

    self.push('{');
    if needs_isolation {
      self.push(LRI);
    }

    cb(self, body);

    if let Some(annotation) = annotation {
      if !matches!(self.out.chars().last(), Some('{' | LRI)) {
        self.push(' ');
      }

      annotation.apply_visitor(self);
    }

    for attr in attributes {
      attr.apply_visitor(self);
    }

    if needs_isolation {
      self.push(PDI);
    }
    self.push('}');
    self.text_last_isolated = self.out.len();
  }

  fn try_visit_match_key(&mut self, key: &'ast Key<'text>) -> String {
    let Key::Literal(key) = key else {
      assert!(matches!(key, Key::Star(_)));
      return "*".to_string();
    };

    let backup = std::mem::take(&mut self.out);

    key.apply_visitor(self);

    std::mem::replace(&mut self.out, backup)
  }

  fn had_empty_line(
    &self,
    start: Location,
    end: Location,
    default: bool,
  ) -> bool {
    let Some(info) = self.info else {
      return default;
    };

    let LineColUtf8 {
      line: start_line, ..
    } = info.utf8_line_col(start);
    let LineColUtf8 { line: end_line, .. } = info.utf8_line_col(end);

    end_line > start_line + 1
  }
}

impl<'ast, 'text> Visit<'ast, 'text> for Printer<'ast, 'text> {
  fn visit_text(&mut self, text: &Text) {
    self.push_str(text.content);
  }

  fn visit_escape(&mut self, escape: &Escape) {
    self.push('\\');
    self.push(escape.escaped_char);
  }

  fn visit_annotation_expression(
    &mut self,
    expr: &'ast AnnotationExpression<'text>,
  ) {
    self.helper_visit_expression(
      None::<()>,
      Some(&expr.annotation),
      &expr.attributes,
      |_, _| {},
    );
  }

  fn visit_literal_expression(&mut self, expr: &'ast LiteralExpression<'text>) {
    self.helper_visit_expression(
      &expr.literal,
      expr.annotation.as_ref(),
      &expr.attributes,
      Self::visit_literal,
    );
  }

  fn visit_variable_expression(
    &mut self,
    expr: &'ast VariableExpression<'text>,
  ) {
    self.helper_visit_expression(
      &expr.variable,
      expr.annotation.as_ref(),
      &expr.attributes,
      Self::visit_variable,
    );
  }

  fn visit_annotation(&mut self, ann: &'ast Annotation<'text>) {
    self.push(':');
    ann.apply_visitor_to_children(self);
  }

  fn visit_identifier(&mut self, id: &Identifier) {
    if let Some(namespace) = id.namespace {
      self.push_isolated_str(namespace);
      self.push(':');
    }
    self.push_isolated_str(id.name);
  }

  fn visit_fn_or_markup_option(
    &mut self,
    option: &'ast FnOrMarkupOption<'text>,
  ) {
    self.push(' ');
    option.key.apply_visitor(self);
    self.push('=');
    option.value.apply_visitor(self);
  }

  fn visit_quoted(&mut self, quoted: &'ast Quoted<'text>) {
    let has_rtl = quoted_literal_has_rtl(quoted, self.info);
    if has_rtl {
      self.push(FSI);
    }
    self.push('|');
    quoted.apply_visitor_to_children(self);
    self.push('|');
    if has_rtl {
      self.push(PDI);
    }
  }

  fn visit_number(&mut self, num: &Number) {
    self.push_str(num.raw);
  }

  fn visit_literal(&mut self, literal: &'ast Literal<'text>) {
    match literal {
      Literal::Text(text) => self.push_isolated_str(text.content),
      _ => literal.apply_visitor_to_children(self),
    }
  }

  fn visit_variable(&mut self, var: &Variable) {
    self.push('$');
    self.push_isolated_str(var.name);
  }

  fn visit_attribute(&mut self, attr: &'ast Attribute<'text>) {
    self.push(' ');
    self.push('@');
    attr.key.apply_visitor(self);

    if let Some(value) = &attr.value {
      self.push('=');
      value.apply_visitor(self);
    }
  }

  fn visit_markup(&mut self, markup: &'ast Markup<'text>) {
    let needs_isolation = has_rtl(&self.out[self.text_last_isolated..]);

    self.push('{');
    if let MarkupKind::Close = markup.kind {
      self.push('/');
    } else {
      self.push('#');
    }
    if needs_isolation {
      self.push(LRI);
    }

    markup.apply_visitor_to_children(self);

    if needs_isolation {
      self.push(PDI);
    }
    if let MarkupKind::Standalone = markup.kind {
      self.push(' ');
      self.push('/');
    }
    self.push('}');
    self.text_last_isolated = self.out.len();
  }

  fn visit_complex_message(&mut self, message: &'ast ComplexMessage<'text>) {
    for (i, decl) in message.declarations.iter().enumerate() {
      decl.apply_visitor(self);
      self.push('\n');

      let next_decl =
        message.declarations.get(i + 1).map(|x| x as &dyn Spanned);
      let next_start = next_decl
        .unwrap_or(&message.body as &dyn Spanned)
        .span()
        .start;

      if self.had_empty_line(decl.span().end, next_start, next_decl.is_none()) {
        self.push('\n');
      }
    }

    message.body.apply_visitor(self);

    self.push('\n');
  }

  fn visit_input_declaration(&mut self, decl: &'ast InputDeclaration<'text>) {
    self.push_str(".input ");
    decl.expression.apply_visitor(self);
  }

  fn visit_local_declaration(&mut self, decl: &'ast LocalDeclaration<'text>) {
    self.push_str(".local ");
    decl.variable.apply_visitor(self);
    self.push_str(" = ");
    decl.expression.apply_visitor(self);
  }

  fn visit_quoted_pattern(&mut self, pattern: &'ast QuotedPattern<'text>) {
    let has_rtl = pattern_has_rtl(&pattern.pattern, self.info);
    if has_rtl {
      self.push(FSI);
    }
    self.push_str("{{");
    pattern.pattern.apply_visitor(self);
    self.push_str("}}");
    if has_rtl {
      self.push(PDI);
    }
  }

  fn visit_matcher(&mut self, matcher: &'ast Matcher<'text>) {
    self.push_str(".match");

    let selectors_count = matcher
      .variants
      .iter()
      .map(|v| v.keys.len())
      .chain(std::iter::once(matcher.selectors.len()))
      .max()
      .expect("There is at least matcher.selectors.len()");
    let mut max_lengths = vec![0; selectors_count];

    assert!(matcher.selectors.len() <= selectors_count);
    for (i, selector) in matcher.selectors.iter().enumerate() {
      max_lengths[i] = selector.name.len() + 1;
    }

    if max_lengths.len() > 1 {
      self.push_str("\n  ");
    } else {
      self.push(' ');
    }

    let mut printed_keys =
      Vec::with_capacity(selectors_count * matcher.variants.len());

    for variant in &matcher.variants {
      assert!(variant.keys.len() <= selectors_count);

      for (i, key) in variant.keys.iter().enumerate() {
        let printed = self.try_visit_match_key(key);
        max_lengths[i] = max_lengths[i].max(printed.len());
        printed_keys.push(printed);
      }
      for _ in variant.keys.len()..selectors_count {
        printed_keys.push("".to_string());
      }
    }
    assert_eq!(printed_keys.len(), printed_keys.capacity());

    for (i, selector) in matcher.selectors.iter().enumerate() {
      selector.apply_visitor(self);
      if i < selectors_count - 1 {
        self.push_n(' ', max_lengths[i] - selector.name.len());
      }
    }

    for (j, variant) in matcher.variants.iter().enumerate() {
      self.push_str("\n  ");

      for i in 0..selectors_count {
        let printed_key = &printed_keys[j * selectors_count + i];
        self.push_str(printed_key);
        self.push_n(' ', max_lengths[i] - printed_key.len());
        self.push(' ');
      }

      variant.pattern.apply_visitor(self);
    }
  }
}

fn quoted_literal_has_rtl(
  quoted: &Quoted,
  info: Option<&SourceTextInfo>,
) -> bool {
  let contigous_text = match info {
    Some(info) => Cow::Borrowed(info.text(quoted.span())),
    None => {
      let mut str = String::new();
      for child in &quoted.parts {
        match child {
          QuotedPart::Text(text) => str.push_str(text.content),
          QuotedPart::Escape(escape) => {
            str.push('\\');
            str.push(escape.escaped_char);
          }
        }
      }
      Cow::Owned(str)
    }
  };
  has_rtl(&contigous_text)
}

fn pattern_has_rtl(pattern: &Pattern, info: Option<&SourceTextInfo>) -> bool {
  if let Some(info) = info {
    let mut contigious_span: Option<Span> = None;
    for part in &pattern.parts {
      match part {
        PatternPart::Text(text) => {
          if let Some(span) = contigious_span {
            contigious_span = Some(Span::new(span.start..text.span().end));
          } else {
            contigious_span = Some(text.span());
          }
        }
        PatternPart::Escape(escape) => {
          if let Some(span) = contigious_span {
            contigious_span = Some(Span::new(span.start..escape.span().end));
          } else {
            contigious_span = Some(escape.span());
          }
        }
        PatternPart::Expression(_) | PatternPart::Markup(_) => {
          if let Some(span) = contigious_span {
            if has_rtl(info.text(span)) {
              return true;
            }
          }
          contigious_span = None;
        }
      }
    }
    if let Some(span) = contigious_span {
      has_rtl(info.text(span))
    } else {
      false
    }
  } else {
    let mut contigous_text = String::new();
    for part in &pattern.parts {
      match part {
        PatternPart::Text(text) => contigous_text.push_str(text.content),
        PatternPart::Escape(escape) => {
          contigous_text.push('\\');
          contigous_text.push(escape.escaped_char);
        }
        PatternPart::Expression(_) | PatternPart::Markup(_) => {
          if has_rtl(&contigous_text) {
            return true;
          }
          contigous_text.clear();
        }
      }
    }
    has_rtl(&contigous_text)
  }
}

fn has_rtl(text: &str) -> bool {
  unicode_bidi::BidiInfo::new(text, None).has_rtl()
}
