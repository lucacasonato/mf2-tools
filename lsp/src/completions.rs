use mf2_parser::Location;
use mf2_parser::Scope;
use mf2_parser::Span;
use mf2_parser::Spanned;
use mf2_parser::ast;
use mf2_parser::ast::AnyNode;
use mf2_parser::ast::Message;

use crate::ast_utils::ContainingNodesAndGreatestPreviousNodes;
use crate::ast_utils::find_node_at_cursor_with_context;

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
  pub fn new<'ast>(ast: &'ast Message<'text>, loc: Location, scope: &'scope Scope<'text>) -> Self {
    Self {
      scope,
      completion_type: get_completion_type(ast, loc),
    }
  }

  pub fn has_completions(&self) -> bool {
    !matches!(self.completion_type, AllowedCompletionType::None)
  }

  pub fn get_completions(&self) -> Vec<Completion> {
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
        let include_self = name.len() > 1 && self.scope.get_spans(name).unwrap().len() > 1;

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

fn get_completion_type<'text>(ast: &Message<'text>, loc: Location) -> AllowedCompletionType<'text> {
  let ContainingNodesAndGreatestPreviousNodes {
    containing_nodes,
    greatest_previous_nodes,
  } = find_node_at_cursor_with_context(ast, loc);

  use AnyNode as X;
  use ast::*;

  macro_rules! match_containing_and_previous {
    ($(($current:pat, $previous:pat) $(if $if:expr)? => $body:block )*
    _ => $default:block) => {
      $(
        if let $current = match_containing_and_previous!(rhs $current, containing_nodes) && let $previous = match_containing_and_previous!(rhs $previous, greatest_previous_nodes) $(&& $if)? $body
      )else*
      else $default
    };
    (rhs $pat:pat, $nodes:expr) => { $nodes.iter().rev().find(#[allow(unused_variables)] |x| matches!(Some(x), $pat)) };
  }

  macro_rules! not_containing {
    ($pat:pat) => {
      containing_nodes.iter().rev().all(|x| !matches!(x, $pat))
    };
  }

  match_containing_and_previous! {
    (Some(X::Variable(var)), _) => {
      // $f|
      // $|
      AllowedCompletionType::Variable(Some((var.span(), var.name)))
    }
    (Some(X::LiteralExpression(literal_expression)), _)
      if literal_expression.literal.span().is_empty() =>
    {
      // { | }  (the empty literal is the only thing in `previous_nodes`)
      AllowedCompletionType::Variable(None)

      // if excludes:
      //  { | 1 }
      //  { 1 | }
    }
    (Some(X::FnOrMarkupOption(FnOrMarkupOption { value: LiteralOrVariable::Literal(Literal::Text(text)), .. })), Some(X::Identifier(_)))
      if text.span().is_empty() && text.span().start <= loc => {
        // :fn param=|
        AllowedCompletionType::Variable(None)

        // if excludes:
        //  :fn param=f| (by empty check)
        //  :fn param |= (by span start check)
      }
    (
      Some(X::VariableExpression(_) | X::AnnotationExpression(_) | X::LiteralExpression(_)),
      Some(X::FnOrMarkupOption(FnOrMarkupOption { key, value: LiteralOrVariable::Literal(Literal::Text(text)), .. }))
    )
      if text.span().is_empty() && key.span().end != text.span().start => {
        // { $a :fn param= | }
        AllowedCompletionType::Variable(None)

        // if excludes:
        //  { $a :fn asd=asd | } (by empty check)
        //  { $a :fn asd | } (by span start check)
      }
    (Some(X::AnnotationExpression(_)), _) if greatest_previous_nodes.is_empty() && not_containing!(X::Annotation(_)) => {
      // { | :fn }
      // { |:fn }
      // {| :fn }
      // { :fn asd=| }
      AllowedCompletionType::Variable(None)

      // if excludes:
      //  { :fn | } (by no previous nodes check)
      //  { :fn asd|=asd } (by not containing annotation check)
    }
    _ => {
      AllowedCompletionType::None
    }
  }
}

#[cfg(test)]
mod tests {
  use mf2_parser::Location;
  use mf2_parser::parse;

  use super::AllowedCompletionType;
  use super::get_completion_type;

  macro_rules! assert_completion_type {
    ($source:expr, $expected:pat) => {
      let loc = Location::new_for_test($source.find('┋').expect("Cursor not found") as u32);
      let message = $source.replace('┋', "");
      let (ast, ..) = parse(&message);
      let result = get_completion_type(&ast, loc);

      std::assert_matches!(result, $expected);
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
    assert_completion_type!("{:fn param┋=}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param ┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param ┋=}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param= ┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{:fn param=$f┋}", AllowedCompletionType::Variable(Some((_, "f"))));
    assert_completion_type!("{:fn param=f┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param=┋f}", AllowedCompletionType::None);
    assert_completion_type!("{:fn p1=a p2=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{:fn p1=a p2=b┋}", AllowedCompletionType::None);
    assert_completion_type!("{ 1 :fn param=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ $x :fn param=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ ┋ :fn }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ ┋:fn }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{┋ :fn }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ $x┋ :fn }", AllowedCompletionType::Variable(Some((_, "x"))));
  }
}
