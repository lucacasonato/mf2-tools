use std::ops::Range;

use crate::ast::Annotation;
use crate::ast::AnnotationExpression;
use crate::ast::Attribute;
use crate::ast::Escape;
use crate::ast::ExponentSign;
use crate::ast::Expression;
use crate::ast::FnOrMarkupOption;
use crate::ast::Function;
use crate::ast::Identifier;
use crate::ast::Literal;
use crate::ast::LiteralExpression;
use crate::ast::LiteralOrVariable;
use crate::ast::Markup;
use crate::ast::MarkupKind;
use crate::ast::MessagePart;
use crate::ast::Number;
use crate::ast::PrivateUseAnnotation;
use crate::ast::Quoted;
use crate::ast::QuotedPart;
use crate::ast::ReservedAnnotation;
use crate::ast::ReservedBodyPart;
use crate::ast::SimpleMessage;
use crate::ast::Text;
use crate::ast::Variable;
use crate::ast::VariableExpression;
use crate::util::LengthShort;
use crate::util::Location;
use crate::util::SourceTextIterator;

pub struct Parser<'a> {
  text: SourceTextIterator<'a>,
}

impl<'a> Parser<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      text: SourceTextIterator::new(input),
    }
  }

  pub fn parse(mut self) -> SimpleMessage<'a> {
    while let Some((_, c)) = self.peek() {
      if is_simple_start(c) {
        return self.parse_simple_message();
      } else {
        panic!("Unexpected character: {:?}", c);
      }
    }

    let start = self.text.start_location();
    let end = self.text.end_location();

    SimpleMessage {
      parts: vec![MessagePart::Text(self.slice_text(start..end))],
    }
  }

  fn current_location(&self) -> Location {
    self.text.current_location()
  }

  fn slice_text(&self, range: Range<Location>) -> Text<'a> {
    let start = range.start;
    let content = self.text.slice(range);
    Text { start, content }
  }

  fn parse_simple_message(&mut self) -> SimpleMessage<'a> {
    let mut parts = vec![];

    let mut start = self.text.start_location();
    while let Some((loc, c)) = self.peek() {
      match c {
        '\\' => {
          if loc != start {
            parts.push(MessagePart::Text(self.slice_text(start..loc)));
          }
          let escape = self.parse_escape();
          parts.push(MessagePart::Escape(escape));
          start = self.current_location();
        }
        '{' => {
          if loc != start {
            parts.push(MessagePart::Text(self.slice_text(start..loc)));
          }
          parts.push(self.parse_placeholder());
          start = self.current_location();
        }
        '.' | '@' | '|' => {
          self.next();
        }
        c if is_content_char(c) || is_space(c) => {
          self.next();
        }
        _ => panic!("Unexpected character: {:?} (at {loc:?})", c),
      }
    }

    let end = self.current_location();
    if end != start {
      parts.push(MessagePart::Text(self.slice_text(start..end)));
    }

    SimpleMessage { parts }
  }

  fn parse_escape(&mut self) -> Escape {
    let (loc, c) = self.next().unwrap();
    debug_assert_eq!(c, '\\');

    let Some((_, char)) = self.next() else {
      panic!("Unexpected end of input")
    };

    Escape {
      start: loc,
      escaped_char: char,
    }
  }

  fn parse_placeholder(&mut self) -> MessagePart<'a> {
    let n = self.next();
    debug_assert!(matches!(n, Some((_, '{'))));

    self.skip_spaces();

    let (variable, literal, mut had_space) = match self.peek() {
      Some((_, '$')) => (Some(self.parse_variable()), None, self.skip_spaces()),
      Some((_, '|' | '-' | '0'..='9')) => {
        (None, Some(self.parse_literal()), self.skip_spaces())
      }
      Some((_, c)) if is_name_start(c) => {
        (None, Some(self.parse_literal()), self.skip_spaces())
      }
      Some((_, '#')) => {
        return MessagePart::Markup(
          self.parse_markup(MarkupStartKind::OpenOrStandalone),
        )
      }
      Some((_, '/')) => {
        return MessagePart::Markup(self.parse_markup(MarkupStartKind::Close))
      }
      _ => (None, None, true),
    };

    let annotation = if had_space {
      self.maybe_parse_annotation()
    } else {
      None
    };
    if annotation.is_some() {
      had_space = self.skip_spaces();
    }

    let mut attributes = vec![];

    while had_space {
      if self.eat('@').is_none() {
        break;
      }

      let key = self.parse_identifier();
      let mut value = None;
      had_space = self.skip_spaces();
      if self.eat('=').is_some() {
        self.skip_spaces();
        value = Some(self.parse_literal_or_variable());
        had_space = self.skip_spaces();
      }

      attributes.push(Attribute { key, value });
    }

    if self.eat('}').is_none() {
      panic!()
    }

    let expr = match (variable, literal) {
      (Some(variable), None) => MessagePart::Expression(
        Expression::VariableExpression(VariableExpression {
          variable,
          annotation,
          attributes,
        }),
      ),
      (None, Some(literal)) => MessagePart::Expression(
        Expression::LiteralExpression(LiteralExpression {
          literal,
          annotation,
          attributes,
        }),
      ),
      (None, None) => {
        if let Some(annotation) = annotation {
          MessagePart::Expression(Expression::AnnotationExpression(
            AnnotationExpression {
              annotation,
              attributes,
            },
          ))
        } else {
          panic!()
        }
      }
      _ => unreachable!(),
    };

    expr
  }

  fn parse_literal_or_variable(&mut self) -> LiteralOrVariable<'a> {
    match self.peek() {
      Some((_, '$')) => LiteralOrVariable::Variable(self.parse_variable()),
      Some((_, '|')) => {
        LiteralOrVariable::Literal(Literal::Quoted(self.parse_quoted()))
      }
      Some((_, c)) if is_name_start(c) => {
        LiteralOrVariable::Literal(Literal::Name(self.parse_literal_name()))
      }
      Some((_, '-' | '0'..='9')) => {
        LiteralOrVariable::Literal(Literal::Number(self.parse_number()))
      }
      _ => panic!(),
    }
  }

  fn parse_variable(&mut self) -> Variable<'a> {
    let start = self.current_location();

    let n = self.next();
    debug_assert_eq!(n.unwrap().1, '$');

    let name = self.parse_name();

    Variable { start, name }
  }

  fn parse_identifier(&mut self) -> Identifier<'a> {
    let start = self.current_location();
    let name_or_namespace = self.parse_name();

    if self.eat(':').is_some() {
      let name = self.parse_name();

      Identifier {
        start,
        namespace: Some(name_or_namespace),
        name,
      }
    } else {
      Identifier {
        start,
        namespace: None,
        name: name_or_namespace,
      }
    }
  }

  fn skip_name(&mut self) {
    let Some((_, c)) = self.next() else { panic!() };
    if !is_name_start(c) {
      panic!()
    }

    while let Some((_, c)) = self.peek() {
      if is_name_char(c) {
        self.next();
      } else {
        break;
      }
    }
  }

  fn parse_name(&mut self) -> &'a str {
    let start = self.current_location();
    self.skip_name();
    let end = self.current_location();

    &self.text.slice(start..end)
  }

  fn parse_literal_name(&mut self) -> Text<'a> {
    let start = self.current_location();
    self.skip_name();
    let end = self.current_location();

    self.slice_text(start..end)
  }

  fn next(&mut self) -> Option<(Location, char)> {
    self.text.next()
  }

  fn peek(&mut self) -> Option<(Location, char)> {
    self.text.peek()
  }

  fn eat(&mut self, c: char) -> Option<Location> {
    if let Some((loc, ch)) = self.text.peek() {
      if ch == c {
        self.text.next();
        return Some(loc);
      }
    }
    None
  }

  fn skip_spaces(&mut self) -> bool {
    let mut any_spaces = false;
    while let Some((_, c)) = self.peek() {
      if is_space(c) {
        any_spaces = true;
        self.next();
      } else {
        break;
      }
    }
    any_spaces
  }

  fn maybe_parse_annotation(&mut self) -> Option<Annotation<'a>> {
    match self.peek() {
      Some((_, ':')) => {
        // function
        self.next(); // consume ':'

        let id = self.parse_identifier();

        let mut options = vec![];

        loop {
          let before_space = self.current_location();
          let has_space = self.skip_spaces();
          if !has_space {
            break;
          }

          let has_name_start =
            self.peek().map(|(_, c)| is_name_start(c)).unwrap_or(false);
          if !has_name_start {
            self.text.reset_to(before_space);
            break;
          }

          options.push(self.parse_option());
        }

        Some(Annotation::Function(Function { id, options }))
      }
      Some((_, start @ ('^' | '&'))) => {
        // private-use-annotation
        self.next(); // consume start

        let reserved_body = self.parse_reserved_body();

        Some(Annotation::PrivateUseAnnotation(PrivateUseAnnotation {
          start,
          body: reserved_body,
        }))
      }
      Some((_, start @ ('!' | '%' | '*' | '+' | '<' | '>' | '?' | '~'))) => {
        // private-use-annotation
        self.next(); // consume start

        let reserved_body = self.parse_reserved_body();

        Some(Annotation::ReservedAnnotation(ReservedAnnotation {
          start,
          body: reserved_body,
        }))
      }
      _ => None,
    }
  }

  fn parse_option(&mut self) -> FnOrMarkupOption<'a> {
    let key = self.parse_identifier();
    self.skip_spaces();
    if self.eat('=').is_none() {
      panic!();
    }
    self.skip_spaces();
    let value = self.parse_literal_or_variable();

    FnOrMarkupOption { key, value }
  }

  fn parse_reserved_body(&mut self) -> Vec<ReservedBodyPart<'a>> {
    let mut parts = vec![];

    let mut start = self.current_location();
    let mut last_space_start = None;

    while let Some((loc, c)) = self.peek() {
      match c {
        c if is_reserved_char(c) => {
          self.next();
          last_space_start = None;
        }
        c if is_space(c) => {
          self.next();
          if last_space_start.is_none() {
            last_space_start = Some(loc);
          }
        }
        '\\' => {
          if loc != start {
            parts.push(ReservedBodyPart::Text(self.slice_text(start..loc)));
          }
          let escape = self.parse_escape();
          parts.push(ReservedBodyPart::Escape(escape));
          start = self.current_location();
          last_space_start = None;
        }
        '|' => {
          if loc != start {
            parts.push(ReservedBodyPart::Text(self.slice_text(start..loc)));
          }
          parts.push(ReservedBodyPart::Quoted(self.parse_quoted()));
          start = self.current_location();
          last_space_start = None;
        }
        _ => break,
      }
    }

    if let Some(start) = last_space_start {
      self.text.reset_to(start);
    }

    let end = self.current_location();
    if end != start {
      parts.push(ReservedBodyPart::Text(self.slice_text(start..end)));
    }

    parts
  }

  fn parse_literal(&mut self) -> Literal<'a> {
    match self.peek() {
      Some((_, '|')) => Literal::Quoted(self.parse_quoted()),
      Some((_, '-' | '0'..='9')) => Literal::Number(self.parse_number()),
      Some((_, c)) if is_name_start(c) => {
        Literal::Name(self.parse_literal_name())
      }
      _ => panic!(),
    }
  }

  fn parse_quoted(&mut self) -> Quoted<'a> {
    let (open, c) = self.next().unwrap(); // consume '|'
    debug_assert_eq!(c, '|');
    let mut parts = vec![];

    let mut start = self.current_location();

    while let Some((loc, ch)) = self.peek() {
      match ch {
        '\\' => {
          if start != loc {
            parts.push(QuotedPart::Text(self.slice_text(start..loc)));
          }
          let escape = self.parse_escape();
          parts.push(QuotedPart::Escape(escape));
          start = self.current_location();
        }
        '|' => {
          if start != loc {
            parts.push(QuotedPart::Text(self.slice_text(start..loc)));
          }
          break;
        }
        c if is_quoted_char(c) => {
          self.next();
        }
        _ => panic!("Unexpected character: {:?}", ch),
      }
    }

    let Some((close, c)) = self.next() else {
      panic!()
    };
    debug_assert_eq!(c, '|');

    Quoted { open, close, parts }
  }

  fn parse_number(&mut self) -> Number<'a> {
    let start = self.current_location();
    let is_negative = self.eat('-').is_some();

    // todo: disallow 01
    let integral_part = self.parse_digits();

    let fractional_part = if self.eat('.').is_some() {
      Some(self.parse_digits())
    } else {
      None
    };

    let exponent_part = if let Some((_, 'e' | 'E')) = self.peek() {
      self.next(); // consume 'e' or 'E'
      let sign = if self.eat('-').is_some() {
        ExponentSign::Minus
      } else {
        if self.eat('+').is_some() {
          ExponentSign::Plus
        } else {
          ExponentSign::None
        }
      };
      Some((sign, self.parse_digits()))
    } else {
      None
    };

    let end = self.current_location();

    Number {
      start,
      raw: self.text.slice(start..end),
      is_negative,
      integral_len: LengthShort::new_from_str(integral_part),
      fractional_len: fractional_part.map(LengthShort::new_from_str),
      exponent_len: exponent_part
        .map(|c| (c.0, LengthShort::new_from_str(c.1))),
    }
  }

  fn parse_digits(&mut self) -> &'a str {
    // todo: at least 1
    let start = self.current_location();
    while let Some((_, '0'..='9')) = self.peek() {
      self.next();
    }
    let end = self.current_location();
    self.text.slice(start..end)
  }

  fn parse_markup(&mut self, kind: MarkupStartKind) -> Markup<'a> {
    let c = self.next();
    debug_assert!(matches!(c, Some((_, '#' | '/'))));

    let id = self.parse_identifier();

    let mut markup_kind = match kind {
      MarkupStartKind::OpenOrStandalone => MarkupKind::Open,
      MarkupStartKind::Close => MarkupKind::Close,
    };
    let mut options = vec![];
    let mut attributes = vec![];

    let mut had_space = self.skip_spaces();
    loop {
      match self.peek() {
        Some((_, '@')) if had_space => {
          self.next(); // consume '@'

          let key = self.parse_identifier();
          let mut value = None;
          had_space = self.skip_spaces();
          if self.eat('=').is_some() {
            self.skip_spaces();
            value = Some(self.parse_literal_or_variable());
            had_space = self.skip_spaces();
          }

          attributes.push(Attribute { key, value });
        }
        Some((_, '/')) if matches!(markup_kind, MarkupKind::Open) => {
          self.next(); // consume '/'
          markup_kind = MarkupKind::Standalone;
          let Some(_) = self.eat('}') else { panic!() };
          break;
        }
        Some((_, '}')) => {
          self.next(); // consume '}'
          break;
        }
        Some((_, c))
          if had_space && is_name_start(c) && attributes.is_empty() =>
        {
          options.push(self.parse_option());
          had_space = self.skip_spaces();
        }
        _ => panic!(),
      }
    }

    Markup {
      kind: markup_kind,
      id,
      options,
      attributes,
    }
  }
}

