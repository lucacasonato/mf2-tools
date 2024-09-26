use std::fmt::Debug;

use crate::util::LengthShort;
use crate::util::Location;
use crate::util::Span;
use crate::util::Spanned;
use crate::visitor::Visit;
use crate::visitor::Visitable;

macro_rules! ast_enum {
  {
    #[visit($visit_method:ident)]
    pub enum $name:ident<$lifetime:lifetime> {
      $( $item:ident $(<$item_lifetime:lifetime>)? ),* $(,)?
    }
  } => {
    #[derive(Clone)]
    pub enum $name<$lifetime> {
      $( $item ( $item$(<$item_lifetime>)? ), )*
    }

    impl ::std::fmt::Debug for $name<'_> {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
          $( $name::$item(item) => ::std::fmt::Debug::fmt(item, f), )*
        }
      }
    }

    impl crate::util::Spanned for $name<'_> {
      fn span(&self) -> Span {
        match self {
          $( $name::$item(item) => item.span(), )*
        }
      }
    }

    impl<'text> crate::visitor::Visitable<'text> for $name<'text> {
      fn apply_visitor<'ast, V: crate::visitor::Visit<'ast, 'text> + ?Sized>(&'ast self, visitor: &mut V) {
        visitor.$visit_method(self);
      }

      fn apply_visitor_to_children<'ast, V: crate::visitor::Visit<'ast, 'text> + ?Sized>(&'ast self, visitor: &mut V) {
        match self {
          $( $name::$item(item) => item.apply_visitor(visitor), )*
        }
      }
    }
  };
}

#[derive(Clone)]
pub enum Message<'text> {
  Simple(Pattern<'text>),
  Complex(ComplexMessage<'text>),
}

impl Debug for Message<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Message::Simple(pattern) => Debug::fmt(pattern, f),
      Message::Complex(complex) => Debug::fmt(complex, f),
    }
  }
}

impl Spanned for Message<'_> {
  fn span(&self) -> Span {
    match self {
      Message::Simple(pattern) => pattern.span(),
      Message::Complex(complex) => complex.span(),
    }
  }
}

impl<'text> Visitable<'text> for Message<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    match self {
      Message::Simple(pattern) => pattern.apply_visitor(visitor),
      Message::Complex(complex) => complex.apply_visitor(visitor),
    }
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    match self {
      Message::Simple(pattern) => pattern.apply_visitor_to_children(visitor),
      Message::Complex(complex) => complex.apply_visitor_to_children(visitor),
    }
  }
}

#[derive(Debug, Clone)]
pub struct Pattern<'text> {
  pub parts: Vec<PatternPart<'text>>,
}

impl Spanned for Pattern<'_> {
  fn span(&self) -> Span {
    match (self.parts.first(), self.parts.last()) {
      (Some(first), Some(last)) => {
        Span::new(first.span().start..last.span().end)
      }
      _ => Span::new(Location::dummy()..Location::dummy()),
    }
  }
}

impl<'text> Visitable<'text> for Pattern<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_pattern(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    for part in &self.parts {
      part.apply_visitor(visitor);
    }
  }
}

ast_enum! {
  #[visit(visit_pattern_part)]
  pub enum PatternPart<'text> {
    Text<'text>,
    Escape,
    Expression<'text>,
    Markup<'text>,
  }
}

#[derive(Debug, Clone)]
pub struct Text<'text> {
  pub start: Location,
  pub content: &'text str,
}

impl Spanned for Text<'_> {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + self.content)
  }
}

impl<'text> Visitable<'text> for Text<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_text(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    _visitor: &mut V,
  ) {
  }
}

#[derive(Debug, Clone)]
pub struct Escape {
  pub start: Location,
  pub escaped_char: char,
}

impl Spanned for Escape {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + '\\' + self.escaped_char)
  }
}

impl<'text> Visitable<'text> for Escape {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_escape(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    _visitor: &mut V,
  ) {
  }
}

ast_enum! {
  #[visit(visit_expression)]
  pub enum Expression<'text> {
    LiteralExpression<'text>,
    VariableExpression<'text>,
    AnnotationExpression<'text>,
  }
}

#[derive(Debug, Clone)]
pub struct LiteralExpression<'text> {
  pub span: Span,
  pub literal: Literal<'text>,
  pub annotation: Option<Annotation<'text>>,
  pub attributes: Vec<Attribute<'text>>,
}

