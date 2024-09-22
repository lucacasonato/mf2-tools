use lsp_types::Diagnostic;

use crate::document::Document;

pub fn mf2_diagnostic_to_lsp(
  doc: &Document,
  diagnostic: &mf2_parser::Diagnostic,
) -> Diagnostic {
  match diagnostic {
    mf2_parser::Diagnostic::AnnotationMissingSpaceBefore { span } => {
      Diagnostic {
        range: doc.span_to_range(*span),
        severity: Some(lsp_types::DiagnosticSeverity::ERROR),
        code: Some(lsp_types::NumberOrString::String(
          "annotation-missing-space-before".to_string(),
        )),
        source: Some("mf2".to_string()),
        message: "Annotation is missing a leading space, which is required"
          .to_string(),
        code_description: None,
        ..Diagnostic::default()
      }
    }
    _ => Diagnostic {
      range: doc.span_to_range(diagnostic.span()),
      severity: Some(lsp_types::DiagnosticSeverity::ERROR),
      code: None,
      code_description: None,
      source: Some("mf2".to_string()),
      message: diagnostic.message(),
      related_information: None,
      tags: None,
      data: None,
    },
  }
}
