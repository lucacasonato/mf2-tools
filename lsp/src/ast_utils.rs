use std::cmp::Ordering;

use mf2_parser::Location;
use mf2_parser::Span;
use mf2_parser::Spanned as _;
use mf2_parser::Visit as _;
use mf2_parser::VisitAny;
use mf2_parser::ast::AnnotationExpression;
use mf2_parser::ast::AnyNode;
use mf2_parser::ast::LiteralExpression;
use mf2_parser::ast::Message;
use mf2_parser::ast::Quoted;
use mf2_parser::ast::QuotedPattern;
use mf2_parser::ast::VariableExpression;

/// Finds the innermost node that contains the given location.
struct FindNodeVisitor<'ast, 'text> {
  loc: Location,
  result: Option<AnyNode<'ast, 'text>>,
}

impl<'ast, 'text> VisitAny<'ast, 'text> for FindNodeVisitor<'ast, 'text> {
  fn before(&mut self, node: AnyNode<'ast, 'text>) {
    if node.span().contains_loc(self.loc) {
      self.result = Some(node);
    }
  }
}

pub fn find_node<'ast, 'text>(
  ast: &'ast Message<'text>,
  loc: Location,
) -> Option<AnyNode<'ast, 'text>> {
  let mut visitor = FindNodeVisitor { loc, result: None };
  visitor.visit_message(ast);
  visitor.result
}

/// Finds the innermost node where the span start is less than the location, and
/// the end span is greater or equal to the location, and some additional
/// context about where that node sits in the AST.
///
/// This additional context is:
/// - The path from the root of the AST to the innermost node, as a list of nodes. (The list of parents.)
/// - The list of nodes with the maximum end position that is less than or equal to the given location. (The list of previous nodes.)
struct FindNodeAtCursorWithContextVisitor<'ast, 'text> {
  loc: Location,
  containing_nodes: Vec<AnyNode<'ast, 'text>>,
  greatest_previous_nodes: Vec<AnyNode<'ast, 'text>>,
}

impl<'ast, 'text> VisitAny<'ast, 'text> for FindNodeAtCursorWithContextVisitor<'ast, 'text> {
  fn before(&mut self, current: AnyNode<'ast, 'text>) {
    let Span {
      start: current_start,
      end: current_end,
    } = current.span();
    let is_exclusive_of_end = is_exclusive_of_end(&current);
    let is_end_after_loc = if is_exclusive_of_end {
      self.loc < current_end
    } else {
      self.loc <= current_end
    };
    if current_start < self.loc && is_end_after_loc {
      self.containing_nodes.push(current);
    }

    if current_start != current_end {
      if let Some(prev) = self.greatest_previous_nodes.last() {
        let prev_end = prev.span().end;
        if current_start < self.loc && current_start >= prev_end {
          self.greatest_previous_nodes.clear();
        }
        if !is_end_after_loc && current_end > prev_end {
          self.greatest_previous_nodes.clear();
        }
      }
    }

    if !is_end_after_loc && current_start != current_end {
      let ord = self
        .greatest_previous_nodes
        .last()
        .map(|prev| prev.span().end.cmp(&current_end));
      match ord {
        Some(Ordering::Less) => self.greatest_previous_nodes.push(current),
        None | Some(Ordering::Equal) => self.greatest_previous_nodes.push(current),
        Some(Ordering::Greater) => {}
      }
    }

    // clear when I see a non-empty node that starts before loc and starts or ends after the greatest_previous_node end

    // clear when I see a non-empty node that at least one of:
    // - starts before loc and starts and starts after the greatest_previous_node end
    // - ends before loc and starts and ends after the greatest_previous_node end
  }
}

pub struct ContainingNodesAndGreatestPreviousNodes<'ast, 'text> {
  pub containing_nodes: Vec<AnyNode<'ast, 'text>>,
  pub greatest_previous_nodes: Vec<AnyNode<'ast, 'text>>,
}