impl Spanned for LiteralExpression<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for LiteralExpression<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_literal_expression(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.literal.apply_visitor(visitor);
    if let Some(annotation) = &self.annotation {
      annotation.apply_visitor(visitor);
    }
    for attribute in &self.attributes {
      attribute.apply_visitor(visitor);
    }
  }
}

#[derive(Debug, Clone)]
pub struct VariableExpression<'text> {
  pub span: Span,
  pub variable: Variable<'text>,
  pub annotation: Option<Annotation<'text>>,
  pub attributes: Vec<Attribute<'text>>,
}

impl Spanned for VariableExpression<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for VariableExpression<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_variable_expression(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.variable.apply_visitor(visitor);
    if let Some(annotation) = &self.annotation {
      annotation.apply_visitor(visitor);
    }
    for attribute in &self.attributes {
      attribute.apply_visitor(visitor);
    }
  }
}

#[derive(Debug, Clone)]
pub struct Variable<'text> {
  pub span: Span,
  pub name: &'text str,
}

impl Spanned for Variable<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for Variable<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_variable(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    _visitor: &mut V,
  ) {
  }
}

#[derive(Debug, Clone)]
pub struct AnnotationExpression<'text> {
  pub span: Span,
  pub annotation: Annotation<'text>,
  pub attributes: Vec<Attribute<'text>>,
}

impl Spanned for AnnotationExpression<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for AnnotationExpression<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_annotation_expression(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.annotation.apply_visitor(visitor);
    for attribute in &self.attributes {
      attribute.apply_visitor(visitor);
    }
  }
}

ast_enum! {
  #[visit(visit_annotation)]
  pub enum Annotation<'text> {
    Function<'text>,
  }
}

#[derive(Debug, Clone)]
pub struct Identifier<'text> {
  pub start: Location,
  pub namespace: Option<&'text str>,
  pub name: &'text str,
}

impl Spanned for Identifier<'_> {
  fn span(&self) -> Span {
    let mut end = self.start;
    if let Some(namespace) = self.namespace {
      end = end + namespace + ':';
    }
    end = end + self.name;

    Span::new(self.start..end)
  }
}

impl<'text> Visitable<'text> for Identifier<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_identifier(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    _visitor: &mut V,
  ) {
  }
}

#[derive(Debug, Clone)]
pub struct Function<'text> {
  pub start: Location,
  pub id: Identifier<'text>,
  pub options: Vec<FnOrMarkupOption<'text>>,
}

impl Spanned for Function<'_> {
  fn span(&self) -> Span {
    let start = self.start;
    let end = self
      .options
      .last()
      .map_or(self.id.span().end, |last| last.span().end);
    Span::new(start..end)
  }
}

impl<'text> Visitable<'text> for Function<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_function(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.id.apply_visitor(visitor);
    for option in &self.options {
      option.apply_visitor(visitor);
    }
  }
}

#[derive(Debug, Clone)]
pub struct FnOrMarkupOption<'text> {
  pub key: Identifier<'text>,
  pub value: LiteralOrVariable<'text>,
}

impl Spanned for FnOrMarkupOption<'_> {
  fn span(&self) -> Span {
    let start = self.key.span().start;
    let end = self.value.span().end;
    Span::new(start..end)
  }
}

impl<'text> Visitable<'text> for FnOrMarkupOption<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_fn_or_markup_option(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.key.apply_visitor(visitor);
    self.value.apply_visitor(visitor);
  }
}

#[derive(Debug, Clone)]
pub struct Attribute<'text> {
  pub span: Span,
  pub key: Identifier<'text>,
  pub value: Option<Literal<'text>>,
}

impl Spanned for Attribute<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for Attribute<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_attribute(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.key.apply_visitor(visitor);
    if let Some(value) = &self.value {
      value.apply_visitor(visitor);
    }
  }
}

ast_enum! {
  #[visit(visit_literal_or_variable)]
  pub enum LiteralOrVariable<'text> {
    Literal<'text>,
    Variable<'text>,
  }
}

ast_enum! {
  #[visit(visit_literal)]
  pub enum Literal<'text> {
    Quoted<'text>,
    Text<'text>,
    Number<'text>,
  }
}

#[derive(Debug, Clone)]
pub struct Quoted<'text> {
  pub span: Span,
  pub parts: Vec<QuotedPart<'text>>,
}

impl Spanned for Quoted<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for Quoted<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_quoted(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    for part in &self.parts {
      part.apply_visitor(visitor);
    }
  }
}

ast_enum! {
  #[visit(visit_quoted_part)]
  pub enum QuotedPart<'text> {
    Text<'text>,
    Escape,
  }
}

