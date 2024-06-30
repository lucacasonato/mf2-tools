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
use crate::diagnostic::Diagnostic;
use crate::util::LengthShort;
use crate::util::Location;
use crate::util::SourceTextInfo;
use crate::util::SourceTextIterator;
use crate::Span;
use crate::Spanned as _;

macro_rules! content_char_pattern {
  () => {
    '\x01'..='\x08' | '\x0B'..='\x0C' | '\x0E'..='\x1F' | '\x21'..='\x2D' |
    '\x2F'..='\x3F' | '\x41'..='\x5B' | '\x5D'..='\x7A' | '\x7E'..='\u{2FFF}' |
    '\u{3001}'..='\u{D7FF}' | '\u{E000}'..='\u{10FFFF}'
  };
}

macro_rules! space_pattern {
  () => {
    ' ' | '\t' | '\r' | '\n' | '\u{3000}'
  };
}

macro_rules! simple_start_pattern {
  () => {
    content_char_pattern!() | space_pattern!() | '@' | '|' | // simple-start-char
    '\\' | // escaped-char
    '{' // placeholder
  };
}

macro_rules! reserved_char_pattern {
  () => {
    content_char_pattern!() | '.'
  };
}

macro_rules! name_start_pattern {
  () => {
    'a'..='z' | 'A'..='Z' | '_' |
    '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}' |
    '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}' |
    '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' |
    '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFC}' | '\u{10000}'..='\u{EFFFF}'
  };
}

macro_rules! name_char_pattern {
  () => {
    name_start_pattern!() |
    '0'..='9' | '-' | '.' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
  };
}

macro_rules! quoted_char_pattern {
  () => {
    content_char_pattern!() | space_pattern!() | '.' | '@' | '{' | '}'
  };
}

pub struct Parser<'a> {
  text: SourceTextIterator<'a>,
  diagnostics: Vec<Diagnostic<'a>>,
}

