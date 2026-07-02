use std::fmt;

use crate::Location;
use crate::Span;
use crate::Spanned as _;
use crate::ast::Attribute;
use crate::ast::Expression;
use crate::ast::FnOrMarkupOption;
use crate::ast::Identifier;
use crate::ast::Number;

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
      fixes: [
      ],
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
