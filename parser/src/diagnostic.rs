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
    OptionMissingKey { span: Span } => {
      message: ("Option is missing a key (at {:?})", span),
      span: *span,
    },
    OptionMissingValue { span: Span } => {
      message: ("Option is missing a value, which is required (at {:?})", span),
      span: *span,
    },
    MarkupMissingIdentifier { span: Span } => {
      message: ("Markup is missing an identifier (at {:?})", span),
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
    FunctionMissingIdentifier { span: Span } => {
      message: ("Function is missing an identifier (at {:?})", span),
      span: *span,
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
    PlaceholderInvalidContents { span: Span } => {
      message: ("Unrecognized placeholder contents (at {:?})", span),
      span: *span,
    },
    MarkupInvalidContents { span: Span } => {
      message: ("Unrecognized markup contents (at {:?})", span),
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
      message: ("'}}' in patterns must be escaped (at {:?})", brace_loc),
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
    AttributeMissingKey { span: Span } => {
      message: ("Attribute is missing a key (at {:?})", span),
      span: *span,
    },
    AttributeMissingValue { span: Span } => {
      message: ("Attribute is missing a value (at {:?})", span),
      span: *span,
    },
    ComplexMessageNotYetSupported { span: Span } => {
      message: ("Complex messages are not yet supported (at {:?})", *span),
      span: *span,
    },
    VariableMissingName { span: Span } => {
      message: ("Variable is missing a name (at {:?})", span),
      span: *span,
    },
    UnterminatedQuotedPattern { span: Span } => {
      message: ("Quoted pattern is missing closing braces (at {:?})", span),
      span: *span,
    },
    LocalKeywordMissingTrailingSpace { span: Span } => {
      message: ("'.local' keyword must be followed by a space (at {:?})", span),
      span: *span,
    },
    LocalVariableMissingDollar { span: Span } => {
      message: ("Variables must be prefixed with a dollar sign (at {:?})", span),
      span: *span,
    },
    MissingSpaceBeforeKey { span: Span } => {
      message: ("Key is missing a leading space (at {:?})", span),
      span: *span,
    },
    ComplexMessageMissingBody { span: Span } => {
      message: ("Complex message is missing a body (at {:?})", span),
      span: *span,
    },
    ComplexMessageTrailingContent { span: Span } => {
      message: ("Complex message has content after it's body (at {:?})", span),
      span: *span,
    },
    ComplexMessageBodyNotQuoted { span: Span } => {
      message: ("Complex message body must be quoted (at {:?})", span),
      span: *span,
    },
    ComplexMessageDeclarationAfterBody { span: Span } => {
      message: ("Declarations must occur before the body, but was found after the body (at {:?})", span),
      span: *span,
    },
    ComplexMessageMultipleBodies { span: Span } => {
      message: ("Complex message can have only one body (at {:?})", span),
      span: *span,
    },
    MatcherKeyIsVariable { span: Span } => {
      message: ("Matcher key cannot be a variable (at {:?})", span),
      span: *span,
    },
  }
}

impl fmt::Debug for Diagnostic<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self)
  }
}
