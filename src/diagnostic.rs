use std::fmt;

use crate::ast::Identifier;
use crate::ast::Number;
use crate::Location;
use crate::Span;
use crate::Spanned as _;

macro_rules! diagnostics {
  (
    pub enum $name:ident<$life:lifetime> {
      $($variant:ident { $($field:ident: $ty:ty),* } => ($($arg:expr),*$(,)?)), *$(,)?
    }
  ) => {
    pub enum $name<$life> {
      $($variant { $($field: $ty),* }),*
    }

    impl<$life> fmt::Display for $name<$life> {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          $(Self::$variant { $($field),* } => write!(f, $($arg,)*),)*
        }
      }
    }
  };
}

diagnostics! {
  pub enum Diagnostic<'a> {
    NumberMissingIntegralPart { number: Number<'a> } => (
      "Number is missing an integral part (at {:?})",
      number.span()
    ),
    NumberLeadingZeroIntegralPart { number: Number<'a> } => (
      "Number has a leading zero in the integral part (at {:?})",
      number.span()
    ),
    NumberMissingFractionalPart { number: Number<'a> }=> (
      "Number is missing a fractional part (at {:?})",
      number.span()
    ),
    NumberMissingExponentPart { number: Number<'a> } => (
      "Number is missing an exponent part (at {:?})",
      number.span()
    ),
    UnterminatedQuoted { span: Span } => (
      "Quoted string is missing a closing quote (at {:?})",
      span
    ),
    PlaceholderMissingClosingBrace { span: Span } => (
      "Placeholder is missing a closing brace (at {:?})",
      span
    ),
    MissingIdentifierName { identifier: Identifier<'a> } => (
      "Identifier is missing a name (at {:?})",
      identifier.span()
    ),
    MissingIdentifierNamespace { identifier: Identifier<'a> } => (
      "Identifier is missing a namespace (at {:?})",
      identifier.span()
    ),
    EscapeInvalidCharacter { char_loc: Location, char: char } => (
      "Escape sequence can only escape '}}', '{{', and '|' (found {:?} at {:?})",
      char,
      char_loc
    ),
    EscapeMissingCharacter { slash: Location } => (
      "Escape sequence is missing a character to escape (at {:?})",
      slash
    ),
  }
}

impl fmt::Debug for Diagnostic<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self)
  }
}