impl<'a> Parser<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      text: SourceTextIterator::new(input),
      diagnostics: vec![],
    }
  }

  pub fn parse(
    mut self,
  ) -> (SimpleMessage<'a>, Vec<Diagnostic<'a>>, SourceTextInfo<'a>) {
    while let Some((_, c)) = self.peek() {
      match c {
        simple_start_pattern!() => {
          return (
            self.parse_simple_message(),
            self.diagnostics,
            self.text.into_info(),
          )
        }
        _ => {
          panic!("Unexpected character: {:?}", c);
        }
      }
    }

    let start = self.text.start_location();
    let end = self.text.end_location();

    (
      SimpleMessage {
        parts: vec![MessagePart::Text(self.slice_text(start..end))],
      },
      self.diagnostics,
      self.text.into_info(),
    )
  }

  fn current_location(&self) -> Location {
    self.text.current_location()
  }

  fn slice_text(&self, range: Range<Location>) -> Text<'a> {
    let start = range.start;
    let content = self.text.slice(range);
    Text { start, content }
  }

  fn report(&mut self, diagnostic: Diagnostic<'a>) {
    self.diagnostics.push(diagnostic);
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
          if let Some(escape) = escape {
            parts.push(MessagePart::Escape(escape));
          }
          start = self.current_location();
        }
        '{' => {
          if loc != start {
            parts.push(MessagePart::Text(self.slice_text(start..loc)));
          }
          parts.push(self.parse_placeholder());
          start = self.current_location();
        }
        '.' | '@' | '|' | content_char_pattern!() | space_pattern!() => {
          self.next();
        }
        '\0' => {
          self.report(Diagnostic::InvalidNullCharacter { char_loc: loc });
          self.next();
        }
        '}' => {
          self.report(Diagnostic::InvalidClosingBrace { brace_loc: loc });
          self.next();
        }
      }
    }

    let end = self.current_location();
    if end != start {
      parts.push(MessagePart::Text(self.slice_text(start..end)));
    }

    SimpleMessage { parts }
  }

  fn parse_escape(&mut self) -> Option<Escape> {
    let (start, c) = self.next().unwrap();
    debug_assert_eq!(c, '\\');

    let escaped_char = match self.next() {
      Some((_, c @ ('}' | '{' | '|' | '\\'))) => c,
      Some((loc, c)) => {
        self.report(Diagnostic::EscapeInvalidCharacter {
          char: c,
          char_loc: loc,
        });
        c
      }
      None => {
        self.report(Diagnostic::EscapeMissingCharacter { slash_loc: start });
        return None;
      }
    };

    Some(Escape {
      start,
      escaped_char,
    })
  }

  fn parse_placeholder(&mut self) -> MessagePart<'a> {
    let (start, c) = self.next().unwrap(); // consume '{'
    debug_assert_eq!(c, '{');

    self.skip_spaces();

    let (variable, literal, mut had_space) = match self.peek() {
      Some((_, '$')) => (Some(self.parse_variable()), None, self.skip_spaces()),
      // '.' is for error recovery of a fractional number literal that is missing the integral part
      Some((_, '|' | '.' | '-' | '0'..='9')) => {
        (None, Some(self.parse_literal()), self.skip_spaces())
      }
      Some((_, name_start_pattern!())) => {
        (None, Some(self.parse_literal()), self.skip_spaces())
      }
      Some((_, '#')) => {
        return MessagePart::Markup(
          self.parse_markup(start, MarkupStartKind::OpenOrStandalone),
        )
      }
      Some((_, '/')) => {
        return MessagePart::Markup(
          self.parse_markup(start, MarkupStartKind::Close),
        )
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
      let Some(start) = self.eat('@') else {
        break;
      };

      let key = self.parse_identifier();
      let mut value = None;
      had_space = self.skip_spaces();
      if self.eat('=').is_some() {
        self.skip_spaces();
        value = Some(
          self
            .parse_literal_or_variable()
            .expect("todo, handle missing attribute value"),
        );
        had_space = self.skip_spaces();
      }

      attributes.push(Attribute { start, key, value });
    }

    let maybe_close = self.eat('}');
    let end = self.current_location();
    let span = Span::new(start..end);

    if maybe_close.is_none() {
      self.report(Diagnostic::PlaceholderMissingClosingBrace { span });
    }

    let expr = match (variable, literal) {
      (Some(variable), None) => MessagePart::Expression(
        Expression::VariableExpression(VariableExpression {
          span,
          variable,
          annotation,
          attributes,
        }),
      ),
      (None, Some(literal)) => MessagePart::Expression(
        Expression::LiteralExpression(LiteralExpression {
          span,
          literal,
          annotation,
          attributes,
        }),
      ),
      (None, None) => {
        if let Some(annotation) = annotation {
          MessagePart::Expression(Expression::AnnotationExpression(
            AnnotationExpression {
              span,
              annotation,
              attributes,
            },
          ))
        } else {
          self.report(Diagnostic::PlaceholderMissingBody { span });

          // We recover from this by injecting a literal expression with an
          // empty text as its literal.
          MessagePart::Expression(Expression::LiteralExpression(
            LiteralExpression {
              span,
              literal: Literal::Text(Text {
                start: span.start,
                content: "",
              }),
              annotation: None,
              attributes,
            },
          ))
        }
      }
      _ => unreachable!(),
    };

    expr
  }

  fn parse_literal_or_variable(&mut self) -> Option<LiteralOrVariable<'a>> {
    let value = match self.peek() {
      Some((_, '$')) => LiteralOrVariable::Variable(self.parse_variable()),
      Some((_, '|')) => {
        LiteralOrVariable::Literal(Literal::Quoted(self.parse_quoted()))
      }
      Some((_, name_start_pattern!())) => {
        LiteralOrVariable::Literal(Literal::Text(self.parse_literal_name()))
      }
      // '.' is for error recovery of a fractional number literal that is missing the integral part
      Some((_, '-' | '.' | '0'..='9')) => {
        LiteralOrVariable::Literal(Literal::Number(self.parse_number()))
      }
      _ => return None,
    };
    Some(value)
  }

  fn parse_variable(&mut self) -> Variable<'a> {
    let (start, n) = self.next().unwrap(); // consume '$'
    debug_assert_eq!(n, '$');

    let name = self.parse_name();

    Variable { start, name }
  }

  fn parse_identifier(&mut self) -> Identifier<'a> {
    let start = self.current_location();
    let name_or_namespace = self.parse_name();

    if name_or_namespace.is_empty() && !matches!(self.peek(), Some((_, ':'))) {
      todo!("Handle the missing identifier name case")
    }

    let id = if self.eat(':').is_some() {
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
    };

    if id.name.is_empty() {
      self.report(Diagnostic::MissingIdentifierName {
        identifier: id.clone(),
      });
    }
    if matches!(id.namespace, Some(s) if s.is_empty()) {
      self.report(Diagnostic::MissingIdentifierNamespace {
        identifier: id.clone(),
      });
    }

    id
  }

  fn skip_name(&mut self) {
    if let Some((_, name_start_pattern!())) = self.peek() {
      self.next();

      while let Some((_, name_char_pattern!())) = self.peek() {
        self.next();
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
      Some((start, ':')) => {
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

          let has_name_start = self
            .peek()
            // also allow : as error recovery for `{ :fn a:b=c :d=e }` (missing namespace on option)
            .map(|(_, c)| matches!(c, name_start_pattern!() | ':'))
            .unwrap_or(false);
          if !has_name_start {
            self.text.reset_to(before_space);
            break;
          }

          options.push(self.parse_option());
        }

        Some(Annotation::Function(Function { start, id, options }))
      }
      Some((start, sigil @ ('^' | '&'))) => {
        // private-use-annotation
        self.next(); // consume start

        let reserved_body = self.parse_reserved_body();

        Some(Annotation::PrivateUseAnnotation(PrivateUseAnnotation {
          start,
          sigil,
          body: reserved_body,
        }))
      }
      Some((
        start,
        sigil @ ('!' | '%' | '*' | '+' | '<' | '>' | '?' | '~'),
      )) => {
        // reserved annotation
        self.next(); // consume start

        let reserved_body = self.parse_reserved_body();

        Some(Annotation::ReservedAnnotation(ReservedAnnotation {
          start,
          sigil,
          body: reserved_body,
        }))
      }
      _ => None,
    }
  }

  fn parse_option(&mut self) -> FnOrMarkupOption<'a> {
    let key = self.parse_identifier();
    self.skip_spaces();
    let value = if let Some(equals_loc) = self.eat('=') {
      self.skip_spaces();
      self.parse_literal_or_variable().unwrap_or_else(|| {
        self.text.reset_to(equals_loc + '='); // un-eat the spaces after the equals
        self.report(Diagnostic::OptionMissingValue {
          span: Span::new(key.start..self.current_location()),
        });
        LiteralOrVariable::Literal(Literal::Text(Text {
          start: self.current_location(),
          content: "",
        }))
      })
    } else {
      self.text.reset_to(key.span().end); // un-eat the spaces after the identifier
      self.report(Diagnostic::OptionMissingValue {
        span: Span::new(key.start..self.current_location()),
      });
      LiteralOrVariable::Literal(Literal::Text(Text {
        start: self.current_location(),
        content: "",
      }))
    };

    FnOrMarkupOption { key, value }
  }

  fn parse_reserved_body(&mut self) -> Vec<ReservedBodyPart<'a>> {
    let mut parts = vec![];

    let mut start = self.current_location();
    let mut last_space_start = None;

    while let Some((loc, c)) = self.peek() {
      match c {
        reserved_char_pattern!() => {
          self.next();
          last_space_start = None;
        }
        space_pattern!() => {
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
          if let Some(escape) = escape {
            parts.push(ReservedBodyPart::Escape(escape));
          }
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
      // '.' is for error recovery of a fractional number literal that is missing the integral part
      Some((_, '-' | '.' | '0'..='9')) => Literal::Number(self.parse_number()),
      Some((_, name_start_pattern!())) => {
        Literal::Text(self.parse_literal_name())
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
          if let Some(escape) = escape {
            parts.push(QuotedPart::Escape(escape));
          }
          start = self.current_location();
        }
        '|' => {
          if start != loc {
            parts.push(QuotedPart::Text(self.slice_text(start..loc)));
          }
          break;
        }
        quoted_char_pattern!() => {
          self.next();
        }
        '\0' => {
          self.report(Diagnostic::InvalidNullCharacter { char_loc: loc });
          self.next();
        }
      }
    }

    let maybe_close = self.eat('|');
    let span = Span::new(open..self.current_location());

    if maybe_close.is_none() {
      self.report(Diagnostic::UnterminatedQuoted { span });
    }

    Quoted { span, parts }
  }

  fn parse_number(&mut self) -> Number<'a> {
    let start = self.current_location();
    let is_negative = self.eat('-').is_some();

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

    let num = Number {
      start,
      raw: self.text.slice(start..end),
      is_negative,
      integral_len: LengthShort::new_from_str(integral_part),
      fractional_len: fractional_part.map(LengthShort::new_from_str),
      exponent_len: exponent_part
        .map(|c| (c.0, LengthShort::new_from_str(c.1))),
    };

    if integral_part.len() > 1 && integral_part.starts_with('0') {
      self.report(Diagnostic::NumberLeadingZeroIntegralPart {
        number: num.clone(),
      });
    }
    if integral_part.is_empty() {
      self.report(Diagnostic::NumberMissingIntegralPart {
        number: num.clone(),
      });
    }
    if matches!(fractional_part, Some(s) if s.is_empty()) {
      self.report(Diagnostic::NumberMissingFractionalPart {
        number: num.clone(),
      });
    }
    if matches!(exponent_part, Some((_, s)) if s.is_empty()) {
      self.report(Diagnostic::NumberMissingExponentPart {
        number: num.clone(),
      });
    }

    num
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

  fn parse_markup(
    &mut self,
    open: Location,
    kind: MarkupStartKind,
  ) -> Markup<'a> {
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
    let report_missing_close = loop {
      match self.peek() {
        Some((start, '@')) => {
          let report_missing_space_before_attribute = !had_space;

          self.next(); // consume '@'

          let key = self.parse_identifier();
          let mut value = None;
          had_space = self.skip_spaces();
          if self.eat('=').is_some() {
            self.skip_spaces();
            value = Some(
              self
                .parse_literal_or_variable()
                .expect("todo, handle missing option value"),
            );
            had_space = self.skip_spaces();
          }

          let attribute = Attribute { start, key, value };

          if report_missing_space_before_attribute {
            self.report(Diagnostic::MarkupMissingSpaceBeforeAttribute {
              attribute: attribute.clone(),
            });
          }

          attributes.push(attribute);
        }
        Some((self_close, '/')) => {
          if matches!(markup_kind, MarkupKind::Close) {
            self.report(Diagnostic::MarkupCloseInvalidSelfClose {
              self_close_loc: self_close,
            });
          }
          self.next(); // consume '/'
          markup_kind = MarkupKind::Standalone;
          match self.peek() {
            Some((_, '}')) => {
              self.next(); // consume '}'
              break false;
            }
            Some((before_first_space, space_pattern!())) => {
              self.skip_spaces();
              match self.peek() {
                Some((close_brace, '}')) => {
                  self.next(); // consume '}'
                  self.report(
                    Diagnostic::MarkupInvalidSpaceBetweenSelfCloseAndBrace {
                      space: Span::new(before_first_space..close_brace),
                    },
                  );
                  break false;
                }
                None => {
                  break true;
                }
                _ => {
                  self.text.reset_to(before_first_space);
                  todo!("report expected }} after /")
                }
              }
            }
            None => {
              break true;
            }
            _ => {
              todo!("report invalid char in markup")
            }
          }
        }
        Some((_, '}')) => {
          self.next(); // consume '}'
          break false;
        }
        Some((_, name_start_pattern!())) if had_space => {
          let option = self.parse_option();
          if let Some(previous_attribute) = attributes.last() {
            self.report(Diagnostic::MarkupOptionAfterAttribute {
              previous_attribute: previous_attribute.clone(),
              option: option.clone(),
            })
          }
          options.push(option);
          had_space = self.skip_spaces();
        }
        None => {
          break true;
        }
        _ => todo!("report invalid char in markup"),
      }
    };

    let end = self.current_location();

    let markup = Markup {
      span: Span::new(open..end),
      kind: markup_kind,
      id,
      options,
      attributes,
    };

    if report_missing_close {
      self.report(Diagnostic::MarkupMissingClosingBrace { span: markup.span });
    }

    markup
  }
}

fn is_space(c: char) -> bool {
  matches!(c, space_pattern!())
}

enum MarkupStartKind {
  OpenOrStandalone,
  Close,
}
