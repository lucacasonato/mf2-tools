use std::fmt;

use crate::ast::Attribute;
use crate::ast::Expression;
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

      pub fn message(&self) -> String {
        match self {
          $(Self::$variant { $($field),* } => format!($($message,)*),)*
        }
      }
    }

    impl fmt::Display for $name<'_> {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at {:?})", self.message(), self.span())
      }
    }
  };
}

diagnostics! {
  pub enum Diagnostic<'text> {
    NumberMissingIntegralPart { number: Number<'text> } => {
      message: ("Number is missing an integral part."),
      span: number.span(),
    },
    NumberLeadingZeroIntegralPart { number: Number<'text> } => {
      message: ("Number has leading zero in integral part, which is not allowed."),
      span: number.span(),
    },
    NumberMissingFractionalPart { number: Number<'text> } => {
      message: ("Number is missing a fractional part, which it must have because it has a decimal point."),
      span: number.span(),
    },
    NumberMissingExponentPart { number: Number<'text> } => {
      message: ("Number is missing an exponent part, which it must have because it is written in scientific notation."),
      span: number.span(),
    },
    OptionMissingKey { span: Span } => {
      message: ("Found equals sign followed by value, but equals sign is not preceeded by a key. Did you forget to add a key to make this an option?"),
      span: *span,
    },
    OptionMissingValue { span: Span } => {
      message: ("Found an identifier followed by an equals sign, but not followed by a value. Did you forget to add a value to make this an option?"),
      span: *span,
    },
    LoneEqualsSign { loc: Location } => {
      message: ("Found an equals sign without a key or value. Did you mean to add a key and value to make this an option?"),
      span: Span { start: *loc, end: *loc + '=' },
    },
    MarkupMissingIdentifier { span: Span } => {
      message: ("Markup tag is missing an identifier."),
      span: *span,
    },
    MarkupInvalidSpaceBeforeIdentifier { id: Identifier<'text>, start_loc: Location } => {
      message: ("Identifier of the markup tag is preceeded by spaces, which is not allowed."),
      span: Span { start: *start_loc + '#', end: id.span().start },
    },
    MarkupMissingClosingBrace { span: Span } => {
      message: ("Markup tag is not closed with a closing brace."),
      span: *span,
    },
    MarkupCloseInvalidSelfClose { self_close_loc: Location } => {
      message: ("Markup tag can not be self-closing if it is a close tag."),
      span: Span::new(*self_close_loc..(*self_close_loc + '/')),
    },
    MarkupInvalidSpaceBetweenSelfCloseAndBrace { space: Span } => {
      message: ("Self-closing tag of a markup tag can not have trailing spaces before the closing brace."),
      span: *space,
    },
    MarkupOptionAfterAttribute { previous_attribute: Attribute<'text>, option: FnOrMarkupOption<'text> } => {
      message: ("Markup tag has an option after an attribute, which is not allowed. All options must come before any attribute."),
      span: option.span(),
    },
    FunctionMissingIdentifier { span: Span } => {
      message: ("Function is missing an identifier."),
      span: *span,
    },
    UnterminatedQuoted { span: Span } => {
      message: ("Quoted string is missing the closing quote."),
      span: *span,
    },
    PlaceholderMissingClosingBrace { span: Span } => {
      message: ("Placeholder is missing the closing brace."),
      span: *span,
    },
    PlaceholderMissingBody { span: Span } => {
      message: ("Placeholder is empty, but should have at least a variable reference, literal, or annotation."),
      span: *span,
    },
    PlaceholderInvalidContents { span: Span } => {
      message: ("Placeholder expression contains invalid content."),
      span: *span,
    },
    QuotedPatternInsidePattern { open_span: Span, close_span: Option<Span> } => {
      message: ("Quoted pattern is not allowed inside of a pattern."),
      span: *open_span,
    },
    MarkupInvalidContents { span: Span } => {
      message: ("Markup tag contains invalid content."),
      span: *span,
    },
    MissingIdentifierName { identifier: Identifier<'text> } => {
      message: ("Namespaced identifier is missing a name, which is required after the colon following the namespace."),
      span: identifier.span(),
    },
    MissingIdentifierNamespace { identifier: Identifier<'text> } => {
      message: ("Identifiers with a colon before the name are namespaced identifiers, but this identifier is missing a namespace before the colon."),
      span: identifier.span(),
    },
    EscapeInvalidCharacter { char_loc: Location, char: char } => {
      message: ( "The character '{char}' can not be escaped as escape sequences can only escape '}}', '{{', '|', and '\\'."),
      span: Span::new(*char_loc..(*char_loc + *char)),
    },
    EscapeMissingCharacter { slash_loc: Location } => {
      message: ("Backslashes start an escape sequence, but no character to be escaped was found. A literal '\\' must be written as '\\\\'."),
      span: Span::new(*slash_loc..(*slash_loc + '\\')),
    },
    InvalidNullCharacter { char_loc: Location } => {
      message: ("The NULL character (0x00) is invalid anywhere inside of messages."),
      span: Span::new(*char_loc..(*char_loc + '\0')),
    },
    InvalidClosingBrace { brace_loc: Location } => {
      message: ("The closing brace character ('}}') is invalid inside of messages, and must be escaped as '\\}}'."),
      span: Span::new(*brace_loc..(*brace_loc + '}')),
    },
    AnnotationMissingSpaceBefore { span: Span } => {
      message: ("Annotation is missing a leading space."),
      span: *span,
    },
    AttributeMissingSpaceBefore { span: Span } => {
      message: ("Attribute is missing a leading space."),
      span: *span,
    },
    AttributeMissingKey { span: Span } => {
      message: ("Attribute is missing a key after the '@' sign."),
      span: *span,
    },
    AttributeMissingValue { span: Span } => {
      message: ("Attribute is missing a value after the '=' sign."),
      span: *span,
    },
    VariableMissingName { span: Span } => {
      message: ("Variable is missing a name after the dollar sign ('$')."),
      span: *span,
    },
    UnterminatedQuotedPattern { span: Span } => {
      message: ("Quoted pattern is missing the closing braces ('}}')."),
      span: *span,
    },
    LocalKeywordMissingTrailingSpace { span: Span } => {
      message: ("'.local' keyword is not followed by a space."),
      span: *span,
    },
    LocalVariableMissingDollar { span: Span } => {
      message: ("Variable is not prefixed with a dollar sign ('$')."),
      span: *span,
    },
    MissingSpaceBeforeMatcherKey { span: Span } => {
      message: ("Matcher key is missing a leading space."),
      span: *span,
    },
    ComplexMessageMissingBody { span: Span } => {
      message: ("Message is missing a body (a matcher or quoted pattern)."),
      span: *span,
    },
    ComplexMessageTrailingContent { span: Span } => {
      message: ("Message has additional invalid content after the body."),
      span: *span,
    },
    ComplexMessageBodyNotQuoted { span: Span } => {
      message: ("Using an unquoted pattern as the body is invalid, because the message contains declarations. Did you mean to quote the pattern?."),
      span: *span,
    },
    ComplexMessageDeclarationAfterBody { span: Span } => {
      message: ("Declarations are not valid after the message body. Did you mean to put the declaration before the body?"),
      span: *span,
    },
    ComplexMessageMultipleBodies { span: Span } => {
      message: ("Message has multiple bodies, but only one is allowed."),
      span: *span,
    },
    MatcherKeyIsVariable { span: Span } => {
      message: ("Matcher key is a variable, which is not allowed. Matcher keys must be literal values, or the wildcard ('*')."),
      span: *span,
    },
    InvalidMatcherLiteralKey { span: Span } => {
      message: ("Found an invalid matcher key (not a valid literal). Did you mean to quote the key to make it a literal?"),
      span: *span,
    },
    ReservedStatementMissingSpaceBeforeBody { span: Span } => {
      message: ("Reserved statement is missing a space before the contents."),
      span: *span,
    },
    ReservedStatementMissingExpression { span: Span } => {
      message: ("Reserved statement does not end with an expression, but it must."),
      span: *span,
    },
    LocalDeclarationValueNotWrappedInBraces { span: Span } => {
      message: ("Value of a local declaration is a literal or variable, but must be an expression. Did you mean to wrap the value in braces?."),
      span: *span,
    },
    LocalDeclarationVariableMissingTrailingEquals { span: Span } => {
      message: ("Local declaration is missing an equals sign after the variable."),
      span: *span,
    },
    LocalDeclarationMissingExpression { span: Span } => {
      message: ("Local declaration is missing an expression as the value after the equals sign."),
      span: *span,
    },
    InputDeclarationMissingExpression { span: Span } => {
      message: ("Input declaration is missing an expression."),
      span: *span,
    },
    InputDeclarationWithInvalidExpression { span: Span, expression: Expression<'text> } => {
      message: ("Input declaration has a non-variable expression, which is invalid. Did you mean to use a local declaration instead of an input declaration?"),
      span: *span,
    },
    MatcherMissingSelectors { span: Span } => {
      message: ("Matcher is missing a selector, but at least one is required."),
      span: *span,
    },
    MatcherVariantMissingKeys { span: Span } => {
      message: ("Matcher variant is missing key(s), but at least one is required."),
      span: *span,
    },
    MatcherVariantExpressionBodyNotQuoted { span: Span } => {
      message: ("Matcher variant has an expression as a body, but only quoted patterns are allowed. Did you mean to wrap the expression in a quoted pattern?"),
      span: *span,
    },
    MatcherVariantMissingBody { span: Span } => {
      message: ("Matcher variant is missing a body."),
      span: *span,
    },
  }
}

impl fmt::Debug for Diagnostic<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self)
  }
}
