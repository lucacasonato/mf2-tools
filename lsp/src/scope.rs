use std::collections::hash_map::Entry;
use std::collections::HashMap;

use mf2_parser::ast;
use mf2_parser::Span;
use mf2_parser::Spanned as _;
use mf2_parser::Visit as _;
use mf2_parser::Visitable as _;

pub enum ScopeDiagnostic<'text> {
  DuplicateDeclaration {
    name: &'text str,
    #[allow(dead_code)]
    first_span: Span,
    second_span: Span,
  },
  UsageBeforeDeclaration {
    name: &'text str,
    #[allow(dead_code)]
    declaration_span: Span,
    usage_span: Span,
  },
}

pub struct VariableUsage {
  pub declaration: Option<Span>,
  pub all: Vec<Span>,
}

pub struct Scope<'text> {
  variables: HashMap<&'text str, VariableUsage>,
}

impl Scope<'_> {
  pub fn analyse<'text>(
    ast: &ast::Message<'text>,
  ) -> (Scope<'text>, Vec<ScopeDiagnostic<'text>>) {
    let mut visitor = ScopeVisitor {
      scope: Scope {
        variables: HashMap::new(),
      },
      diagnostics: vec![],
    };
    visitor.visit_message(ast);

    (visitor.scope, visitor.diagnostics)
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

struct ScopeVisitor<'text> {
  scope: Scope<'text>,
  diagnostics: Vec<ScopeDiagnostic<'text>>,
}

impl<'text> ScopeVisitor<'text> {
  fn push_variable_declaration<'ast>(
    &mut self,
    var: &'ast mf2_parser::ast::Variable<'text>,
  ) {
    match self.scope.variables.entry(var.name) {
      Entry::Occupied(existing) => {
        let existing = existing.into_mut();
        if let Some(existing_span) = existing.declaration {
          self
            .diagnostics
            .push(ScopeDiagnostic::DuplicateDeclaration {
              name: var.name,
              first_span: existing_span,
              second_span: var.span(),
            });
        } else {
          for reference in &existing.all {
            self
              .diagnostics
              .push(ScopeDiagnostic::UsageBeforeDeclaration {
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

  fn push_variable_reference<'ast>(
    &mut self,
    var: &'ast mf2_parser::ast::Variable<'text>,
  ) {
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
