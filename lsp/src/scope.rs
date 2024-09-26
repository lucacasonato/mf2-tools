use std::collections::HashMap;

use mf2_parser::Span;
use mf2_parser::Spanned as _;
use mf2_parser::Visitable as _;

use crate::diagnostics::Diagnostic;
use crate::diagnostics::ScopeDiagnostic;

pub struct VariableUsage {
  declaration: Option<Span>,
  references: Vec<Span>,
}

pub struct ScopeVisitor<'text> {
  variables: HashMap<&'text str, VariableUsage>,
  pub diagnostics: Vec<Diagnostic<'text>>,
}

impl<'text> ScopeVisitor<'text> {
  pub fn new(diagnostics: Vec<Diagnostic<'text>>) -> ScopeVisitor<'text> {
    ScopeVisitor {
      diagnostics,
      variables: HashMap::new(),
    }
  }

  fn push_variable_declaration<'ast>(
    &mut self,
    var: &'ast mf2_parser::ast::Variable<'text>,
  ) {
    if let Some(existing) = self.variables.get_mut(var.name) {
      if let Some(existing_span) = existing.declaration {
        self.diagnostics.push(Diagnostic::Scope(
          ScopeDiagnostic::DuplicateDeclaration {
            name: var.name,
            first_span: existing_span,
            second_span: var.span(),
          },
        ));

        existing.references.push(var.span());
      } else {
        for reference in &existing.references {
          self.diagnostics.push(Diagnostic::Scope(
            ScopeDiagnostic::UsageBeforeDeclaration {
              name: var.name,
              declaration_span: var.span(),
              usage_span: *reference,
            },
          ));
        }

        existing.declaration = Some(var.span());
      }
    } else {
      self.variables.insert(
        var.name,
        VariableUsage {
          declaration: Some(var.span()),
          references: Vec::new(),
        },
      );
    }
  }

  fn push_variable_reference<'ast>(
    &mut self,
    var: &'ast mf2_parser::ast::Variable<'text>,
  ) {
    if let Some(existing) = self.variables.get_mut(var.name) {
      existing.references.push(var.span());
    } else {
      self.variables.insert(
        var.name,
        VariableUsage {
          declaration: None,
          references: vec![var.span()],
        },
      );
    }
  }
}

impl<'ast, 'text> mf2_parser::Visit<'ast, 'text> for ScopeVisitor<'text> {
  fn visit_local_declaration(
    &mut self,
    decl: &'ast mf2_parser::ast::LocalDeclaration<'text>,
  ) {
    decl.expression.apply_visitor(self);

    self.push_variable_declaration(&decl.variable);
  }

  fn visit_input_declaration(
    &mut self,
    decl: &'ast mf2_parser::ast::InputDeclaration<'text>,
  ) {
    if let Some(annotation) = &decl.expression.annotation {
      annotation.apply_visitor(self);
    }

    self.push_variable_declaration(&decl.expression.variable);
  }

  fn visit_variable(&mut self, var: &'ast mf2_parser::ast::Variable<'text>) {
    self.push_variable_reference(var);
  }
}