#[derive(Debug, Clone, Copy)]
pub enum ExponentSign {
  Plus,
  Minus,
  None,
}

#[derive(Debug, Clone)]
pub struct Number<'text> {
  pub start: Location,
  pub raw: &'text str,
  pub is_negative: bool,
  pub integral_len: LengthShort,
  pub fractional_len: Option<LengthShort>,
  pub exponent_len: Option<(ExponentSign, LengthShort)>,
}

impl Spanned for Number<'_> {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + self.raw)
  }
}

impl<'text> Visitable<'text> for Number<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_number(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    _visitor: &mut V,
  ) {
  }
}

impl<'text> Number<'text> {
  fn slice(&self, span: Span) -> &'text str {
    &self.raw[span.start.inner() as usize..span.end.inner() as usize]
  }

  fn integral_start(&self) -> Location {
    if self.is_negative {
      self.start + '-'
    } else {
      self.start
    }
  }

  fn integral_end(&self) -> Location {
    self.integral_start() + self.integral_len
  }

  pub fn integral_span(&self) -> Span {
    Span::new(self.integral_start()..self.integral_end())
  }

  pub fn integral_part(&self) -> &'text str {
    self.slice(self.integral_span())
  }

  pub fn fractional_span(&self) -> Option<Span> {
    self.fractional_len.map(|fractional_len| {
      let start = self.integral_end() + '.';
      let end = start + fractional_len;
      Span::new(start..end)
    })
  }

  pub fn fractional_part(&self) -> Option<&'text str> {
    self.fractional_span().map(|span| self.slice(span))
  }

  pub fn exponent_span(&self) -> Option<Span> {
    self.exponent_len.map(|(sign, exponent_len)| {
      let mut start = self.integral_end();
      if let Some(fractional_len) = &self.fractional_len {
        start = start + '.';
        start = start + *fractional_len;
      }

      start = start + 'e';

      if !matches!(sign, ExponentSign::None) {
        start = start + '-';
      };

      let end = start + exponent_len;

      Span::new(start..end)
    })
  }

  pub fn exponent_part(&self) -> Option<(ExponentSign, &'text str)> {
    self
      .exponent_span()
      .map(|span| (self.exponent_len.as_ref().unwrap().0, self.slice(span)))
  }
}

#[derive(Debug, Clone)]
pub struct Markup<'text> {
  pub span: Span,
  pub kind: MarkupKind,
  pub id: Identifier<'text>,
  pub options: Vec<FnOrMarkupOption<'text>>,
  pub attributes: Vec<Attribute<'text>>,
}

#[derive(Debug, Clone)]
pub enum MarkupKind {
  Open,
  Standalone,
  Close,
}

impl Spanned for Markup<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for Markup<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_markup(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.id.apply_visitor(visitor);
    for option in &self.options {
      option.apply_visitor(visitor);
    }
    for attribute in &self.attributes {
      attribute.apply_visitor(visitor);
    }
  }
}

#[derive(Debug, Clone)]
pub struct ComplexMessage<'text> {
  pub span: Span,
  pub declarations: Vec<Declaration<'text>>,
  pub body: ComplexMessageBody<'text>,
}

impl Spanned for ComplexMessage<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for ComplexMessage<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_complex_message(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    for declaration in &self.declarations {
      declaration.apply_visitor(visitor);
    }
    self.body.apply_visitor(visitor);
  }
}

ast_enum! {
  #[visit(visit_declaration)]
  pub enum Declaration<'text> {
    InputDeclaration<'text>,
    LocalDeclaration<'text>,
  }
}

#[derive(Debug, Clone)]
pub struct InputDeclaration<'text> {
  pub start: Location,
  pub expression: VariableExpression<'text>,
}

impl Spanned for InputDeclaration<'_> {
  fn span(&self) -> Span {
    let start = self.start;
    let end = self.expression.span().end;
    Span::new(start..end)
  }
}

impl<'text> Visitable<'text> for InputDeclaration<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_input_declaration(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.expression.apply_visitor(visitor);
  }
}

#[derive(Debug, Clone)]
pub struct LocalDeclaration<'text> {
  pub start: Location,
  pub variable: Variable<'text>,
  pub expression: Expression<'text>,
}

impl Spanned for LocalDeclaration<'_> {
  fn span(&self) -> Span {
    let start = self.start;
    let end = self.expression.span().end;
    Span::new(start..end)
  }
}

impl<'text> Visitable<'text> for LocalDeclaration<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_local_declaration(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.variable.apply_visitor(visitor);
    self.expression.apply_visitor(visitor);
  }
}

