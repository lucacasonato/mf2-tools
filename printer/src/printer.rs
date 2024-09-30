use mf2_parser::ast::*;

pub struct Printer<'ast, 'text> {
  ast: &'ast Message<'text>,
  out: String,
}

macro_rules! dispatch_print {
    (
      $self:expr, $expr:expr, $enum:ident {
        $( $member:ident => $fun:ident ),* $(,)?
      }
    ) => {
      match $expr {
        $( $enum::$member(x) => { Self::$fun($self, x) }, )*
      }
    };
}

impl<'ast, 'text> Printer<'ast, 'text> {
  pub fn new(ast: &'ast Message<'text>) -> Self {
    Self {
      ast,
      out: String::new(),
    }
  }

  pub fn print(mut self) -> String {
    self.print_message(self.ast);
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

  fn print_message(&mut self, message: &Message) {
    dispatch_print!(self, message, Message {
      Simple => print_pattern,
      Complex => print_complex_message,
    });
  }

  fn print_pattern(&mut self, pattern: &Pattern) {
    for part in &pattern.parts {
      dispatch_print!(self, part, PatternPart {
        Text => print_text,
        Escape => print_escape,
        Expression => print_expression,
        Markup => print_markup,
      });
    }
  }

  fn print_text(&mut self, text: &Text) {
    self.push_str(text.content);
  }

  fn print_escape(&mut self, escape: &Escape) {
    self.push('\\');
    self.push(escape.escaped_char);
  }

  fn print_expression(&mut self, expr: &Expression) {
    dispatch_print!(self, expr, Expression {
      AnnotationExpression => print_annotation_expression,
      LiteralExpression => print_literal_expression,
      VariableExpression => print_variable_expression,
    });
  }

  fn print_annotation_expression(&mut self, expr: &AnnotationExpression) {
    self.helper_print_expression(
      None::<()>,
      Some(&expr.annotation),
      &expr.attributes,
      |_, _| {},
    );
  }

  fn print_literal_expression(&mut self, expr: &LiteralExpression) {
    self.helper_print_expression(
      &expr.literal,
      expr.annotation.as_ref(),
      &expr.attributes,
      Self::print_literal,
    );
  }

  fn print_variable_expression(&mut self, expr: &VariableExpression) {
    self.helper_print_expression(
      &expr.variable,
      expr.annotation.as_ref(),
      &expr.attributes,
      Self::print_variable,
    );
  }

  fn helper_print_expression<T, F>(
    &mut self,
    body: T,
    annotation: Option<&Annotation>,
    attributes: &Vec<Attribute>,
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
      self.print_function(fun);
    }

    for attr in attributes {
      self.print_attribute(attr);
    }

    self.push(' ');
    self.push('}');
  }

  fn print_function(&mut self, fun: &Function) {
    self.push(':');
    self.print_identifier(&fun.id);

    for option in &fun.options {
      self.print_option(option);
    }
  }

  fn print_identifier(&mut self, id: &Identifier) {
    if let Some(namespace) = id.namespace {
      self.push_str(namespace);
      self.push(':');
    }
    self.push_str(id.name);
  }

  fn print_option(&mut self, option: &FnOrMarkupOption) {
    self.push(' ');
    self.print_identifier(&option.key);
    self.push('=');
    dispatch_print!(self, &option.value, LiteralOrVariable {
      Literal => print_literal,
      Variable => print_variable,
    })
  }

  fn print_literal(&mut self, lit: &Literal) {
    dispatch_print!(self, lit, Literal {
      Text => print_text,
      Quoted => print_quoted,
      Number => print_number,
    });
  }

  fn print_quoted(&mut self, quoted: &Quoted) {
    self.push('|');
    for part in &quoted.parts {
      dispatch_print!(self, part, QuotedPart {
        Text => print_text,
        Escape => print_escape,
      });
    }
    self.push('|');
  }

  fn print_number(&mut self, num: &Number) {
    self.push_str(num.raw);
  }

  fn print_variable(&mut self, var: &Variable) {
    self.push('$');
    self.push_str(var.name);
  }

  fn print_attribute(&mut self, attr: &Attribute) {
    self.push(' ');
    self.push('@');
    self.print_identifier(&attr.key);

    if let Some(value) = &attr.value {
      self.push('=');
      self.print_literal(value);
    }
  }

  fn print_markup(&mut self, markup: &Markup) {
    self.push('{');
    if let MarkupKind::Close = markup.kind {
      self.push('/');
    } else {
      self.push('#');
    }

    self.print_identifier(&markup.id);

    for option in &markup.options {
      self.print_option(option);
    }
    for attr in &markup.attributes {
      self.print_attribute(attr);
    }

    self.push(' ');
    if let MarkupKind::Standalone = markup.kind {
      self.push('/');
    }
    self.push('}');
  }

  fn print_complex_message(&mut self, message: &ComplexMessage) {
    let mut is_input = None;

    for decl in &message.declarations {
      let prev_is_input =
        is_input.replace(matches!(decl, Declaration::InputDeclaration(_)));
      if prev_is_input.is_some() && prev_is_input != is_input {
        self.push('\n');
      }

      dispatch_print!(self, decl, Declaration {
        InputDeclaration => print_input_declaration,
        LocalDeclaration => print_local_declaration,
      });
      self.push('\n');
    }

    if !is_input.is_none() {
      self.push('\n');
    }

    dispatch_print!(self, &message.body, ComplexMessageBody {
      QuotedPattern => print_quoted_pattern,
      Matcher => print_matcher,
    });

    self.push('\n');
  }

  fn print_input_declaration(&mut self, decl: &InputDeclaration) {
    self.push_str(".input ");
    self.print_variable_expression(&decl.expression);
  }

  fn print_local_declaration(&mut self, decl: &LocalDeclaration) {
    self.push_str(".local ");
    self.print_variable(&decl.variable);
    self.push_str(" = ");
    self.print_expression(&decl.expression);
  }

  fn print_quoted_pattern(&mut self, pattern: &QuotedPattern) {
    self.push_str("{{");
    self.print_pattern(&pattern.pattern);
    self.push_str("}}");
  }

  fn print_matcher(&mut self, matcher: &Matcher) {
    self.push_str(".match");

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
        let printed = self.try_print_match_key(key);
        max_lengths[i] = max_lengths[i].max(printed.len());
        printed_keys.push(printed);
      }
    }
    assert_eq!(printed_keys.len(), printed_keys.capacity());

    for (i, selector) in matcher.selectors.iter().enumerate() {
      self.push(' ');
      self.print_variable(selector);
      self.push_n(' ', max_lengths[i] - selector.name.len() - 1);
    }

    for (j, variant) in matcher.variants.iter().enumerate() {
      //            "\n.match "
      self.push_str("\n       ");

      for i in 0..selectors_count {
        let printed_key = &printed_keys[j * selectors_count + i];
        self.push_str(&printed_key);
        self.push_n(' ', max_lengths[i] - printed_key.len());
        self.push(' ');
      }

      self.print_quoted_pattern(&variant.pattern);
    }
  }

  fn try_print_match_key(&mut self, key: &Key) -> String {
    let Key::Literal(key) = key else {
      assert!(matches!(key, Key::Star(_)));
      return "*".to_string();
    };

    let backup = std::mem::replace(&mut self.out, String::new());

    self.print_literal(&key);

    return std::mem::replace(&mut self.out, backup);
  }
}