pub fn find_node_at_cursor_with_context<'ast, 'text>(
  ast: &'ast Message<'text>,
  loc: Location,
) -> ContainingNodesAndGreatestPreviousNodes<'ast, 'text> {
  let mut visitor = FindNodeAtCursorWithContextVisitor {
    loc,
    containing_nodes: Vec::new(),
    greatest_previous_nodes: Vec::new(),
  };
  visitor.visit_message(ast);
  ContainingNodesAndGreatestPreviousNodes {
    containing_nodes: visitor.containing_nodes,
    greatest_previous_nodes: visitor.greatest_previous_nodes,
  }
}

fn is_exclusive_of_end(node: &AnyNode) -> bool {
  match node {
    AnyNode::LiteralExpression(LiteralExpression {
      has_closing_brace: true,
      ..
    })
    | AnyNode::AnnotationExpression(AnnotationExpression {
      has_closing_brace: true,
      ..
    })
    | AnyNode::VariableExpression(VariableExpression {
      has_closing_brace: true,
      ..
    }) => true,
    AnyNode::QuotedPattern(QuotedPattern { span, pattern }) if span.end != pattern.span().end => {
      true
    }
    AnyNode::Quoted(Quoted { span, parts })
      if parts.last().map_or(span.start + "||" == span.end, |part| {
        part.span().end != span.end
      }) =>
    {
      true
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use std::ops::Deref;

  use mf2_parser::{Location, ast::AnyNode as A, parse};

  use crate::ast_utils::find_node_at_cursor_with_context;

  macro_rules! find_node_at_cursor_with_context_test {
    ($name:ident, $src:expr, $expected_containing:pat, $expected_previous:pat) => {
      #[test]
      fn $name() {
        let src = $src;
        let loc = Location::new_for_test(src.find('┋').unwrap() as u32);
        let message = src.replace('┋', "");
        let (ast, ..) = parse(&message);
        let result = find_node_at_cursor_with_context(&ast, loc);
        assert!(
          matches!(result.containing_nodes.deref(), &$expected_containing),
          "Expected containing nodes to match pattern {:?}, but got {:#?}",
          stringify!($expected_containing),
          result.containing_nodes
        );
        assert!(
          matches!(result.greatest_previous_nodes.deref(), &$expected_previous),
          "Expected greatest previous nodes to match pattern {:?}, but got {:#?}",
          stringify!($expected_previous),
          result.greatest_previous_nodes
        );
      }
    };
  }

  find_node_at_cursor_with_context_test!(
    empty_expression_padded_both,
    r#"{ ┋ }"#,
    [A::Pattern(_), A::LiteralExpression(_)],
    []
  );

  find_node_at_cursor_with_context_test!(
    empty_expression_padded_left,
    r#"{ ┋}"#,
    [A::Pattern(_), A::LiteralExpression(_)],
    []
  );

  find_node_at_cursor_with_context_test!(
    empty_expression_padded_right,
    r#"{┋ }"#,
    [A::Pattern(_), A::LiteralExpression(_)],
    []
  );

  find_node_at_cursor_with_context_test!(
    empty_expression,
    r#"{┋}"#,
    [A::Pattern(_), A::LiteralExpression(_)],
    []
  );

  find_node_at_cursor_with_context_test!(
    end_of_function,
    r#"{ :fn┋ }"#,
    [
      A::Pattern(_),
      A::AnnotationExpression(_),
      A::Annotation(_),
      A::Identifier(_)
    ],
    []
  );

  find_node_at_cursor_with_context_test!(
    end_of_function_sigil,
    r#"{ :┋ }"#,
    [A::Pattern(_), A::AnnotationExpression(_), A::Annotation(_)],
    []
  );

  find_node_at_cursor_with_context_test!(
    after_annotation,
    r#"{ :fn ┋ }"#,
    [A::Pattern(_), A::AnnotationExpression(_)],
    [A::Annotation(_), A::Identifier(_)]
  );

  find_node_at_cursor_with_context_test!(
    end_of_variable,
    r#"{ $var┋ }"#,
    [A::Pattern(_), A::VariableExpression(_), A::Variable(_)],
    []
  );

  find_node_at_cursor_with_context_test!(
    end_of_variable_sigil,
    r#"{ $┋ }"#,
    [A::Pattern(_), A::VariableExpression(_), A::Variable(_)],
    []
  );

  find_node_at_cursor_with_context_test!(
    after_variable,
    r#"{ $var ┋ }"#,
    [A::Pattern(_), A::VariableExpression(_)],
    [A::Variable(_)]
  );

  find_node_at_cursor_with_context_test!(
    after_variable_in_annotation,
    r#"{ $var :fn ┋ }"#,
    [A::Pattern(_), A::VariableExpression(_)],
    [A::Annotation(_), A::Identifier(_)]
  );

  find_node_at_cursor_with_context_test!(
    end_at_annotation_option_no_value,
    r#"{ $var :fn opt=┋ }"#,
    [
      A::Pattern(_),
      A::VariableExpression(_),
      A::Annotation(_),
      A::FnOrMarkupOption(_)
    ],
    [A::Identifier(_)]
  );

  find_node_at_cursor_with_context_test!(
    after_annotation_option,
    r#"{ $var :fn opt=asd ┋ }"#,
    [A::Pattern(_), A::VariableExpression(_)],
    [A::Annotation(_), A::FnOrMarkupOption(_), A::Text(_)]
  );

  find_node_at_cursor_with_context_test!(
    end_of_annotation_option,
    r#"{ $var :fn opt=asd┋ }"#,
    [
      A::Pattern(_),
      A::VariableExpression(_),
      A::Annotation(_),
      A::FnOrMarkupOption(_),
      A::Text(_)
    ],
    []
  );

  find_node_at_cursor_with_context_test!(
    end_of_second_option,
    r#"{ :fn a=b c┋ }"#,
    [
      A::Pattern(_),
      A::AnnotationExpression(_),
      A::Annotation(_),
      A::FnOrMarkupOption(_),
      A::Identifier(_)
    ],
    []
  );

  find_node_at_cursor_with_context_test!(
    match_after_key,
    r#".match $var foo ┋ {{ }}"#,
    [A::ComplexMessage(_), A::Matcher(_), A::Variant(_)],
    [A::Text(_)]
  );

  find_node_at_cursor_with_context_test!(
    match_after_key_no_body,
    r#".match $var foo ┋"#,
    [A::ComplexMessage(_), A::Matcher(_), A::Variant(_)],
    [A::Text(_)]
  );

  find_node_at_cursor_with_context_test!(
    match_after_body,
    r#".match $var foo {{}} ┋"#,
    [A::ComplexMessage(_)],
    [A::Matcher(_), A::Variant(_), A::QuotedPattern(_)]
  );

  find_node_at_cursor_with_context_test!(
    end_of_body,
    r#".match $var foo {{}}┋"#,
    [A::ComplexMessage(_), A::Matcher(_), A::Variant(_),],
    [A::QuotedPattern(_)]
  );

  find_node_at_cursor_with_context_test!(
    match_end_of_body,
    r#".match $var foo {{┋}}"#,
    [
      A::ComplexMessage(_),
      A::Matcher(_),
      A::Variant(_),
      A::QuotedPattern(_)
    ],
    []
  );

  find_node_at_cursor_with_context_test!(
    after_annotation_expression,
    r#"{:fn opt=}┋"#,
    [A::Pattern(_)],
    [A::AnnotationExpression(_)]
  );

  find_node_at_cursor_with_context_test!(
    after_annotation_expression_no_closing_brace,
    r#"{:fn opt= ┋"#,
    [A::Pattern(_), A::AnnotationExpression(_),],
    [A::Annotation(_), A::FnOrMarkupOption(_)]
  );
}
