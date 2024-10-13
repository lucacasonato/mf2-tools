use mf2_parser::ast::*;
use mf2_parser::LineColUtf8;
use mf2_parser::Location;
use mf2_parser::SourceTextInfo;
use mf2_parser::Spanned;
use mf2_parser::Visit;
use mf2_parser::Visitable;

pub struct Printer<'ast, 'text> {
  ast: &'ast Message<'text>,
  info: Option<&'text SourceTextInfo<'text>>,
  out: String,
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

  fn helper_visit_expression<T, F>(
    &mut self,
    body: T,
    annotation: Option<&'ast Annotation<'text>>,
    attributes: &'ast Vec<Attribute<'text>>,
    cb: F,
  ) where
    F: FnOnce(&mut Self, T),
  {
    self.push('{');
    self.push(' ');

    cb(self, body);

    if let Some(annotation) = annotation {
      if !matches!(self.out.chars().last(), Some(' ')) {
        self.push(' ');
      }

      let Annotation::Function(fun) = annotation;
      fun.apply_visitor(self);
    }

    for attr in attributes {
      attr.apply_visitor(self);
    }

    self.push(' ');
    self.push('}');
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

  fn visit_function(&mut self, fun: &'ast Function<'text>) {
    self.push(':');
    fun.apply_visitor_to_children(self);
  }

  fn visit_identifier(&mut self, id: &Identifier) {
    if let Some(namespace) = id.namespace {
      self.push_str(namespace);
      self.push(':');
    }
    self.push_str(id.name);
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
    self.push('|');
    quoted.apply_visitor_to_children(self);
    self.push('|');
  }

  fn visit_number(&mut self, num: &Number) {
    self.push_str(num.raw);
  }

  fn visit_variable(&mut self, var: &Variable) {
    self.push('$');
    self.push_str(var.name);
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
    self.push('{');
    if let MarkupKind::Close = markup.kind {
      self.push('/');
    } else {
      self.push('#');
    }

    markup.apply_visitor_to_children(self);

    self.push(' ');
    if let MarkupKind::Standalone = markup.kind {
      self.push('/');
    }
    self.push('}');
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
    self.push_str("{{");
    pattern.pattern.apply_visitor(self);
    self.push_str("}}");
  }

  fn visit_matcher(&mut self, matcher: &'ast Matcher<'text>) {
    self.push_str(".match\n");

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
      self.push('\n');

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
