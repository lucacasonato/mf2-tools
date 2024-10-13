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
        message: ($($message:expr),*$(,)?),
        span: $span:expr,
        fatal: $fatal:expr $(,)?
      }), *$(,)?
    }
  ) => {
    /// Diagnostics that can be produced by the parser. Each diagnostic has a
    /// message that describes the error, and a span that indicates the location
    /// in the source text where the error occurred.
    ///
    /// Fatal diagnostics indicate that the parser was unable to recover from
    /// the error, and the AST may be incomplete or incorrect. Non-fatal
    /// diagnostics indicate that the parser was able to recover from the error,
    /// and the AST still fully represents the input text, but the AST may still
    /// be invalid in some way (like escaping a character that can not be
    /// escaped).
    pub enum $name<$life> {
      $($variant { $($field: $ty),* }),*
    }

    #[allow(unused_variables)]
    impl<$life> $name<$life> {
      /// Get the span of the diagnostic.
      pub fn span(&self) -> Span {
        match self {
          $(Self::$variant { $($field,)* } => $span,)*
        }
      }

      /// Get a human-readable message describing the diagnostic.
      pub fn message(&self) -> String {
        match self {
          $(Self::$variant { $($field),* } => format!($($message,)*),)*
        }
      }

      /// Check if the diagnostic is fatal. Fatal diagnostics indicate that the
      /// parser was unable to recover from the error, and the AST may be
      /// incomplete or incorrect.
      ///
      /// Non-fatal diagnostics indicate that the parser was able to recover
      /// from the error, and the AST still fully represents the input text, but
      /// the AST may still be invalid in some way (like escaping a character
      /// that can not be escaped).
      pub fn fatal(&self) -> bool {
        match self {
          $(Self::$variant { .. } => $fatal,)*
        }
      }
    }
  };
}

