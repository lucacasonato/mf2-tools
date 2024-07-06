use std::fmt;

use crate::ast::Attribute;
use crate::ast::FnOrMarkupOption;
use crate::ast::Identifier;
use crate::ast::Number;
use crate::Location;
use crate::Span;
use crate::Spanned as _;

macro_rules! diagnostics {
  (
    pub enum $name:ident<$life:lifetime> {
      $($variant:ident { $($field:ident: $ty:ty),* } => {
        message: ($($message:expr),*$(,)?) $(,)?
        span: $span:expr $(,)?
      }), *$(,)?
    }
  ) => {
    pub enum $name<$life> {
      $($variant { $($field: $ty),* }),*
    }

    #[allow(unused_variables)]
    impl<$life> $name<$life> {
      pub fn span(&self) -> Span {
        match self {
          $(Self::$variant { $($field,)* } => $span,)*
        }
      }
    }

    #[allow(unused_variables)]
    impl<$life> fmt::Display for $name<$life> {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          $(Self::$variant { $($field),* } => write!(f, $($message,)*),)*
        }
      }
    }
  };
}

diagnostics! {
  pub enum Diagnostic<'a> {
    NumberMissingIntegralPart { number: Number<'a> } => {
      message: ("Number is missing an integral part (at {:?})", number.span()),
      span: number.span(),
    },
    NumberLeadingZeroIntegralPart { number: Number<'a> } => {
      message: ("Number has a leading zero in the integral part (at {:?})", number.span()),
      span: number.span(),
    },
    NumberMissingFractionalPart { number: Number<'a> } => {
      message: ("Number is missing a fractional part (at {:?})", number.span()),
      span: number.span(),
    },
    NumberMissingExponentPart { number: Number<'a> } => {
      message: ("Number is missing an exponent part (at {:?})", number.span()),
      span: number.span(),
    },
    OptionMissingValue { span: Span } => {
      message: ("Option is missing a value, which is required (at {:?})", span),
      span: *span,
    },
    MarkupMissingClosingBrace { span: Span } => {
      message: ("Markup is missing a closing brace (at {:?})", span),
      span: *span,
    },
    MarkupCloseInvalidSelfClose { self_close_loc: Location } => {
      message: ("Markup has an invalid self-closing tag on a markup close (at {:?})", self_close_loc),
      span: Span::new(*self_close_loc..(*self_close_loc + '/')),
    },
    MarkupInvalidSpaceBetweenSelfCloseAndBrace { space: Span } => {
      message: ("Markup has invalid spaces between self-closing tag and closing brace (at {:?})", space),
      span: *space,
    },
    MarkupOptionAfterAttribute { previous_attribute: Attribute<'a>, option: FnOrMarkupOption<'a> } => {
      message: ("Markup has option after attribute (at {:?})", option.span()),
      span: option.span(),
    },
    UnterminatedQuoted { span: Span } => {
      message: ("Quoted string is missing a closing quote (at {:?})", span),
      span: *span,
    },
    PlaceholderMissingClosingBrace { span: Span } => {
      message: ("Placeholder is missing a closing brace (at {:?})", span),
      span: *span,
    },
    PlaceholderMissingBody { span: Span } => {
      message: ("Placeholder is missing a variable reference, literal, or annotation (at {:?})", span),
      span: *span,
    },
    MissingIdentifierName { identifier: Identifier<'a> } => {
      message: ("Identifier is missing a name (at {:?})", identifier.span()),
      span: identifier.span(),
    },
    MissingIdentifierNamespace { identifier: Identifier<'a> } => {
      message: ("Identifier is missing a namespace (at {:?})", identifier.span()),
      span: identifier.span(),
    },
    EscapeInvalidCharacter { char_loc: Location, char: char } => {
      message: ( "Escape sequence can only escape '}}', '{{', '|', and '\\' (found {char:?} at {char_loc:?})"),
      span: Span::new(*char_loc..(*char_loc + *char)),
    },
    EscapeMissingCharacter { slash_loc: Location } => {
      message: ("Escape sequence is missing a character to escape (at {:?})", slash_loc),
      span: Span::new(*slash_loc..(*slash_loc + '\\')),
    },
    InvalidNullCharacter { char_loc: Location } => {
      message: ("Invalid NULL (0x00) character (at {:?})", char_loc),
      span: Span::new(*char_loc..(*char_loc + '\0')),
    },
    InvalidClosingBrace { brace_loc: Location } => {
      message: ("'}}' in simple messages must be escaped (at {:?})", brace_loc),
      span: Span::new(*brace_loc..(*brace_loc + '}')),
    },
    AnnotationMissingSpaceBefore { span: Span } => {
      message: ("Annotations must be preceeded by a leading space (at {:?})", span),
      span: *span,
    },
    AttributeMissingSpaceBefore { span: Span } => {
      message: ("Attributes must be preceeded by a leading space (at {:?})", span),
      span: *span,
    },
    AttributeInvalidSpacesAfterAt { span: Span } => {
      message: ("'@' must be immediately followed by an identifier, without spaces (at {:?})", span),
      span: *span,
    },
    AttributeMissingValue { span: Span } => {
      message: ("Attribute is missing a value (at {:?})", span),
      span: *span,
    },
    ComplexMessageNotYetSupported { span: Span } => {
      message: ("Complex messages are not yet supported (at {:?})", *span),
      span: *span,
    }
  }
}

impl fmt::Debug for Diagnostic<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self)
  }
}
