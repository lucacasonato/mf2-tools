use mf2_parser::ast::AnyNode;
use mf2_parser::ast::Message;
use mf2_parser::Location;
use mf2_parser::Spanned as _;
use mf2_parser::Visit as _;
use mf2_parser::VisitAny;

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
