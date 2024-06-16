use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;

use crate::util::Location;
use crate::util::Span;
use crate::util::Spanned;

#[derive(Debug)]
pub struct SimpleMessage<'a> {
  pub parts: Vec<MessagePart<'a>>,
}

pub enum MessagePart<'a> {
  Text(Text<'a>),
  Escape(Escape),
  Expression(Expression<'a>),
  Markup(Markup<'a>),
}

impl fmt::Debug for MessagePart<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      MessagePart::Text(text) => Debug::fmt(text, f),
      MessagePart::Escape(escape) => Debug::fmt(escape, f),
      MessagePart::Expression(expression) => Debug::fmt(expression, f),
      MessagePart::Markup(markup) => Debug::fmt(markup, f),
    }
  }
}

#[derive(Debug)]
pub struct Text<'a> {
  pub start: Location,
  pub content: &'a str,
}

impl Spanned for Text<'_> {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + self.content)
  }
}

#[derive(Debug)]
pub struct Escape {
  pub start: Location,
  pub escaped_char: char,
}

impl Spanned for Escape {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + '\\' + self.escaped_char)
  }
}

pub enum Expression<'a> {
  LiteralExpression(LiteralExpression<'a>),
  VariableExpression(VariableExpression<'a>),
  AnnotationExpression(AnnotationExpression<'a>),
}

impl fmt::Debug for Expression<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Expression::LiteralExpression(literal_expression) => {
        Debug::fmt(literal_expression, f)
      }
      Expression::VariableExpression(variable_expression) => {
        Debug::fmt(variable_expression, f)
      }
      Expression::AnnotationExpression(annotation_expression) => {
        Debug::fmt(annotation_expression, f)
      }
    }
  }
}

#[derive(Debug)]
pub struct LiteralExpression<'a> {
  pub literal: Literal<'a>,
  pub annotation: Option<Annotation<'a>>,
  pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug)]
pub struct VariableExpression<'a> {
  pub variable: Variable<'a>,
  pub annotation: Option<Annotation<'a>>,
  pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug)]
pub struct Variable<'a> {
  pub start: Location,
  pub name: &'a str,
}

impl Spanned for Variable<'_> {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + '$' + self.name)
  }
}

#[derive(Debug)]
pub struct AnnotationExpression<'a> {
  pub annotation: Annotation<'a>,
  pub attributes: Vec<Attribute<'a>>,
}

pub enum Annotation<'a> {
  Function(Function<'a>),
  PrivateUseAnnotation(PrivateUseAnnotation<'a>),
  ReservedAnnotation(ReservedAnnotation<'a>),
}

impl fmt::Debug for Annotation<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Annotation::Function(function) => Debug::fmt(function, f),
      Annotation::PrivateUseAnnotation(private_use_annotation) => {
        Debug::fmt(private_use_annotation, f)
      }
      Annotation::ReservedAnnotation(reserved_annotation) => {
        Debug::fmt(reserved_annotation, f)
      }
    }
  }
}

#[derive(Debug)]
pub struct Identifier<'a> {
  pub start: Location,
  pub namespace: Option<&'a str>,
  pub name: &'a str,
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

#[derive(Debug)]
pub struct Function<'a> {
  pub id: Identifier<'a>,
  pub options: Vec<FnOrMarkupOption<'a>>,
}

#[derive(Debug)]
pub struct FnOrMarkupOption<'a> {
  pub key: Identifier<'a>,
  pub value: LiteralOrVariable<'a>,
}

#[derive(Debug)]
pub struct Attribute<'a> {
  pub key: Identifier<'a>,
  pub value: Option<LiteralOrVariable<'a>>,
}

pub enum LiteralOrVariable<'a> {
  Literal(Literal<'a>),
  Variable(Variable<'a>),
}

impl fmt::Debug for LiteralOrVariable<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      LiteralOrVariable::Literal(literal) => Debug::fmt(literal, f),
      LiteralOrVariable::Variable(variable) => Debug::fmt(variable, f),
    }
  }
}

#[derive(Debug)]
pub struct PrivateUseAnnotation<'a> {
  pub start: char,
  pub body: Vec<ReservedBodyPart<'a>>,
}

#[derive(Debug)]
pub struct ReservedAnnotation<'a> {
  pub start: char,
  pub body: Vec<ReservedBodyPart<'a>>,
}

pub enum ReservedBodyPart<'a> {
  Text(Text<'a>),
  Escape(Escape),
  Quoted(Quoted<'a>),
}

impl fmt::Debug for ReservedBodyPart<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      ReservedBodyPart::Text(text) => Debug::fmt(text, f),
      ReservedBodyPart::Escape(escape) => Debug::fmt(escape, f),
      ReservedBodyPart::Quoted(quoted) => Debug::fmt(quoted, f),
    }
  }
}

pub enum Literal<'a> {
  Quoted(Quoted<'a>),
  Name(&'a str),
  Number(Number<'a>),
}

impl fmt::Debug for Literal<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Literal::Quoted(quoted) => Debug::fmt(quoted, f),
      Literal::Name(name) => Debug::fmt(name, f),
      Literal::Number(number) => Debug::fmt(number, f),
    }
  }
}

#[derive(Debug)]
pub struct Quoted<'a> {
  pub open: Location,
  pub close: Location,
  pub parts: Vec<QuotedPart<'a>>,
}

impl Spanned for Quoted<'_> {
  fn span(&self) -> Span {
    let start = self.open;
    let end = self.close + '|';
    Span::new(start..end)
  }
}

pub enum QuotedPart<'a> {
  Text(Text<'a>),
  Escape(Escape),
}

impl fmt::Debug for QuotedPart<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      QuotedPart::Text(text) => Debug::fmt(text, f),
      QuotedPart::Escape(escape) => Debug::fmt(escape, f),
    }
  }
}

#[derive(Debug)]
pub struct Number<'a> {
  pub raw: &'a str,
  pub is_negative: bool,
  pub integral_part: &'a str,
  pub fractional_part: Option<&'a str>,
  pub exponent_part: Option<(/* is_negative */ bool, &'a str)>,
}

#[derive(Debug)]
pub struct Markup<'a> {
  pub kind: MarkupKind,
  pub id: Identifier<'a>,
  pub options: Vec<FnOrMarkupOption<'a>>,
  pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug)]
pub enum MarkupKind {
  Open,
  Standalone,
  Close,
}
