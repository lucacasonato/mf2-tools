use std::iter::Peekable;
use std::str::CharIndices;

use crate::ast::Annotation;
use crate::ast::Attribute;
use crate::ast::Escape;
use crate::ast::Expression;
use crate::ast::Function;
use crate::ast::Identifier;
use crate::ast::LiteralOrVariable;
use crate::ast::MessagePart;
use crate::ast::PrivateUseAnnotation;
use crate::ast::ReservedAnnotation;
use crate::ast::ReservedBodyPart;
use crate::ast::SimpleMessage;
use crate::ast::Text;
use crate::ast::Variable;
use crate::ast::VariableExpression;
use crate::util::ResettablePeekableCharIndices;

pub struct Parser<'a> {
  input: &'a str,
  chars: ResettablePeekableCharIndices<'a>,
}

impl<'a> Parser<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      input,
      chars: ResettablePeekableCharIndices::new(input),
    }
  }

  pub fn parse(mut self) -> SimpleMessage<'a> {
    while let Some((_, c)) = self.peek() {
      if is_space(c) {
        self.next();
        continue;
      } else if is_simple_start(c) {
        return self.parse_simple_message();
      } else {
        panic!("Unexpected character: {:?}", c);
      }
    }
    SimpleMessage {
      parts: vec![MessagePart::Text(Text {
        content: self.input,
      })],
    }
  }

  /// The start index of the char that would be returned from `next()`.
  fn next_byte_index(&mut self) -> usize {
    self.chars.front_offset
  }

  fn parse_simple_message(&mut self) -> SimpleMessage<'a> {
    let mut parts = vec![];

    let mut start = 0;
    while let Some((byte_index, c)) = self.peek() {
      match c {
        '\\' => {
          if byte_index != start {
            parts.push(MessagePart::Text(Text {
              content: &self.input[start..byte_index],
            }))
          }
          self.next(); // consume '\\'
          let Some((_, char)) = self.next() else {
            panic!("Unexpected end of input")
          };
          parts.push(MessagePart::Escape(Escape { escaped_char: char }));
          start = self.next_byte_index();
        }
        '{' => {
          if byte_index != start {
            parts.push(MessagePart::Text(Text {
              content: &self.input[start..byte_index],
            }))
          }
          parts.push(self.parse_placeholder());
          start = self.next_byte_index();
        }
        '.' | '@' | '|' => {
          self.next();
        }
        c if is_content_char(c) || is_space(c) => {
          self.next();
        }
        _ => panic!("Unexpected character: {:?}", c),
      }
    }

    let end = self.next_byte_index();
    if end != start {
      parts.push(MessagePart::Text(Text {
        content: &self.input[start..end],
      }))
    }

    SimpleMessage { parts }
  }

  fn parse_placeholder(&mut self) -> MessagePart<'a> {
    let n = self.next();
    debug_assert!(matches!(n, Some((_, '{'))));

    self.skip_spaces();

    let expr = match self.peek() {
      Some((_, '$')) => {
        let variable = self.parse_variable(); // eats the $
        let mut had_space = self.skip_spaces();

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
            value = Some(LiteralOrVariable::Variable(self.parse_variable()));
            had_space = self.skip_spaces();
          }

          attributes.push(Attribute { key, value });
        }

        MessagePart::Expression(Expression::VariableExpression(
          VariableExpression {
            variable,
            annotation,
            attributes,
          },
        ))
      }
      _ => unimplemented!(),
    };

    if self.eat('}').is_none() {
      panic!()
    }

    expr
  }

  fn parse_variable(&mut self) -> Variable<'a> {
    let n = self.next();
    debug_assert_eq!(n.unwrap().1, '$');

    let name = self.parse_name();

    Variable { name }
  }

  fn parse_identifier(&mut self) -> Identifier<'a> {
    let name_or_namespace = self.parse_name();

    if self.eat(':').is_some() {
      let name = self.parse_name();

      Identifier {
        namespace: Some(name_or_namespace),
        name,
      }
    } else {
      Identifier {
        namespace: None,
        name: name_or_namespace,
      }
    }
  }

  fn parse_name(&mut self) -> &'a str {
    let start = self.next_byte_index();

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

    &self.input[start..self.next_byte_index()]
  }

  fn next(&mut self) -> Option<(usize, char)> {
    self.chars.next()
  }

  fn peek(&mut self) -> Option<(usize, char)> {
    self.chars.peek()
  }

  fn eat(&mut self, c: char) -> Option<usize> {
    if let Some((index, ch)) = self.peek() {
      if ch == c {
        self.next();
        return Some(index);
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

        Some(Annotation::Function(Function {
          id,
          options: vec![],
        }))
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

  fn parse_reserved_body(&mut self) -> Vec<ReservedBodyPart<'a>> {
    let mut parts = vec![];

    let mut start = self.next_byte_index();
    let mut last_space_start = None;

    while let Some((byte_index, c)) = self.peek() {
      match c {
        c if is_reserved_char(c) => {
          self.next();
          last_space_start = None;
        }
        c if is_space(c) => {
          self.next();
          if last_space_start.is_none() {
            last_space_start = Some(byte_index);
          }
        }
        '\\' => {
          if byte_index != start {
            parts.push(ReservedBodyPart::Text(Text {
              content: &self.input[start..byte_index],
            }))
          }
          self.next(); // consume '\\'
          let Some((_, char)) = self.next() else {
            panic!("Unexpected end of input")
          };
          parts.push(ReservedBodyPart::Escape(Escape { escaped_char: char }));
          start = self.next_byte_index();
          last_space_start = None;
        }
        '|' => {
          if byte_index != start {
            parts.push(ReservedBodyPart::Text(Text {
              content: &self.input[start..byte_index],
            }))
          }
          self.next(); // consume '|'
          unimplemented!("quoted");
          start = self.next_byte_index();
          last_space_start = None;
        }
        _ => break,
      }
    }

    if let Some(start) = last_space_start {
      self.chars.reset_to(start);
    }

    let byte_index = self.next_byte_index();
    if byte_index != start {
      parts.push(ReservedBodyPart::Text(Text {
        content: &self.input[start..byte_index],
      }))
    }

    parts
  }

  fn parse_literal(&mut self) -> () {
    match self.peek() {
      '|' => Literal(self.parse_quoted()),
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
  is_content_char(c) || c == '@' || c == '|' // simple-start-char
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
