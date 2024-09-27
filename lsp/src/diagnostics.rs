use crate::document::Document;
use crate::scope::ScopeDiagnostic;
use lsp_types::Diagnostic as LspDiagnostic;
use mf2_parser::Span;
use std::fmt;

pub enum Diagnostic<'t> {
  Parser(mf2_parser::Diagnostic<'t>),
  Scope(ScopeDiagnostic<'t>),
}

#[allow(unused_variables)]
impl<'text> Diagnostic<'text> {
  pub fn span(&self) -> Span {
    use ScopeDiagnostic::*;

    match self {
      Self::Parser(d) => d.span(),
      Self::Scope(DuplicateDeclaration { second_span, .. }) => *second_span,
      Self::Scope(UsageBeforeDeclaration { usage_span, .. }) => *usage_span,
    }
  }

  pub fn message(&self) -> String {
    use ScopeDiagnostic::*;

    match self {
      Self::Parser(d) => d.message(),
      Self::Scope(DuplicateDeclaration { name, .. }) => {
        format!("${name} has already been declared.")
      }
      Self::Scope(UsageBeforeDeclaration { name, .. }) => {
        format!("${name} is used before it is declared.")
      }
    }
  }

  pub fn to_lsp(&self, doc: &Document) -> LspDiagnostic {
    use mf2_parser::Diagnostic::*;

    match self {
      Diagnostic::Parser(AnnotationMissingSpaceBefore { span }) => {
        LspDiagnostic {
          range: doc.span_to_range(*span),
          severity: Some(lsp_types::DiagnosticSeverity::ERROR),
          code: Some(lsp_types::NumberOrString::String(
            "annotation-missing-space-before".to_string(),
          )),
          source: Some("mf2".to_string()),
          message: "Annotation is missing a leading space, which is required"
            .to_string(),
          code_description: None,
          ..LspDiagnostic::default()
        }
      }
      _ => LspDiagnostic {
        range: doc.span_to_range(self.span()),
        severity: Some(lsp_types::DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("mf2".to_string()),
        message: self.message(),
        related_information: None,
        tags: None,
        data: None,
      },
    }
  }
}

impl fmt::Display for Diagnostic<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} (at {:?})", self.message(), self.span())
  }
}
