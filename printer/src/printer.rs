use mf2_parser::ast::*;
use mf2_parser::Visit;
use mf2_parser::Visitable;

pub struct Printer<'ast, 'text> {
  ast: &'ast Message<'text>,
  out: String,
}

impl<'ast, 'text> Printer<'ast, 'text> {
  pub fn new(ast: &'ast Message<'text>) -> Self {
    Self {
      ast,
      out: String::new(),
    }
  }

  pub fn print(mut self) -> String {
    self.visit_message(self.ast);
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
      self.visit_function(fun);
    }

    for attr in attributes {
      self.visit_attribute(attr);
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

    self.visit_literal(key);

    std::mem::replace(&mut self.out, backup)
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
    self.visit_identifier(&option.key);
    self.push('=');
    self.visit_literal_or_variable(&option.value);
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
    self.visit_identifier(&attr.key);

    if let Some(value) = &attr.value {
      self.push('=');
      self.visit_literal(value);
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
    for decl in &message.declarations {
      self.visit_declaration(decl);
      self.push('\n');
    }

    if !message.declarations.is_empty() {
      self.push('\n');
    }

    self.visit_complex_message_body(&message.body);

    self.push('\n');
  }

  fn visit_input_declaration(&mut self, decl: &'ast InputDeclaration<'text>) {
    self.push_str(".input ");
    self.visit_variable_expression(&decl.expression);
  }

  fn visit_local_declaration(&mut self, decl: &'ast LocalDeclaration<'text>) {
    self.push_str(".local ");
    self.visit_variable(&decl.variable);
    self.push_str(" = ");
    self.visit_expression(&decl.expression);
  }

  fn visit_quoted_pattern(&mut self, pattern: &'ast QuotedPattern<'text>) {
    self.push_str("{{");
    self.visit_pattern(&pattern.pattern);
    self.push_str("}}");
  }

  fn visit_matcher(&mut self, matcher: &'ast Matcher<'text>) {
    self.push_str(".match\n");

    let selectors_count = matcher.selectors.len();
    let mut max_lengths = vec![0; selectors_count];

    for (i, selector) in matcher.selectors.iter().enumerate() {
      max_lengths[i] = selector.name.len() + 1;
    }

    let mut printed_keys =
      Vec::with_capacity(selectors_count * matcher.variants.len());

    for variant in &matcher.variants {
      assert_eq!(variant.keys.len(), selectors_count);

      for (i, key) in variant.keys.iter().enumerate() {
        let printed = self.try_visit_match_key(key);
        max_lengths[i] = max_lengths[i].max(printed.len());
        printed_keys.push(printed);
      }
    }
    assert_eq!(printed_keys.len(), printed_keys.capacity());

    for (i, selector) in matcher.selectors.iter().enumerate() {
      self.visit_variable(selector);
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

      self.visit_quoted_pattern(&variant.pattern);
    }
  }
}
