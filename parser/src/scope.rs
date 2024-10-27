use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::ast;
use crate::Diagnostic;
use crate::Span;
use crate::Spanned as _;
use crate::Visit;
use crate::Visitable as _;

pub struct VariableUsage {
  pub declaration: Option<Span>,
  pub all: Vec<Span>,
}

pub struct Scope<'text> {
  variables: HashMap<&'text str, VariableUsage>,
}

impl Scope<'_> {
  pub(crate) fn analyze<'text>(
    ast: &ast::Message<'text>,
    diagnostics: &mut Vec<Diagnostic<'text>>,
  ) -> Scope<'text> {
    let mut visitor = ScopeVisitor {
      scope: Scope {
        variables: HashMap::new(),
      },
      diagnostics,
    };
    visitor.visit_message(ast);
    visitor.scope
  }

  pub fn get_spans(&self, name: &str) -> Option<&Vec<Span>> {
    self.variables.get(name).map(|u| &u.all)
  }

  pub fn get_declaration_span(&self, name: &str) -> Option<Span> {
    self.variables.get(name).and_then(|u| u.declaration)
  }

  pub fn get_names(&self) -> impl Iterator<Item = &str> {
    self.variables.keys().copied()
  }
}

struct ScopeVisitor<'diag, 'text> {
  scope: Scope<'text>,
  diagnostics: &'diag mut Vec<Diagnostic<'text>>,
}

impl<'text> ScopeVisitor<'_, 'text> {
  fn push_variable_declaration<'ast>(
    &mut self,
    var: &'ast ast::Variable<'text>,
  ) {
    match self.scope.variables.entry(var.name) {
      Entry::Occupied(existing) => {
        let existing = existing.into_mut();
        if let Some(existing_span) = existing.declaration {
          self.diagnostics.push(Diagnostic::DuplicateDeclaration {
            name: var.name,
            first_span: existing_span,
            second_span: var.span(),
          });
        } else {
          for reference in &existing.all {
            self.diagnostics.push(Diagnostic::UsageBeforeDeclaration {
              name: var.name,
              declaration_span: var.span(),
              usage_span: *reference,
            });
          }

          existing.declaration = Some(var.span());
        }

        existing.all.push(var.span());
      }
      Entry::Vacant(vacant) => {
        vacant.insert(VariableUsage {
          declaration: Some(var.span()),
          all: vec![var.span()],
        });
      }
    };
  }

  fn push_variable_reference<'ast>(&mut self, var: &'ast ast::Variable<'text>) {
    if let Some(existing) = self.scope.variables.get_mut(var.name) {
      existing.all.push(var.span());
    } else {
      self.scope.variables.insert(
        var.name,
        VariableUsage {
          declaration: None,
          all: vec![var.span()],
        },
      );
    }
  }
}

impl<'ast, 'text> Visit<'ast, 'text> for ScopeVisitor<'_, 'text> {
  fn visit_local_declaration(
    &mut self,
    decl: &'ast ast::LocalDeclaration<'text>,
  ) {
    decl.expression.apply_visitor(self);

    self.push_variable_declaration(&decl.variable);
  }

  fn visit_input_declaration(
    &mut self,
    decl: &'ast ast::InputDeclaration<'text>,
  ) {
    if let Some(annotation) = &decl.expression.annotation {
      annotation.apply_visitor(self);
    }

    self.push_variable_declaration(&decl.expression.variable);
  }

  fn visit_variable(&mut self, var: &'ast ast::Variable<'text>) {
    self.push_variable_reference(var);
  }
}