ast_enum! {
  #[visit(visit_complex_message_body)]
  pub enum ComplexMessageBody<'text> {
    QuotedPattern<'text>,
    Matcher<'text>,
  }
}

#[derive(Debug, Clone)]
pub struct QuotedPattern<'text> {
  pub span: Span,
  pub pattern: Pattern<'text>,
}

impl Spanned for QuotedPattern<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

impl<'text> Visitable<'text> for QuotedPattern<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_quoted_pattern(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    self.pattern.apply_visitor(visitor);
  }
}

#[derive(Debug, Clone)]
pub struct Matcher<'text> {
  pub start: Location,
  pub selectors: Vec<Variable<'text>>,
  pub variants: Vec<Variant<'text>>,
}

impl Spanned for Matcher<'_> {
  fn span(&self) -> Span {
    let start = self.start;
    let end = self
      .variants
      .last()
      .map(|last| last.span().end)
      .unwrap_or_else(|| {
        self
          .selectors
          .last()
          .map(|last| last.span().end)
          .unwrap_or_else(|| start + ".match")
      });
    Span::new(start..end)
  }
}

impl<'text> Visitable<'text> for Matcher<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_matcher(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    for selector in &self.selectors {
      selector.apply_visitor(visitor);
    }
    for variant in &self.variants {
      variant.apply_visitor(visitor);
    }
  }
}

#[derive(Debug, Clone)]
pub struct Variant<'text> {
  pub keys: Vec<Key<'text>>,
  pub pattern: QuotedPattern<'text>,
}

impl Spanned for Variant<'_> {
  fn span(&self) -> Span {
    let start = self
      .keys
      .first()
      .map(|first| first.span().start)
      .unwrap_or_else(|| self.pattern.span().start);
    let end = self.pattern.span().end;
    Span::new(start..end)
  }
}

impl<'text> Visitable<'text> for Variant<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_variant(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    for key in &self.keys {
      key.apply_visitor(visitor);
    }
    self.pattern.apply_visitor(visitor);
  }
}

ast_enum! {
  #[visit(visit_key)]
  pub enum Key<'text> {
    Literal<'text>,
    Star,
  }
}

#[derive(Debug, Clone)]
pub struct Star {
  pub start: Location,
}

impl Spanned for Star {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + '*')
  }
}

impl<'text> Visitable<'text> for Star {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  ) {
    visitor.visit_star(self);
  }

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    _visitor: &mut V,
  ) {
  }
}

macro_rules! any_node {
    (
      pub enum $name:ident<$ast_lifetime:lifetime, $text_lifetime:lifetime> {
        $( $item:ident $(<$item_lifetime:lifetime>)? ),* $(,)?
      }
    ) => {
      #[derive(Clone)]
      pub enum $name<$ast_lifetime, $text_lifetime> {
        $( $item ( &'ast $item$(<$item_lifetime>)? ), )*
      }

      impl ::std::fmt::Debug for $name<'_, '_> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
          match self {
            $( $name::$item(item) => ::std::fmt::Debug::fmt(item, f), )*
          }
        }
      }

      impl crate::util::Spanned for $name<'_, '_> {
        fn span(&self) -> Span {
          match self {
            $( $name::$item(item) => item.span(), )*
          }
        }
      }

      impl<'ast, 'text> $name<'ast, 'text> {
        pub fn apply_visitor<V: crate::visitor::Visit<'ast, 'text> + ?Sized>(&self, visitor: &mut V) {
          match self {
            $( $name::$item(item) => item.apply_visitor(visitor), )*
          }
        }
      }
    };
}

any_node! {
  pub enum AnyNode<'ast, 'text> {
    Message<'text>,
    Pattern<'text>,
    PatternPart<'text>,
    Text<'text>,
    Escape,
    Expression<'text>,
    LiteralExpression<'text>,
    VariableExpression<'text>,
    Variable<'text>,
    AnnotationExpression<'text>,
    Annotation<'text>,
    Function<'text>,
    FnOrMarkupOption<'text>,
    Attribute<'text>,
    LiteralOrVariable<'text>,
    Quoted<'text>,
    QuotedPart<'text>,
    Literal<'text>,
    Number<'text>,
    Markup<'text>,
    Identifier<'text>,
    ComplexMessage<'text>,
    Declaration<'text>,
    InputDeclaration<'text>,
    LocalDeclaration<'text>,
    ComplexMessageBody<'text>,
    QuotedPattern<'text>,
    Matcher<'text>,
    Variant<'text>,
    Key<'text>,
    Star,
  }
}
