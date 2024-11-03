use crate::ast;
use crate::ast::AnyNode;

macro_rules! visit {
  ($fn:ident, $param:ident, $type:ident$(<$lt:lifetime>)?) => {
    fn $fn(&mut self, $param: &'ast ast::$type$(<$lt>)?) {
      $param.apply_visitor_to_children(self);
    }
  };
}

/// The [Visit] trait is used to traverse the AST. You can implement this trait
/// to visit each node in the AST in source text order.
///
/// Each method in the trait corresponds to a node type in the AST. The method
/// is called with a reference to the node. The default implementation of each
/// method calls [Visitable::apply_visitor_to_children] on the node, which then
/// recursively applies the visitor to the node's children. To implement a
/// recursive visitor, you must also call [Visitable::apply_visitor_to_children]
/// on any overridden visit methods.
///
/// ### Example
///
/// ```rust
/// use mf2_parser::ast::*;
/// use mf2_parser::Visit;
/// use mf2_parser::Visitable as _;
/// use mf2_parser::parse;
///
/// struct MyVisitor;
///
/// impl<'ast, 'text> Visit<'ast, 'text> for MyVisitor {
///   fn visit_variable(&mut self, var: &'ast Variable<'text>) {
///     println!("Found variable: {}", var.name);
///     var.apply_visitor_to_children(self);
///   }
/// }
///
///
/// let (ast, _, _) = parse("Hello, {$name}!");
/// let mut visitor = MyVisitor;
/// ast.apply_visitor(&mut visitor);
/// ```
pub trait Visit<'ast, 'text> {
  visit!(visit_message, message, Message<'text>);
  visit!(visit_pattern, msg, Pattern<'text>);
  visit!(visit_pattern_part, part, PatternPart<'text>);
  visit!(visit_text, text, Text<'text>);
  visit!(visit_escape, escape, Escape);
  visit!(visit_expression, expr, Expression<'text>);
  visit!(visit_literal_expression, expr, LiteralExpression<'text>);
  visit!(visit_literal, literal, Literal<'text>);
  visit!(visit_quoted, quoted, Quoted<'text>);
  visit!(visit_quoted_part, part, QuotedPart<'text>);
  visit!(visit_number, num, Number<'text>);
  visit!(visit_annotation, ann, Annotation<'text>);
  visit!(visit_identifier, ident, Identifier<'text>);
  visit!(visit_fn_or_markup_option, opt, FnOrMarkupOption<'text>);
  visit!(
    visit_literal_or_variable,
    lit_or_var,
    LiteralOrVariable<'text>
  );
  visit!(visit_variable, var, Variable<'text>);
  visit!(visit_attribute, attr, Attribute<'text>);
  visit!(visit_variable_expression, expr, VariableExpression<'text>);
  visit!(
    visit_annotation_expression,
    expr,
    AnnotationExpression<'text>
  );
  visit!(visit_markup, markup, Markup<'text>);
  visit!(visit_complex_message, msg, ComplexMessage<'text>);
  visit!(visit_declaration, decl, Declaration<'text>);
  visit!(visit_input_declaration, decl, InputDeclaration<'text>);
  visit!(visit_local_declaration, decl, LocalDeclaration<'text>);
  visit!(visit_complex_message_body, body, ComplexMessageBody<'text>);
  visit!(visit_quoted_pattern, pattern, QuotedPattern<'text>);
  visit!(visit_matcher, matcher, Matcher<'text>);
  visit!(visit_variant, variant, Variant<'text>);
  visit!(visit_key, key, Key<'text>);
  visit!(visit_star, star, Star);
}

/// The [Visitable] trait is used to apply a [Visit]or to an AST node.
pub trait Visitable<'text> {
  /// Call the visitor method for this node on the visitor.
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  );

  /// Call the visitor method for each child node on the visitor. This does not
  /// call the visitor method for this node itself.
  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  );
}

/// The [VisitAny] trait is used to visit the AST without having to know the
/// specific shape of each node. There are two methods, [VisitAny::before] and
/// [VisitAny::after], which are called before and after visiting the children
/// of a given node, respectively.
///
/// The [AnyNode] enum is used to represent any node in the AST.
///
/// ### Example
///
/// ```rust
/// use mf2_parser::ast::*;
/// use mf2_parser::VisitAny;
/// use mf2_parser::Visitable as _;
/// use mf2_parser::parse;
///
/// struct MyVisitor;
///
/// impl<'ast, 'text: 'ast> VisitAny<'ast, 'text> for MyVisitor {
///   fn before(&mut self, node: AnyNode<'ast, 'text>) {
///     println!("Start visiting node: {:?}", node);
///   }
///
///   fn after(&mut self, node: AnyNode<'ast, 'text>) {
///     println!("Finished visiting node: {:?}", node);
///   }
/// }
///
/// let (ast, _, _) = parse("Hello, {$name}!");
/// let mut visitor = MyVisitor;
/// ast.apply_visitor(&mut visitor);
/// ```
pub trait VisitAny<'ast, 'text: 'ast> {
  /// Called before visiting the children of a node.
  ///
  /// The default implementation of this method does nothing.
  fn before(&mut self, _node: AnyNode<'ast, 'text>) {}
  /// Called after visiting the children of a node.
  ///
  /// The default implementation of this method does nothing.
  fn after(&mut self, _node: AnyNode<'ast, 'text>) {}
}

macro_rules! visit_any {
  ($fn:ident, $param:ident, $type:ident) => {
    fn $fn(&mut self, $param: &'ast ast::$type<'text>) {
      self.before(AnyNode::$type($param));
      $param.apply_visitor_to_children(self);
      self.after(AnyNode::$type($param));
    }
  };
}

impl<'ast, 'text: 'ast, T: VisitAny<'ast, 'text>> Visit<'ast, 'text> for T {
  fn visit_escape(&mut self, escape: &'ast ast::Escape) {
    self.before(AnyNode::Escape(escape));
    escape.apply_visitor_to_children(self);
    self.after(AnyNode::Escape(escape));
  }
  fn visit_star(&mut self, star: &'ast ast::Star) {
    self.before(AnyNode::Star(star));
    star.apply_visitor_to_children(self);
    self.after(AnyNode::Star(star));
  }
  visit_any!(visit_pattern, msg, Pattern);
  visit_any!(visit_text, text, Text);
  visit_any!(visit_literal_expression, expr, LiteralExpression);
  visit_any!(visit_quoted, quoted, Quoted);
  visit_any!(visit_number, num, Number);
  visit_any!(visit_identifier, ident, Identifier);
  visit_any!(visit_fn_or_markup_option, opt, FnOrMarkupOption);
  visit_any!(visit_variable, var, Variable);
  visit_any!(visit_attribute, attr, Attribute);
  visit_any!(visit_variable_expression, expr, VariableExpression);
  visit_any!(visit_annotation_expression, expr, AnnotationExpression);
  visit_any!(visit_markup, markup, Markup);
  visit_any!(visit_complex_message, msg, ComplexMessage);
  visit_any!(visit_input_declaration, decl, InputDeclaration);
  visit_any!(visit_local_declaration, decl, LocalDeclaration);
  visit_any!(visit_quoted_pattern, pattern, QuotedPattern);
  visit_any!(visit_matcher, matcher, Matcher);
  visit_any!(visit_variant, variant, Variant);
}
