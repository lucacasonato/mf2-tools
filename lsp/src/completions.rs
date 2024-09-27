use mf2_parser::ast;
use mf2_parser::ast::AnyNode;
use mf2_parser::ast::Message;
use mf2_parser::Location;
use mf2_parser::Span;
use mf2_parser::Spanned;
use mf2_parser::Visit as _;
use mf2_parser::VisitAny;

use crate::scope::Scope;

#[derive(Debug)]
pub enum CompletionAction {
  Insert,
  Replace(Span),
}

#[derive(Debug)]
pub struct Completion {
  pub text: String,
  pub action: CompletionAction,
}

#[derive(Debug)]
enum AllowedCompletionType<'text> {
  None,
  Variable(Option<(Span, &'text str)>),
}

pub struct CompletionsProvider<'scope: 'text, 'text> {
  scope: &'scope Scope<'text>,
  completion_type: AllowedCompletionType<'text>,
}

impl<'scope, 'text> CompletionsProvider<'scope, 'text> {
  pub fn new<'ast>(
    ast: &'ast Message<'text>,
    loc: Location,
    scope: &'scope Scope<'text>,
  ) -> Self {
    let mut visitor = CompletionLocationVisitor {
      loc,
      current_node: AnyNode::Message(ast),
      parent_node: AnyNode::Message(ast),
      previous_node: None,
    };
    visitor.visit_message(ast);

    Self {
      scope,
      completion_type: get_completion_type(ast, loc),
    }
  }

  pub fn has_completions(&self) -> bool {
    !matches!(self.completion_type, AllowedCompletionType::None)
  }

  pub fn get_completions(&self) -> Vec<Completion> {
    dbg!(&self.completion_type);
    match self.completion_type {
      AllowedCompletionType::None => vec![],
      AllowedCompletionType::Variable(None) => self
        .scope
        .get_names()
        .map(|n| Completion {
          text: format!("${}", n),
          action: CompletionAction::Insert,
        })
        .collect(),
      AllowedCompletionType::Variable(Some((span, name))) => {
        let include_self =
          name.len() > 1 && self.scope.get_spans(name).unwrap().len() > 1;

        let all_names = self
          .scope
          .get_names()
          .filter(|n| include_self || *n != name)
          .map(|n| Completion {
            text: format!("${}", n),
            action: CompletionAction::Replace(span),
          });

        all_names.collect()
      }
    }
  }
}

struct CompletionLocationVisitor<'ast, 'text> {
  loc: Location,
  parent_node: AnyNode<'ast, 'text>,
  current_node: AnyNode<'ast, 'text>,
  previous_node: Option<AnyNode<'ast, 'text>>,
}

impl<'ast, 'text> VisitAny<'ast, 'text>
  for CompletionLocationVisitor<'ast, 'text>
{
  fn before(&mut self, node: AnyNode<'ast, 'text>) {
    let span = node.span();
    if (span.start < self.loc && self.loc <= span.end)
      || (span.start == self.loc && span.is_empty())
    {
      self.parent_node = std::mem::replace(&mut self.current_node, node);
      assert!(!self.parent_node.same(&self.current_node));
      self.previous_node = None;
    }
  }

  fn after(&mut self, node: AnyNode<'ast, 'text>) {
    if node.span().end < self.loc && !node.span().is_empty() {
      self.previous_node = Some(node);
    }
  }
}

fn get_completion_type<'text>(
  ast: &Message<'text>,
  loc: Location,
) -> AllowedCompletionType<'text> {
  let mut visitor = CompletionLocationVisitor {
    loc,
    current_node: AnyNode::Message(ast),
    parent_node: AnyNode::Message(ast),
    previous_node: None,
  };
  visitor.visit_message(ast);

  let CompletionLocationVisitor {
    current_node,
    parent_node,
    previous_node,
    ..
  } = visitor;

  use ast::*;
  use AnyNode as X;

  match (current_node, parent_node, previous_node) {
    (X::Variable(var), _, _) => {
      // $f|
      AllowedCompletionType::Variable(Some((var.span(), var.name)))
    }
    (X::LiteralExpression(literal_expression), _, None) => {
      if literal_expression.literal.span().is_empty() {
        // { | }
        AllowedCompletionType::Variable(None)
      } else {
        // { | 1 }
        // { 1 | }
        AllowedCompletionType::None
      }
    }
    (X::Text(text), X::FnOrMarkupOption(_), _) => {
      if text.span().is_empty() {
        // :fn param=|
        AllowedCompletionType::Variable(None)
      } else {
        // :fn param=f|
        AllowedCompletionType::None
      }
    }
    (
      X::VariableExpression(_)
      | X::AnnotationExpression(_)
      | X::LiteralExpression(_),
      _,
      Some(X::Function(fun)),
    ) => {
      #[allow(clippy::collapsible_match)]
      if let Some(FnOrMarkupOption { value, .. }) = fun.options.last() {
        if let LiteralOrVariable::Literal(Literal::Text(text)) = &value {
          if text.span().is_empty() {
            // { $a :fn param= | }
            return AllowedCompletionType::Variable(None);
          }
        }
      }
      AllowedCompletionType::None
    }
    (X::AnnotationExpression(_), _, None) => {
      // { | :fn }
      AllowedCompletionType::Variable(None)
    }
    _ => AllowedCompletionType::None,
  }
}

#[cfg(test)]
mod tests {
  use mf2_parser::parse;
  use mf2_parser::Location;

  use super::get_completion_type;
  use super::AllowedCompletionType;

  macro_rules! assert_completion_type {
    ($source:expr, $expected:pat) => {
      let loc = Location::new_for_test(
        $source.find('┋').expect("Cursor not found") as u32,
      );
      let message = $source.replace('┋', "");
      let (ast, ..) = parse(&message);
      let result = get_completion_type(&ast, loc);

      assert!(
        matches!(result, $expected),
        "expected: {}\nactual: {:?}",
        stringify!($expected),
        result
      );
    };
  }

  #[test]
  #[rustfmt::skip]
  fn works() {
    assert_completion_type!("{$┋}", AllowedCompletionType::Variable(Some((_, ""))));
    assert_completion_type!("{$f┋}", AllowedCompletionType::Variable(Some((_, "f"))));
    assert_completion_type!("{$┋f}", AllowedCompletionType::Variable(Some((_, "f"))));
    assert_completion_type!("{┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ ┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{┋ }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ ┋ }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{1┋}", AllowedCompletionType::None);
    assert_completion_type!("{1 ┋}", AllowedCompletionType::None);
    assert_completion_type!("{┋1}", AllowedCompletionType::None);
    assert_completion_type!("{ ┋1}", AllowedCompletionType::None);
    assert_completion_type!("{:fn ┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{:fn param=$f┋}", AllowedCompletionType::Variable(Some((_, "f"))));
    assert_completion_type!("{:fn param=f┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param=┋f}", AllowedCompletionType::None);
    assert_completion_type!("{:fn p1=a p2=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{:fn p1=a p2=b┋}", AllowedCompletionType::None);
    assert_completion_type!("{ 1 :fn param=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ $x :fn param=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ ┋ :fn }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ $x┋ :fn }", AllowedCompletionType::Variable(Some((_, "x"))));
  }
}
