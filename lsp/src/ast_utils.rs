use mf2_parser::ast::AnyNode;
use mf2_parser::ast::Message;
use mf2_parser::AnyNodeVisitor;
use mf2_parser::Location;
use mf2_parser::Spanned as _;
use mf2_parser::Visit as _;

pub fn find_node<'ast, 'text>(
  ast: &'ast Message<'text>,
  loc: Location,
) -> Option<AnyNode<'ast, 'text>> {
  let mut result = None;

  let mut visitor = AnyNodeVisitor::new(|node| {
    if node.span().contains_loc(loc) {
      result = Some(node);
    }
  });

  visitor.visit_message(ast);

  result
}
