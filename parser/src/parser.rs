use std::ops::Range;

use crate::ast::Annotation;
use crate::ast::AnnotationExpression;
use crate::ast::Attribute;
use crate::ast::ComplexMessage;
use crate::ast::ComplexMessageBody;
use crate::ast::Declaration;
use crate::ast::Escape;
use crate::ast::ExponentSign;
use crate::ast::Expression;
use crate::ast::FnOrMarkupOption;
use crate::ast::Function;
use crate::ast::Identifier;
use crate::ast::InputDeclaration;
use crate::ast::Key;
use crate::ast::Literal;
use crate::ast::LiteralExpression;
use crate::ast::LiteralOrVariable;
use crate::ast::LocalDeclaration;
use crate::ast::Markup;
use crate::ast::MarkupKind;
use crate::ast::Matcher;
use crate::ast::Message;
use crate::ast::Number;
use crate::ast::Pattern;
use crate::ast::PatternPart;
use crate::ast::PrivateUseAnnotation;
use crate::ast::Quoted;
use crate::ast::QuotedPart;
use crate::ast::QuotedPattern;
use crate::ast::ReservedAnnotation;
use crate::ast::ReservedBodyPart;
use crate::ast::ReservedStatement;
use crate::ast::Star;
use crate::ast::Text;
use crate::ast::Variable;
use crate::ast::VariableExpression;
use crate::ast::Variant;
use crate::chars;
use crate::diagnostic::Diagnostic;
use crate::util::LengthShort;
use crate::util::Location;
use crate::util::SourceTextInfo;
use crate::util::SourceTextIterator;
use crate::Span;
use crate::Spanned as _;

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
  ) -> (Message<'a>, Vec<Diagnostic<'a>>, SourceTextInfo<'a>) {
    while let Some((loc, c)) = self.peek() {
      match c {
        chars::space!() => {
          self.next();
        }

        chars::content!() | '@' | '|' // simple-start-char
         | '\\' // escaped-char
         | '\0' | '}' // error recovery
        => {
          return (
            Message::Simple(self.parse_pattern(self.text.start_location(), false)),
            self.diagnostics,
            self.text.into_info(),
          )
        }
        '{' => {
          // This could now either be a quoted pattern (so a complex message),
          // or a placeholder (so a simple message).
          self.next(); // eat '{'
          let peeked = self.peek();
          self.text.reset_to(loc); // reset to '{'
          match peeked {
            Some((_, '{')) => {
              return (
                Message::Complex(self.parse_complex_message()),
                self.diagnostics,
                self.text.into_info(),
              )
            }
            _ => {
              return (
                Message::Simple(self.parse_pattern(self.text.start_location(), false)),
                self.diagnostics,
                self.text.into_info(),
              )
            }
          }
        }
        '.' => {
          return (
            Message::Complex(self.parse_complex_message()),
            self.diagnostics,
            self.text.into_info(),
          )
        }
      }
    }

    let start = self.text.start_location();
    let end = self.text.end_location();

    (
      Message::Simple(Pattern {
        parts: vec![PatternPart::Text(self.slice_text(start..end))],
      }),
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

  fn parse_pattern(
    &mut self,
    mut start: Location,
    inside_quoted: bool,
  ) -> Pattern<'a> {
    let mut parts = vec![];

    while let Some((loc, c)) = self.peek() {
      match c {
        '\\' => {
          if loc != start {
            parts.push(PatternPart::Text(self.slice_text(start..loc)));
          }
          let escape = self.parse_escape();
          if let Some(escape) = escape {
            parts.push(PatternPart::Escape(escape));
          }
          start = self.current_location();
        }
        '{' => {
          if loc != start {
            parts.push(PatternPart::Text(self.slice_text(start..loc)));
          }
          parts.push(self.parse_placeholder());
          start = self.current_location();
        }
        '.' | '@' | '|' | chars::content!() | chars::space!() => {
          self.next();
        }
        '\0' => {
          self.report(Diagnostic::InvalidNullCharacter { char_loc: loc });
          self.next();
        }
        '}' => {
          // If we are inside a quoted pattern, and we see a double closing
          // brace, we should return early.
          self.next();
          if inside_quoted && self.peek().map(|(_, c)| c) == Some('}') {
            self.text.reset_to(loc);
            break;
          } else {
            self.report(Diagnostic::InvalidClosingBrace { brace_loc: loc });
          }
        }
      }
    }

    let end = self.current_location();
    if end != start {
      parts.push(PatternPart::Text(self.slice_text(start..end)));
    }

    Pattern { parts }
  }

  fn parse_escape(&mut self) -> Option<Escape> {
    let (start, c) = self.next().unwrap(); // consume '\'
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

  fn parse_placeholder(&mut self) -> PatternPart<'a> {
    let (start, c) = self.next().unwrap(); // consume '{'
    debug_assert_eq!(c, '{');

    self.skip_spaces();

    match self.peek() {
      Some((_, '#')) => {
        return PatternPart::Markup(
          self.parse_markup(start, MarkupStartKind::OpenOrStandalone),
        )
      }
      Some((_, '/')) => {
        return PatternPart::Markup(
          self.parse_markup(start, MarkupStartKind::Close),
        )
      }
      _ => {}
    }

    PatternPart::Expression(self.parse_expression(start))
  }

  // Caller must consume the opening `{` before calling this function and pass
  // the location of the opening `{` as `start`. The caller must also consume
  // spaces after the opening `{` before calling this function.
  fn parse_expression(&mut self, start: Location) -> Expression<'a> {
    let lit_or_var = self.parse_literal_or_variable();

    let had_space_before_annotation =
      lit_or_var.is_none() || self.skip_spaces();
    let mut had_space = false;

    let annotation = self.maybe_parse_annotation(&mut had_space);
    if let Some(ref annotation) = annotation {
      if !had_space_before_annotation {
        self.report(Diagnostic::AnnotationMissingSpaceBefore {
          span: annotation.span(),
        });
      }
    } else {
      had_space = had_space || had_space_before_annotation;
    }

    let mut attributes = vec![];

    while let Some((start, '@')) = self.peek() {
      attributes.push(self.parse_attribute(start, &mut had_space));
    }

    let contents_end = self.current_location();
    let mut after_invalid = None;

    loop {
      match self.peek() {
        Some((_, '|')) => {
          self.parse_quoted();
          after_invalid = Some(self.current_location());
        }
        Some((_, '}')) => {
          self.next();
          break;
        }
        Some((_, chars::space!())) => {
          self.next();
        }
        Some((_, '\\')) => {
          self.parse_escape();
          after_invalid = Some(self.current_location());
        }
        Some(_) => {
          self.next();
          after_invalid = Some(self.current_location());
        }
        None => {
          self.report(Diagnostic::PlaceholderMissingClosingBrace {
            span: Span::new(start..self.current_location()),
          });
          break;
        }
      }
    }

    if let Some(invalid_end) = after_invalid {
      self.report(Diagnostic::PlaceholderInvalidContents {
        span: Span::new(contents_end..invalid_end),
      });
    }

    let end = self.current_location();
    let span = Span::new(start..end);

    match lit_or_var {
      Some(LiteralOrVariable::Variable(variable)) => {
        Expression::VariableExpression(VariableExpression {
          span,
          variable,
          annotation,
          attributes,
        })
      }
      Some(LiteralOrVariable::Literal(literal)) => {
        Expression::LiteralExpression(LiteralExpression {
          span,
          literal,
          annotation,
          attributes,
        })
      }
      None => {
        if let Some(annotation) = annotation {
          Expression::AnnotationExpression(AnnotationExpression {
            span,
            annotation,
            attributes,
          })
        } else {
          self.report(Diagnostic::PlaceholderMissingBody { span });

          // We recover from this by injecting a literal expression with an
          // empty text as its literal.
          Expression::LiteralExpression(LiteralExpression {
            span,
            literal: Literal::Text(Text {
              start: span.start,
              content: "",
            }),
            annotation: None,
            attributes,
          })
        }
      }
    }
  }

  fn parse_literal_or_variable(&mut self) -> Option<LiteralOrVariable<'a>> {
    let value = match self.peek() {
      Some((_, '$')) => LiteralOrVariable::Variable(self.parse_variable()),
      Some((_, '|')) => {
        LiteralOrVariable::Literal(Literal::Quoted(self.parse_quoted()))
      }
      Some((_, chars::name_start!())) => {
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
    let span = Span::new(start..self.current_location());

    if name.is_empty() {
      self.report(Diagnostic::VariableMissingName { span });
    }

    Variable { span, name }
  }

  fn parse_attribute(
    &mut self,
    start: Location,
    had_space: &mut bool,
  ) -> Attribute<'a> {
    let c = self.next();
    debug_assert!(matches!(c, Some((_, '@'))));

    let report_missing_space_before_attribute = !*had_space;
    if self.skip_spaces() {
      self.report(Diagnostic::AttributeInvalidSpacesAfterAt {
        span: Span::new((start + '@')..self.current_location()),
      });
    }

    let (key, is_key_empty) = self.parse_identifier();

    let mut end = self.current_location();
    *had_space = self.skip_spaces();

    let value = self.eat('=').and_then(|_| {
      end = self.current_location();
      *had_space = self.skip_spaces();

      match self.parse_literal_or_variable() {
        Some(v) => {
          end = self.current_location();
          *had_space = self.skip_spaces();
          Some(v)
        }
        None => {
          self.report(Diagnostic::AttributeMissingValue {
            span: Span::new(start..end),
          });
          None
        }
      }
    });

    let span = Span::new(start..end);

    if report_missing_space_before_attribute {
      self.report(Diagnostic::AttributeMissingSpaceBefore { span });
    }

    if is_key_empty {
      self.report(Diagnostic::AttributeMissingKey { span });
    }

    Attribute { span, key, value }
  }

  // Returns the identifier and a boolean indicating if the identifier is empty.
  // The caller should report an error if the identifier is empty.
  fn parse_identifier(&mut self) -> (Identifier<'a>, bool) {
    let start = self.current_location();
    let name_or_namespace = self.parse_name();

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

    // Report an error if id.name is empty, but id.namespace is not. If they are
    // both empty, the caller will deal with it.
    if id.name.is_empty() && id.namespace.is_some() {
      self.report(Diagnostic::MissingIdentifierName {
        identifier: id.clone(),
      });
    }
    if matches!(id.namespace, Some(s) if s.is_empty()) {
      self.report(Diagnostic::MissingIdentifierNamespace {
        identifier: id.clone(),
      });
    }

    let is_empty = id.namespace.is_none() && id.name.is_empty();
    (id, is_empty)
  }

  fn skip_name(&mut self) {
    if let Some((_, chars::name_start!())) = self.peek() {
      self.next();

      while let Some((_, chars::name!())) = self.peek() {
        self.next();
      }
    }
  }

  // Caller must handle empty name
  fn parse_name(&mut self) -> &'a str {
    let start = self.current_location();
    self.skip_name();
    let end = self.current_location();

    self.text.slice(start..end)
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
    while let Some((_, chars::space!())) = self.peek() {
      any_spaces = true;
      self.next();
    }
    any_spaces
  }

  fn maybe_parse_annotation(
    &mut self,
    had_space: &mut bool,
  ) -> Option<Annotation<'a>> {
    match self.peek() {
      Some((start, ':')) => {
        // function
        self.next(); // consume ':'

        let (id, is_id_empty) = self.parse_identifier();

        let mut options = vec![];

        loop {
          *had_space = self.skip_spaces();
          if !*had_space {
            break;
          }

          let has_name_start = self
            .peek()
            // also allow : as error recovery for `{ :fn a:b=c :d=e }` (missing namespace on option)
            // also allow = as error recovery for { :fn a=b =c } (missing key on option)
            .map(|(_, c)| matches!(c, chars::name_start!() | ':' | '='))
            .unwrap_or(false);
          if !has_name_start {
            break;
          }

          options.push(self.parse_option());
        }

        let function = Function { start, id, options };

        if is_id_empty {
          self.report(Diagnostic::FunctionMissingIdentifier {
            span: function.span(),
          });
        }

        Some(Annotation::Function(function))
      }
      Some((start, sigil @ ('^' | '&'))) => {
        // private-use-annotation
        self.next(); // consume start

        let reserved_body = self.parse_reserved_body(had_space);

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

        let reserved_body = self.parse_reserved_body(had_space);

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
    let (key, is_key_empty) = self.parse_identifier();
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

    let option = FnOrMarkupOption { key, value };

    if is_key_empty {
      self.report(Diagnostic::OptionMissingKey {
        span: option.span(),
      })
    }

    option
  }

  fn parse_reserved_body(
    &mut self,
    had_space: &mut bool,
  ) -> Vec<ReservedBodyPart<'a>> {
    let mut parts = vec![];

    let mut start = self.current_location();
    let mut last_space_start = None;

    while let Some((loc, c)) = self.peek() {
      match c {
        chars::reserved!() => {
          self.next();
          last_space_start = None;
        }
        chars::space!() => {
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

    let end = if let Some(start) = last_space_start {
      *had_space = true;
      start
    } else {
      self.current_location()
    };

    if end != start {
      parts.push(ReservedBodyPart::Text(self.slice_text(start..end)));
    }

    parts
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
        chars::quoted!() => {
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
      } else if self.eat('+').is_some() {
        ExponentSign::Plus
      } else {
        ExponentSign::None
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

  // Caller must handle empty digits, and leading zero
  fn parse_digits(&mut self) -> &'a str {
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

    let (id, is_id_empty) = self.parse_identifier();

    let mut markup_kind = match kind {
      MarkupStartKind::OpenOrStandalone => MarkupKind::Open,
      MarkupStartKind::Close => MarkupKind::Close,
    };
    let mut options = vec![];
    let mut attributes = vec![];

    let mut had_space = self.skip_spaces();
    let report_missing_close = 'outer: loop {
      match self.peek() {
        Some((start, '@')) => {
          attributes.push(self.parse_attribute(start, &mut had_space));
        }
        Some((self_close, '/')) => {
          self.next(); // consume '/'

          had_space = self.skip_spaces();

          let report_missing_close = match self.peek() {
            Some((_, '}')) => {
              if had_space {
                self.report(
                  Diagnostic::MarkupInvalidSpaceBetweenSelfCloseAndBrace {
                    space: Span::new(
                      (self_close + '/')..self.current_location(),
                    ),
                  },
                );
              }

              self.next(); // consume '}'
              false
            }
            None => true,
            Some(_) => {
              self.skip_invalid_markup_contents(self_close, &mut had_space);
              continue 'outer;
            }
          };

          if matches!(markup_kind, MarkupKind::Close) {
            self.report(Diagnostic::MarkupCloseInvalidSelfClose {
              self_close_loc: self_close,
            });
          }
          markup_kind = MarkupKind::Standalone;

          break report_missing_close;
        }
        Some((_, '}')) => {
          self.next(); // consume '}'
          break false;
        }
        // also allow : as error recovery for `{#fn a:b=c :d=e}` (missing namespace on option)
        // also allow = as error recovery for {#fn a=b =c} (missing key on option)
        Some((_, chars::name_start!() | ':' | '=')) if had_space => {
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
        Some((loc, _)) => {
          self.skip_invalid_markup_contents(loc, &mut had_space);
        }
        None => {
          break true;
        }
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

    if is_id_empty {
      self.report(Diagnostic::MarkupMissingIdentifier { span: markup.span })
    }

    if report_missing_close {
      self.report(Diagnostic::MarkupMissingClosingBrace { span: markup.span });
    }

    markup
  }

  fn skip_invalid_markup_contents(
    &mut self,
    start: Location,
    had_space: &mut bool,
  ) {
    let mut last_space_start = None;

    while let Some((loc, c)) = self.peek() {
      match c {
        '}' => {
          break;
        }
        '\\' => {
          self.parse_escape();
          last_space_start = None;
        }
        '|' => {
          self.parse_quoted();
          last_space_start = None;
        }
        '/' | '@' => {
          break;
        }
        chars::space!() => {
          if last_space_start.is_none() {
            last_space_start = Some(loc);
          }
          self.next();
        }
        chars::name_start!() | ':' | '=' if last_space_start.is_some() => {
          break;
        }
        _ => {
          self.next();
          last_space_start = None;
        }
      }
    }

    let end = if let Some(start) = last_space_start {
      *had_space = true;
      start
    } else {
      *had_space = false;
      self.current_location()
    };

    if end != start {
      self.report(Diagnostic::MarkupInvalidContents {
        span: Span::new(start..end),
      });
    }
  }

  fn parse_complex_message(&mut self) -> ComplexMessage<'a> {
    let mut declarations = vec![];
    let mut body = None;

    loop {
      match self.peek() {
        Some((_, chars::space!())) => {
          self.next();
        }
        Some((start, '.')) => {
          self.next(); // consume '.'
          let name = self.parse_name();
          let declaration = match name {
            "input" => {
              let input = self.parse_input_declaration(start);
              Declaration::InputDeclaration(input)
            }
            "local" => {
              let local = self.parse_local_declaration(start);
              Declaration::LocalDeclaration(local)
            }
            "match" => {
              let matcher = self.parse_matcher(start);
              if body.is_some() {
                self.report(Diagnostic::ComplexMessageMultipleBodies {
                  span: matcher.span(),
                });
              } else {
                body = Some(ComplexMessageBody::Matcher(matcher));
              }
              continue;
            }
            _ => {
              let reserved = self.parse_reserved_statement();
              Declaration::ReservedStatement(reserved)
            }
          };
          if body.is_some() {
            self.report(Diagnostic::ComplexMessageDeclarationAfterBody {
              span: declaration.span(),
            });
          }
          declarations.push(declaration);
        }
        Some((loc, '{')) => {
          // parse quoted pattern, or error recover for placeholder
          self.next(); // consume '{'
          let peeked = self.peek();
          if let Some((_, '{')) = peeked {
            let quoted = self.parse_quoted_pattern(loc);
            if body.is_some() {
              self.report(Diagnostic::ComplexMessageMultipleBodies {
                span: quoted.span(),
              });
            } else {
              body = Some(ComplexMessageBody::QuotedPattern(quoted));
            }
          } else {
            self.text.reset_to(loc); // reset to '{'
            break;
          }
        }
        _ => {
          break;
        }
      }
    }

    // error recovery for an unquoted pattern
    if self.peek().is_some() {
      debug_assert!(!matches!(self.peek(), Some((_, chars::space!()))));
      if body.is_some() {
        self.report(Diagnostic::ComplexMessageTrailingContent {
          span: Span::new(self.current_location()..self.text.end_location()),
        });
      } else {
        let pattern = self.parse_pattern(self.current_location(), false);
        // todo: remove trailing spaces from the pattern
        self.report(Diagnostic::ComplexMessageBodyNotQuoted {
          span: pattern.span(),
        });
        body = Some(ComplexMessageBody::QuotedPattern(QuotedPattern {
          span: pattern.span(),
          pattern,
        }));
      }
    }

    let body = body.unwrap_or_else(|| {
      self.report(Diagnostic::ComplexMessageMissingBody {
        span: Span::new(self.current_location()..self.current_location()),
      });
      ComplexMessageBody::QuotedPattern(QuotedPattern {
        span: Span::new(self.current_location()..self.current_location()),
        pattern: Pattern {
          parts: vec![PatternPart::Text(Text {
            start: self.current_location(),
            content: "",
          })],
        },
      })
    });

    ComplexMessage { declarations, body }
  }

  fn parse_local_declaration(
    &mut self,
    start: Location,
  ) -> LocalDeclaration<'a> {
    // At this point, `.local` has already been consumed. `start` is the location of the `.`.
    let has_space = self.skip_spaces();
    if !has_space {
      self.report(Diagnostic::LocalKeywordMissingTrailingSpace {
        span: Span::new(start..self.current_location()),
      });
    }

    let next = self.peek();
    let variable = match next {
      Some((_, '$')) => self.parse_variable(),
      Some((start, chars::name_start!())) => {
        let name = self.parse_name();
        let span = Span::new(start..self.current_location());
        self.report(Diagnostic::LocalVariableMissingDollar { span });
        Variable { span, name }
      }
      _ => todo!("go into declaration error recovery"),
    };

    self.skip_spaces();

    if self.eat('=').is_none() {
      // if next token is a brace, report missing equals but keep parsing as local decl
      todo!("go into declaration error recovery");
    }

    self.skip_spaces();

    let Some(open) = self.eat('{') else {
      todo!("go into declaration error recovery");
    };

    self.skip_spaces();

    let expression = self.parse_expression(open);

    LocalDeclaration {
      start,
      variable,
      expression,
    }
  }

  fn parse_input_declaration(
    &mut self,
    start: Location,
  ) -> InputDeclaration<'a> {
    // At this point, `.input` has already been consumed. `start` is the location of the `.`.

    self.skip_spaces();

    let Some(open) = self.eat('{') else {
      todo!("go into declaration error recovery");
    };

    self.skip_spaces();

    let expression = self.parse_expression(open);
    let Expression::VariableExpression(expression) = expression else {
      todo!("report non variable input declaration")
    };

    InputDeclaration { start, expression }
  }

  fn parse_reserved_statement(&mut self) -> ReservedStatement<'a> {
    todo!()
  }

  fn parse_matcher(&mut self, start: Location) -> Matcher<'a> {
    // At this point, `.match` has already been consumed. `start` is the location of the `.`.

    let mut selectors = vec![];

    self.skip_spaces();
    while let Some(open) = self.eat('{') {
      self.skip_spaces();
      let expression = self.parse_expression(open);
      selectors.push(expression);
      self.skip_spaces();
    }

    // todo, report error for no selectors

    let mut variants = vec![];
    let mut current_variant_keys = vec![];

    let mut had_space_or_closing_curly = true; // we had an expression
    while let Some((loc, c)) = self.peek() {
      match c {
        '*' => {
          self.next();
          let key = Key::Star(Star { start: loc });
          if !had_space_or_closing_curly {
            self.report(Diagnostic::MissingSpaceBeforeKey { span: key.span() })
          }
          current_variant_keys.push(key);
          had_space_or_closing_curly = self.skip_spaces();
        }
        '{' => {
          self.next();
          if let Some((_, '{')) = self.peek() {
            let pattern = self.parse_quoted_pattern(loc);
            let keys = std::mem::take(&mut current_variant_keys);
            // todo, at least one key is required
            variants.push(Variant { keys, pattern });
          } else {
            todo!("parse as expression and use as quoted pattern for variant")
          }
          self.skip_spaces();
          had_space_or_closing_curly = true;
        }
        '.' => {
          break;
        }
        _ => {
          let literal_or_variable = self.parse_literal_or_variable();
          let key = match literal_or_variable {
            Some(LiteralOrVariable::Literal(literal)) => Key::Literal(literal),
            Some(LiteralOrVariable::Variable(variable)) => {
              let span = variable.span();
              self.report(Diagnostic::MatcherKeyIsVariable { span });
              Key::Literal(Literal::Text(Text {
                start: variable.span.start,
                content: self.text.slice(span.start..span.end),
              }))
            }
            None => {
              // eat until the next space or quoted pattern

              todo!("error recovery for invalid matcher key")
            }
          };
          if !had_space_or_closing_curly {
            self.report(Diagnostic::MissingSpaceBeforeKey { span: key.span() })
          }
          current_variant_keys.push(key);
          had_space_or_closing_curly = self.skip_spaces();
        }
      }
    }

    if !current_variant_keys.is_empty() {
      variants.push(Variant {
        keys: std::mem::take(&mut current_variant_keys),
        pattern: QuotedPattern {
          span: Span::new(self.current_location()..self.current_location()),
          pattern: Pattern {
            parts: vec![PatternPart::Text(Text {
              start: self.current_location(),
              content: "",
            })],
          },
        },
      });
      todo!("report missing pattern for matcher variant")
    }

    Matcher {
      start,
      selectors,
      variants,
    }
  }

  fn parse_quoted_pattern(&mut self, start: Location) -> QuotedPattern<'a> {
    // At this point we have consumed the first `{` (this is `start`) and are
    // sure that the next character is `{`.
    self.eat('{').unwrap(); // consume '{'

    let pattern = self.parse_pattern(self.current_location(), true);

    // Now consume the closing `}}`.

    let maybe_close = self.next();
    match maybe_close {
      Some((_, '}')) => {
        self.eat('}').unwrap(); // consume the second '}' - parse_pattern guarantees it's there
      }
      Some(_) => unreachable!(),
      None => {
        self.report(Diagnostic::UnterminatedQuotedPattern {
          span: Span::new(start..self.current_location()),
        });
      }
    }

    QuotedPattern {
      span: Span::new(start..self.current_location()),
      pattern,
    }
  }
}

enum MarkupStartKind {
  OpenOrStandalone,
  Close,
}
