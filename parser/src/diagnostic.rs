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
        fatal: $fatal:expr,
        fixes: [$({ label: $label:expr, fix($($this:tt $(, $info:tt)?)?) $fix:block }),* $(,)?] $(,)?
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

      /// Get a list of fixes that can be applied to the source text to resolve
      /// the diagnostic. Each fix has a label that describes the fix, and a list
      /// of edits that describe the changes to make to the source text if the
      /// fix is applied.
      pub fn fixes(&self, info: &crate::SourceTextInfo) -> Vec<DiagnosticFix> {
        match self {
          $(Self::$variant { $($field),* } => vec![$(DiagnosticFix {
            label: $label,
            edits: {
              $(let $this = self; $(let $info = info;)?)?
              $fix
            },
          }),*],)*
        }
      }
    }
  };
}

diagnostics! {
  pub enum Diagnostic<'text> {
    // Syntax Errors
    NumberMissingIntegralPart { number: Number<'text> } => {
      message: ("Number is missing an integral part."),
      span: number.span(),
      fatal: false,
      fixes: [{
        label: "Add 0 before the decimal point",
        fix() {
          vec![DiagnosticEdit {
            span: number.integral_span(),
            new_text: "0".to_string(),
          }]
        }
      }],
    },
    NumberLeadingZeroIntegralPart { number: Number<'text> } => {
      message: ("Number has leading zero in integral part, which is not allowed."),
      span: number.span(),
      fatal: false,
      fixes: [{
        label: "Remove leading zeros",
        fix() {
          let integral_span = number.integral_span();
          let integral = number.integral_part();
          let trimmed = integral.trim_start_matches('0');
          let trimmed = if trimmed.is_empty() { "0" } else { trimmed };
          vec![DiagnosticEdit {
            span: number.integral_span(),
            new_text: trimmed.to_owned(),
          }]
        }
      }],
    },
    NumberMissingFractionalPart { number: Number<'text> } => {
      message: ("Number is missing a fractional part, which it must have because it has a decimal point."),
      span: number.span(),
      fatal: false,
      fixes: [
        {
          label: "Add 0 after the decimal point",
          fix() {
            vec![DiagnosticEdit {
              span: number.fractional_span().unwrap(),
              new_text: "0".to_string(),
            }]
          }
        },
        {
          label: "Remove decimal point",
          fix() {
            vec![DiagnosticEdit {
              span: Span::new(number.integral_span().end..number.fractional_span().unwrap().start),
              new_text: "".to_string(),
            }]
          }
        }
      ],
    },
    NumberMissingExponentPart { number: Number<'text> } => {
      message: ("Number is missing an exponent part, which it must have because it is written in scientific notation."),
      span: number.span(),
      fatal: false,
      fixes: [{
        label: "Remove the 'e'",
        fix() {
          vec![DiagnosticEdit {
            span: Span::new(number.fractional_span().unwrap_or(number.integral_span()).end..number.exponent_span().unwrap().start),
            new_text: "".to_string(),
          }]
        }
      }],
    },
    OptionMissingKey { span: Span } => {
      message: ("Found equals sign followed by value, but equals sign is not preceeded by a key. Did you forget to add a key to make this an option?"),
      span: *span,
      fatal: false,
      fixes: [],
    },
    OptionMissingValue { span: Span } => {
      message: ("Found an identifier followed by an equals sign, but not followed by a value. Did you forget to add a value to make this an option?"),
      span: *span,
      fatal: true,
      fixes: [],
    },
    LoneEqualsSign { loc: Location } => {
      message: ("Found an equals sign without a key or value. Did you mean to add a key and value to make this an option?"),
      span: Span { start: *loc, end: *loc + '=' },
      fatal: true,
      fixes: [],
    },
    MarkupMissingIdentifier { span: Span } => {
      message: ("Markup tag is missing an identifier."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    MarkupInvalidSpaceBeforeIdentifier { id: Identifier<'text>, start_loc: Location } => {
      message: ("Identifier of the markup tag is preceeded by spaces, which is not allowed."),
      span: Span { start: *start_loc + '#', end: id.span().start },
      fatal: true,
      fixes: [{
        label: "Remove space before identifier",
        fix(this) {
          vec![DiagnosticEdit {
            span: this.span(),
            new_text: "".to_string(),
          }]
        }
      }],
    },
    MarkupMissingClosingBrace { span: Span } => {
      message: ("Markup tag is not closed with a closing brace."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    MarkupCloseInvalidSelfClose { self_close_loc: Location } => {
      message: ("Markup tag can not be self-closing if it is a close tag."),
      span: Span::new(*self_close_loc..(*self_close_loc + '/')),
      fatal: true,
      fixes: [{
        label: "Remove self-closing slash",
        fix(this) {
          vec![DiagnosticEdit {
            span: this.span(),
            new_text: "".to_string(),
          }]
        }
      }],
    },
    MarkupInvalidSpaceBetweenSelfCloseAndBrace { space: Span } => {
      message: ("Self-closing tag of a markup tag can not have trailing spaces before the closing brace."),
      span: *space,
      fatal: true,
      fixes: [{
        label: "Remove space before closing brace",
        fix() {
          vec![DiagnosticEdit {
            span: *space,
            new_text: "".to_string(),
          }]
        }
      }],
    },
    MarkupOptionAfterAttribute { previous_attribute: Attribute<'text>, option: FnOrMarkupOption<'text> } => {
      message: ("Markup tag has an option after an attribute, which is not allowed. All options must come before any attribute."),
      span: option.span(),
      fatal: false,
      fixes: [{
        label: "Move option before attribute",
        fix(_, info) {
          let start = previous_attribute.span().start;
          vec![
            DiagnosticEdit {
              span: option.span(),
              new_text: "".to_string(),
            },
            DiagnosticEdit {
              span: Span::new(start..start),
              new_text: format!("{} ", info.text(option.span())),
            }
          ]
        }
      }],
    },
    FunctionMissingIdentifier { span: Span } => {
      message: ("Function is missing an identifier."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    UnterminatedQuoted { span: Span } => {
      message: ("Quoted string is missing the closing quote."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    PlaceholderMissingClosingBrace { span: Span } => {
      message: ("Placeholder is missing the closing brace."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    PlaceholderMissingBody { span: Span } => {
      message: ("Placeholder is empty, but should have at least a variable reference, literal, or annotation."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    PlaceholderInvalidLiteral { span: Span } => {
      message: ("Placeholder expression contains a literal that is not valid when unquoted. Did you mean to quote it?"),
      span: *span,
      fatal: false,
      fixes: [{
        label: "Quote literal",
        fix() {
          vec![
            DiagnosticEdit {
              span: Span::new(span.start..span.start),
              new_text: "|".to_owned(),
            },
            DiagnosticEdit {
              span: Span::new(span.end..span.end),
              new_text: "|".to_owned(),
            }
          ]
        }
      }],
    },
    PlaceholderInvalidContents { span: Span } => {
      message: ("Placeholder expression contains invalid content."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    QuotedPatternInsidePattern { open_span: Span, close_span: Option<Span> } => {
      message: ("Quoted pattern is not allowed inside of a pattern."),
      span: *open_span,
      fatal: false,
      fixes: [{
        label: "Remove quotes",
        fix() {
          let mut fixes = vec![
            DiagnosticEdit {
              span: *open_span,
              new_text: "".to_string(),
            }
          ];
          if let Some(close_span) = close_span {
            fixes.push(DiagnosticEdit {
              span: *close_span,
              new_text: "".to_string(),
            });
          }
          fixes
        }
      }],
    },
    MarkupInvalidContents { span: Span } => {
      message: ("Markup tag contains invalid content."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    MissingIdentifierName { identifier: Identifier<'text> } => {
      message: ("Namespaced identifier is missing a name, which is required after the colon following the namespace."),
      span: identifier.span(),
      fatal: false,
      fixes: [],
    },
    MissingIdentifierNamespace { identifier: Identifier<'text> } => {
      message: ("Identifiers with a colon before the name are namespaced identifiers, but this identifier is missing a namespace before the colon."),
      span: identifier.span(),
      fatal: false,
      fixes: [],
    },
    EscapeInvalidCharacter { slash_loc: Location, char: char } => {
      message: ( "The character '{char}' can not be escaped, as escape sequences can only escape '}}', '{{', '|', and '\\'."),
      span: Span::new(*slash_loc..(*slash_loc + '\\' + *char)),
      fatal: false,
      fixes: [{
        label: "Remove backslash",
        fix() {
          vec![DiagnosticEdit {
            span: Span::new(*slash_loc..*slash_loc + '\\'),
            new_text: "".to_string(),
          }]
        }
      }],
    },
    EscapeMissingCharacter { slash_loc: Location } => {
      message: ("Backslashes start an escape sequence, but no character to be escaped was found. A literal '\\' must be written as '\\\\'."),
      span: Span::new(*slash_loc..(*slash_loc + '\\')),
      fatal: true,
      fixes: [],
    },
    InvalidNullCharacter { char_loc: Location } => {
      message: ("The NULL character (0x00) is invalid anywhere inside of messages."),
      span: Span::new(*char_loc..(*char_loc + '\0')),
      fatal: false,
      fixes: [{
        label: "Remove NULL character",
        fix(this) {
          vec![DiagnosticEdit {
            span: this.span(),
            new_text: "".to_string(),
          }]
        }
      }],
    },
    InvalidClosingBrace { brace_loc: Location } => {
      message: ("The closing brace character ('}}') is invalid inside of messages, and must be escaped as '\\}}'."),
      span: Span::new(*brace_loc..(*brace_loc + '}')),
      fatal: false,
      fixes: [{
        label: "Escape the brace",
        fix() {
          vec![DiagnosticEdit {
            span: Span::new(*brace_loc..*brace_loc),
            new_text: "\\".to_string(),
          }]
        }
      }],
    },
    AnnotationMissingSpaceBefore { span: Span } => {
      message: ("Annotation is missing a leading space."),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Add space before annotation",
        fix() {
          vec![DiagnosticEdit {
            span: Span::new(span.start..span.start),
            new_text: " ".to_owned(),
          }]
        }
      }],
    },
    AttributeMissingSpaceBefore { span: Span } => {
      message: ("Attribute is missing a leading space."),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Add space before attribute",
        fix() {
          vec![DiagnosticEdit {
            span: Span::new(span.start..span.start),
            new_text: " ".to_owned(),
          }]
        }
      }],
    },
    AttributeMissingKey { span: Span } => {
      message: ("Attribute is missing a key after the '@' sign."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    AttributeMissingValue { span: Span } => {
      message: ("Attribute is missing a value after the '=' sign."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    AttributeValueIsVariable { span: Span } => {
      message: ("Attribute value can not be a variable, but must be a literal value."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    VariableMissingName { span: Span } => {
      message: ("Variable is missing a name after the dollar sign ('$')."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    UnterminatedQuotedPattern { span: Span } => {
      message: ("Quoted pattern is missing the closing braces ('}}}}')."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    LocalKeywordMissingTrailingSpace { span: Span } => {
      message: ("'.local' keyword is not followed by a space."),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Add space after '.local'",
        fix() {
          let start = span.start + ".local";
          vec![DiagnosticEdit {
            span: Span::new(start..start),
            new_text: " ".to_owned(),
          }]
        }
      }],
    },
    LocalVariableMissingDollar { span: Span } => {
      message: ("Variable is not prefixed with a dollar sign ('$')."),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Add dollar sign",
        fix() {
          let start = span.start;
          vec![DiagnosticEdit {
            span: Span::new(start..start),
            new_text: "$".to_owned(),
          }]
        }
      }],
    },
    MissingSpaceBeforeMatcherSelector { span: Span } => {
      message: ("Matcher selector is missing a leading space."),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Add space before selector",
        fix() {
          vec![DiagnosticEdit {
            span: Span::new(span.start..span.start),
            new_text: " ".to_owned(),
          }]
        }
      }],
    },
    MissingSpaceBeforeMatcherKey { span: Span } => {
      message: ("Matcher key is missing a leading space."),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Add space before key",
        fix() {
          vec![DiagnosticEdit {
            span: Span::new(span.start..span.start),
            new_text: " ".to_owned(),
          }]
        }
      }],
    },
    ComplexMessageMissingBody { span: Span } => {
      message: ("Message is missing a body (a matcher or quoted pattern)."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    ComplexMessageTrailingContent { span: Span } => {
      message: ("Message has additional invalid content after the body."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    ComplexMessageBodyNotQuoted { span: Span } => {
      message: ("Using an unquoted pattern as the body is invalid, because the message contains declarations. Did you mean to quote the pattern?."),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Surround with quotes",
        fix() {
          vec![
            DiagnosticEdit {
              span: Span::new(span.start..span.start),
              new_text: "{{".to_owned(),
            },
            DiagnosticEdit {
              span: Span::new(span.end..span.end),
              new_text: "}}".to_owned(),
            }
          ]
        }
      }],
    },
    ComplexMessageDeclarationAfterBody { span: Span, body_start: Location } => {
      message: ("Declarations are not valid after the message body. Did you mean to put the declaration before the body?"),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Move declaration before body",
        fix(_, info) {
          vec![
            DiagnosticEdit {
              span: Span::new(*body_start..*body_start),
              new_text: format!("{}\n", info.text(*span))
            },
            DiagnosticEdit {
              span: *span,
              new_text: "".to_owned()
            }
          ]
        }
      }],
    },
    ComplexMessageMultipleBodies { span: Span } => {
      message: ("Message has multiple bodies, but only one is allowed."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    MatcherKeyIsVariable { span: Span } => {
      message: ("Matcher key is a variable, which is not allowed. Matcher keys must be literal values, or the wildcard ('*')."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    InvalidMatcherLiteralKey { span: Span } => {
      message: ("Found an invalid matcher key (not a valid literal). Did you mean to quote the key to make it a literal?"),
      span: *span,
      fatal: false,
      fixes: [{
        label: "Quote literal",
        fix() {
          vec![
            DiagnosticEdit {
              span: Span::new(span.start..span.start),
              new_text: "|".to_owned(),
            },
            DiagnosticEdit {
              span: Span::new(span.end..span.end),
              new_text: "|".to_owned(),
            }
          ]
        }
      }],
    },
    InvalidStatement { span: Span, keyword: &'text str } => {
      message: ("Found a statement that is invalid because the keyword '{keyword}' is unrecognized."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    LocalDeclarationMalformed { span: Span } => {
      message: ("Found a local declaration that is missing or malformed name."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    LocalDeclarationValueNotWrappedInBraces { span: Span } => {
      message: ("Value of a local declaration is a literal or variable, but must be an expression. Did you mean to wrap the value in braces?"),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Wrap value in braces",
        fix() {
          vec![
            DiagnosticEdit {
              span: Span::new(span.start..span.start),
              new_text: "{".to_owned()
            },
            DiagnosticEdit {
              span: Span::new(span.end..span.end),
              new_text: "}".to_owned()
            }
          ]
        }
      }],
    },
    LocalDeclarationVariableMissingTrailingEquals { span: Span } => {
      message: ("Local declaration is missing an equals sign after the variable."),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Add equals after variable",
        fix() {
          vec![DiagnosticEdit {
            span: Span::new(span.end..span.end),
            new_text: " =".to_owned()
          }]
        }
      }],
    },
    LocalDeclarationMissingExpression { span: Span } => {
      message: ("Local declaration is missing an expression as the value after the equals sign."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    InputDeclarationMissingExpression { span: Span } => {
      message: ("Input declaration is missing an expression."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    InputDeclarationWithInvalidExpression { span: Span, expression: Expression<'text> } => {
      message: ("Input declaration has a non-variable expression, which is invalid. Did you mean to use a local declaration instead of an input declaration?"),
      span: *span,
      fatal: true,
      fixes: [],
    },
    MatcherMissingSelectors { span: Span } => {
      message: ("Matcher is missing a selector, but at least one is required."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    MatcherVariantKeysMismatch { span: Span, selectors: usize, keys: usize } => {
      message: ("Matcher variant has {keys} keys, but there are {selectors} selectors."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    MatcherVariantMissingKeys { span: Span } => {
      message: ("Matcher variant is missing key(s), but at least one is required."),
      span: *span,
      fatal: false,
      fixes: [],
    },
    MatcherVariantExpressionBodyNotQuoted { span: Span } => {
      message: ("Matcher variant has an expression as a body, but only quoted patterns are allowed. Did you mean to wrap the expression in a quoted pattern?"),
      span: *span,
      fatal: true,
      fixes: [{
        label: "Quote the expression",
        fix() {
          vec![
            DiagnosticEdit {
              span: Span::new(span.start..span.start),
              new_text: "{{".to_owned()
            },
            DiagnosticEdit {
              span: Span::new(span.end..span.end),
              new_text: "}}".to_owned()
            }
          ]
        }
      }],
    },
    MatcherVariantMissingBody { span: Span } => {
      message: ("Matcher variant is missing a body."),
      span: *span,
      fatal: true,
      fixes: [],
    },
    MatcherMissingFallback { span: Span } => {
      message: ("Matcher is missing a catch-all variant, where all keys are *."),
      span: *span,
      fatal: false,
      fixes: [],
    },

    // Scope Erorrs
    DuplicateDeclaration { first_span: Span, second_span: Span, name: &'text str } => {
      message: ("${name} has already been declared."),
      span: *second_span,
      fatal: false,
      fixes: [],
    },
    UsageBeforeDeclaration { declaration_span: Span, usage_span: Span, name: &'text str } => {
      message: ("${name} is used before it is declared."),
      span: *usage_span,
      fatal: false,
      fixes: [],
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

pub struct DiagnosticFix {
  pub label: &'static str,
  pub edits: Vec<DiagnosticEdit>,
}

pub struct DiagnosticEdit {
  pub span: Span,
  pub new_text: String,
}