fn is_content_char(c: char) -> bool {
  matches!(c,
    '\x01'..='\x08' | '\x0B'..='\x0C' | '\x0E'..='\x1F' | '\x21'..='\x2D' |
    '\x2F'..='\x3F' | '\x41'..='\x5B' | '\x5D'..='\x7A' | '\x7E'..='\u{2FFF}' |
    '\u{3001}'..='\u{D7FF}' | '\u{E000}'..='\u{10FFFF}'
  )
}

fn is_reserved_char(c: char) -> bool {
  is_content_char(c) || c == '.'
}

fn is_simple_start(c: char) -> bool {
  is_content_char(c) || is_space(c) || c == '@' || c == '|' // simple-start-char
    || c == '\\' // escaped-char
    || c == '{' // placeholder
}

fn is_space(c: char) -> bool {
  matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{3000}')
}

fn is_name_start(c: char) -> bool {
  matches!(c,
    'a'..='z' | 'A'..='Z' | '_' |
    '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}' |
    '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}' |
    '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' |
    '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFC}' | '\u{10000}'..='\u{EFFFF}'
  )
}

fn is_name_char(c: char) -> bool {
  is_name_start(c)
    || matches!(c,
      '0'..='9' | '-' | '.' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
    )
}

fn is_quoted_char(c: char) -> bool {
  is_content_char(c) || is_space(c) || matches!(c, '.' | '@' | '{' | '}')
}

enum MarkupStartKind {
  OpenOrStandalone,
  Close,
}
