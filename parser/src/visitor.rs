use crate::ast;
use crate::ast::AnyNode;

macro_rules! visit {
  ($fn:ident, $param:ident, $type:ident$(<$lt:lifetime>)?) => {
    fn $fn(&mut self, $param: &'ast ast::$type$(<$lt>)?) {
      $param.apply_visitor_to_children(self);
    }
  };
}

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
  visit!(visit_function, func, Function<'text>);
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
  visit!(visit_reserved_body_part, part, ReservedBodyPart<'text>);
  visit!(visit_reserved_statement, stmt, ReservedStatement<'text>);
  visit!(visit_complex_message_body, body, ComplexMessageBody<'text>);
  visit!(visit_quoted_pattern, pattern, QuotedPattern<'text>);
  visit!(visit_matcher, matcher, Matcher<'text>);
  visit!(visit_variant, variant, Variant<'text>);
  visit!(visit_key, key, Key<'text>);
  visit!(visit_star, star, Star);
}

pub trait Visitable<'text> {
  fn apply_visitor<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  );

  fn apply_visitor_to_children<'ast, V: Visit<'ast, 'text> + ?Sized>(
    &'ast self,
    visitor: &mut V,
  );
}

pub struct AnyNodeVisitor<'ast, 'text: 'ast, F>
where
  F: FnMut(AnyNode<'ast, 'text>),
{
  callback: F,
  phantom: std::marker::PhantomData<&'text ()>,
  phantom2: std::marker::PhantomData<&'ast ()>,
}

impl<'ast, 'text, F> AnyNodeVisitor<'ast, 'text, F>
where
  F: FnMut(AnyNode<'ast, 'text>),
{
  pub fn new(callback: F) -> Self {
    AnyNodeVisitor {
      callback,
      phantom: std::marker::PhantomData,
      phantom2: std::marker::PhantomData,
    }
  }
}

macro_rules! any_visit {
  ($fn:ident, $param:ident, $type:ident) => {
    fn $fn(&mut self, $param: &'ast ast::$type<'text>) {
      (self.callback)(AnyNode::$type($param));
      $param.apply_visitor_to_children(self);
    }
  };
}

impl<'ast, 'text, F> Visit<'ast, 'text> for AnyNodeVisitor<'ast, 'text, F>
where
  F: FnMut(AnyNode<'ast, 'text>),
{
  any_visit!(visit_message, message, Message);
  any_visit!(visit_pattern, msg, Pattern);
  any_visit!(visit_pattern_part, part, PatternPart);
  any_visit!(visit_text, text, Text);
  fn visit_escape(&mut self, escape: &'ast ast::Escape) {
    (self.callback)(AnyNode::Escape(escape));
    escape.apply_visitor_to_children(self);
  }
  any_visit!(visit_expression, expr, Expression);
  any_visit!(visit_literal_expression, expr, LiteralExpression);
  any_visit!(visit_literal, literal, Literal);
  any_visit!(visit_quoted, quoted, Quoted);
  any_visit!(visit_quoted_part, part, QuotedPart);
  any_visit!(visit_number, num, Number);
  any_visit!(visit_annotation, ann, Annotation);
  any_visit!(visit_function, func, Function);
  any_visit!(visit_identifier, ident, Identifier);
  any_visit!(visit_fn_or_markup_option, opt, FnOrMarkupOption);
  any_visit!(visit_literal_or_variable, lit_or_var, LiteralOrVariable);
  any_visit!(visit_variable, var, Variable);
  any_visit!(visit_attribute, attr, Attribute);
  any_visit!(visit_variable_expression, expr, VariableExpression);
  any_visit!(visit_annotation_expression, expr, AnnotationExpression);
  any_visit!(visit_markup, markup, Markup);
  any_visit!(visit_complex_message, msg, ComplexMessage);
  any_visit!(visit_declaration, decl, Declaration);
  any_visit!(visit_input_declaration, decl, InputDeclaration);
  any_visit!(visit_local_declaration, decl, LocalDeclaration);
  any_visit!(visit_reserved_body_part, part, ReservedBodyPart);
  any_visit!(visit_reserved_statement, stmt, ReservedStatement);
  any_visit!(visit_complex_message_body, body, ComplexMessageBody);
  any_visit!(visit_quoted_pattern, pattern, QuotedPattern);
  any_visit!(visit_matcher, matcher, Matcher);
  any_visit!(visit_variant, variant, Variant);
  any_visit!(visit_key, key, Key);
  fn visit_star(&mut self, star: &'ast ast::Star) {
    (self.callback)(AnyNode::Star(star));
    star.apply_visitor_to_children(self);
  }
}