diagnostics! {
  pub enum Diagnostic<'text> {
    NumberMissingIntegralPart { number: Number<'text> } => {
      message: ("Number is missing an integral part."),
      span: number.span(),
      fatal: false,
    },
    NumberLeadingZeroIntegralPart { number: Number<'text> } => {
      message: ("Number has leading zero in integral part, which is not allowed."),
      span: number.span(),
      fatal: false,
    },
    NumberMissingFractionalPart { number: Number<'text> } => {
      message: ("Number is missing a fractional part, which it must have because it has a decimal point."),
      span: number.span(),
      fatal: false,
    },
    NumberMissingExponentPart { number: Number<'text> } => {
      message: ("Number is missing an exponent part, which it must have because it is written in scientific notation."),
      span: number.span(),
      fatal: false,
    },
    OptionMissingKey { span: Span } => {
      message: ("Found equals sign followed by value, but equals sign is not preceeded by a key. Did you forget to add a key to make this an option?"),
      span: *span,
      fatal: false,
    },
    OptionMissingValue { span: Span } => {
      message: ("Found an identifier followed by an equals sign, but not followed by a value. Did you forget to add a value to make this an option?"),
      span: *span,
      fatal: true,
    },
    LoneEqualsSign { loc: Location } => {
      message: ("Found an equals sign without a key or value. Did you mean to add a key and value to make this an option?"),
      span: Span { start: *loc, end: *loc + '=' },
      fatal: true,
    },
    MarkupMissingIdentifier { span: Span } => {
      message: ("Markup tag is missing an identifier."),
      span: *span,
      fatal: false,
    },
    MarkupInvalidSpaceBeforeIdentifier { id: Identifier<'text>, start_loc: Location } => {
      message: ("Identifier of the markup tag is preceeded by spaces, which is not allowed."),
      span: Span { start: *start_loc + '#', end: id.span().start },
      fatal: true,
    },
    MarkupMissingClosingBrace { span: Span } => {
      message: ("Markup tag is not closed with a closing brace."),
      span: *span,
      fatal: true,
    },
    MarkupCloseInvalidSelfClose { self_close_loc: Location } => {
      message: ("Markup tag can not be self-closing if it is a close tag."),
      span: Span::new(*self_close_loc..(*self_close_loc + '/')),
      fatal: true,
    },
    MarkupInvalidSpaceBetweenSelfCloseAndBrace { space: Span } => {
      message: ("Self-closing tag of a markup tag can not have trailing spaces before the closing brace."),
      span: *space,
      fatal: true,
    },
    MarkupOptionAfterAttribute { previous_attribute: Attribute<'text>, option: FnOrMarkupOption<'text> } => {
      message: ("Markup tag has an option after an attribute, which is not allowed. All options must come before any attribute."),
      span: option.span(),
      fatal: false,
    },
    FunctionMissingIdentifier { span: Span } => {
      message: ("Function is missing an identifier."),
      span: *span,
      fatal: false,
    },
    UnterminatedQuoted { span: Span } => {
      message: ("Quoted string is missing the closing quote."),
      span: *span,
      fatal: true,
    },
    PlaceholderMissingClosingBrace { span: Span } => {
      message: ("Placeholder is missing the closing brace."),
      span: *span,
      fatal: true,
    },
    PlaceholderMissingBody { span: Span } => {
      message: ("Placeholder is empty, but should have at least a variable reference, literal, or annotation."),
      span: *span,
      fatal: false,
    },
    PlaceholderInvalidLiteral { span: Span } => {
      message: ("Placeholder expression contains a literal that is not valid when unquoted. Did you mean to quote it?"),
      span: *span,
      fatal: false,
    },
    PlaceholderInvalidContents { span: Span } => {
      message: ("Placeholder expression contains invalid content."),
      span: *span,
      fatal: true,
    },
    QuotedPatternInsidePattern { open_span: Span, close_span: Option<Span> } => {
      message: ("Quoted pattern is not allowed inside of a pattern."),
      span: *open_span,
      fatal: false,
    },
    MarkupInvalidContents { span: Span } => {
      message: ("Markup tag contains invalid content."),
      span: *span,
      fatal: true,
    },
    MissingIdentifierName { identifier: Identifier<'text> } => {
      message: ("Namespaced identifier is missing a name, which is required after the colon following the namespace."),
      span: identifier.span(),
      fatal: false,
    },
    MissingIdentifierNamespace { identifier: Identifier<'text> } => {
      message: ("Identifiers with a colon before the name are namespaced identifiers, but this identifier is missing a namespace before the colon."),
      span: identifier.span(),
      fatal: false,
    },
    EscapeInvalidCharacter { char_loc: Location, char: char } => {
      message: ( "The character '{char}' can not be escaped as escape sequences can only escape '}}', '{{', '|', and '\\'."),
      span: Span::new(*char_loc..(*char_loc + *char)),
      fatal: false,
    },
    EscapeMissingCharacter { slash_loc: Location } => {
      message: ("Backslashes start an escape sequence, but no character to be escaped was found. A literal '\\' must be written as '\\\\'."),
      span: Span::new(*slash_loc..(*slash_loc + '\\')),
      fatal: true,
    },
    InvalidNullCharacter { char_loc: Location } => {
      message: ("The NULL character (0x00) is invalid anywhere inside of messages."),
      span: Span::new(*char_loc..(*char_loc + '\0')),
      fatal: false,
    },
    InvalidClosingBrace { brace_loc: Location } => {
      message: ("The closing brace character ('}}') is invalid inside of messages, and must be escaped as '\\}}'."),
      span: Span::new(*brace_loc..(*brace_loc + '}')),
      fatal: false,
    },
    AnnotationMissingSpaceBefore { span: Span } => {
      message: ("Annotation is missing a leading space."),
      span: *span,
      fatal: true,
    },
    AttributeMissingSpaceBefore { span: Span } => {
      message: ("Attribute is missing a leading space."),
      span: *span,
      fatal: true,
    },
    AttributeMissingKey { span: Span } => {
      message: ("Attribute is missing a key after the '@' sign."),
      span: *span,
      fatal: false,
    },
    AttributeMissingValue { span: Span } => {
      message: ("Attribute is missing a value after the '=' sign."),
      span: *span,
      fatal: true,
    },
    AttributeValueIsVariable { span: Span } => {
      message: ("Attribute value can not be a variable, but must be a literal value."),
      span: *span,
      fatal: false,
    },
    VariableMissingName { span: Span } => {
      message: ("Variable is missing a name after the dollar sign ('$')."),
      span: *span,
      fatal: false,
    },
    UnterminatedQuotedPattern { span: Span } => {
      message: ("Quoted pattern is missing the closing braces ('}}')."),
      span: *span,
      fatal: true,
    },
    LocalKeywordMissingTrailingSpace { span: Span } => {
      message: ("'.local' keyword is not followed by a space."),
      span: *span,
      fatal: true,
    },
    LocalVariableMissingDollar { span: Span } => {
      message: ("Variable is not prefixed with a dollar sign ('$')."),
      span: *span,
      fatal: true,
    },
    MissingSpaceBeforeMatcherSelector { span: Span } => {
      message: ("Matcher selector is missing a leading space."),
      span: *span,
      fatal: true,
    },
    MissingSpaceBeforeMatcherKey { span: Span } => {
      message: ("Matcher key is missing a leading space."),
      span: *span,
      fatal: true,
    },
    ComplexMessageMissingBody { span: Span } => {
      message: ("Message is missing a body (a matcher or quoted pattern)."),
      span: *span,
      fatal: true,
    },
    ComplexMessageTrailingContent { span: Span } => {
      message: ("Message has additional invalid content after the body."),
      span: *span,
      fatal: true,
    },
    ComplexMessageBodyNotQuoted { span: Span } => {
      message: ("Using an unquoted pattern as the body is invalid, because the message contains declarations. Did you mean to quote the pattern?."),
      span: *span,
      fatal: true,
    },
    ComplexMessageDeclarationAfterBody { span: Span } => {
      message: ("Declarations are not valid after the message body. Did you mean to put the declaration before the body?"),
      span: *span,
      fatal: true,
    },
    ComplexMessageMultipleBodies { span: Span } => {
      message: ("Message has multiple bodies, but only one is allowed."),
      span: *span,
      fatal: true,
    },
    MatcherKeyIsVariable { span: Span } => {
      message: ("Matcher key is a variable, which is not allowed. Matcher keys must be literal values, or the wildcard ('*')."),
      span: *span,
      fatal: false,
    },
    InvalidMatcherLiteralKey { span: Span } => {
      message: ("Found an invalid matcher key (not a valid literal). Did you mean to quote the key to make it a literal?"),
      span: *span,
      fatal: false,
    },
    InvalidStatement { span: Span, keyword: &'text str } => {
      message: ("Found a statement that is invalid because the keyword '{keyword}' is keyword."),
      span: *span,
      fatal: true,
    },
    LocalDeclarationMalformed { span: Span } => {
      message: ("Found a local declaration that is missing or malformed name."),
      span: *span,
      fatal: true,
    },
    LocalDeclarationValueNotWrappedInBraces { span: Span } => {
      message: ("Value of a local declaration is a literal or variable, but must be an expression. Did you mean to wrap the value in braces?."),
      span: *span,
      fatal: true,
    },
    LocalDeclarationVariableMissingTrailingEquals { span: Span } => {
      message: ("Local declaration is missing an equals sign after the variable."),
      span: *span,
      fatal: true,
    },
    LocalDeclarationMissingExpression { span: Span } => {
      message: ("Local declaration is missing an expression as the value after the equals sign."),
      span: *span,
      fatal: true,
    },
    InputDeclarationMissingExpression { span: Span } => {
      message: ("Input declaration is missing an expression."),
      span: *span,
      fatal: true,
    },
    InputDeclarationWithInvalidExpression { span: Span, expression: Expression<'text> } => {
      message: ("Input declaration has a non-variable expression, which is invalid. Did you mean to use a local declaration instead of an input declaration?"),
      span: *span,
      fatal: true,
    },
    MatcherMissingSelectors { span: Span } => {
      message: ("Matcher is missing a selector, but at least one is required."),
      span: *span,
      fatal: false,
    },
    MatcherVariantMissingKeys { span: Span } => {
      message: ("Matcher variant is missing key(s), but at least one is required."),
      span: *span,
      fatal: true,
    },
    MatcherVariantExpressionBodyNotQuoted { span: Span } => {
      message: ("Matcher variant has an expression as a body, but only quoted patterns are allowed. Did you mean to wrap the expression in a quoted pattern?"),
      span: *span,
      fatal: true,
    },
    MatcherVariantMissingBody { span: Span } => {
      message: ("Matcher variant is missing a body."),
      span: *span,
      fatal: true,
    },
  }
}

impl fmt::Display for Diagnostic<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} (at {:?})", self.message(), self.span())
  }
}

impl fmt::Debug for Diagnostic<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self)
  }
}
