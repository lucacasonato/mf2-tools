use crate::ast;
use crate::ast::AnyNode;

macro_rules! visit {
  ($fn:ident, $param:ident, $type:ident$(<$lt:lifetime>)?) => {
    fn $fn(&mut self, $param: &'a ast::$type$(<$lt>)?) {
      $param.apply_visitor_to_children(self);
    }
  };
}

pub trait Visit<'a> {
  visit!(visit_message, message, Message<'a>);
  visit!(visit_pattern, msg, Pattern<'a>);
  visit!(visit_pattern_part, part, PatternPart<'a>);
  visit!(visit_text, text, Text<'a>);
  visit!(visit_escape, escape, Escape);
  visit!(visit_expression, expr, Expression<'a>);
  visit!(visit_literal_expression, expr, LiteralExpression<'a>);
  visit!(visit_literal, literal, Literal<'a>);
  visit!(visit_quoted, quoted, Quoted<'a>);
  visit!(visit_quoted_part, part, QuotedPart<'a>);
  visit!(visit_number, num, Number<'a>);
  visit!(visit_annotation, ann, Annotation<'a>);
  visit!(visit_function, func, Function<'a>);
  visit!(visit_identifier, ident, Identifier<'a>);
  visit!(visit_fn_or_markup_option, opt, FnOrMarkupOption<'a>);
  visit!(visit_literal_or_variable, lit_or_var, LiteralOrVariable<'a>);
  visit!(visit_variable, var, Variable<'a>);
  visit!(visit_attribute, attr, Attribute<'a>);
  visit!(visit_private_use_annotation, ann, PrivateUseAnnotation<'a>);
  visit!(visit_reserved_body_part, part, ReservedBodyPart<'a>);
  visit!(visit_reserved_annotation, ann, ReservedAnnotation<'a>);
  visit!(visit_variable_expression, expr, VariableExpression<'a>);
  visit!(visit_annotation_expression, expr, AnnotationExpression<'a>);
  visit!(visit_markup, markup, Markup<'a>);
  visit!(visit_complex_message, msg, ComplexMessage<'a>);
  visit!(visit_declaration, decl, Declaration<'a>);
  visit!(visit_input_declaration, decl, InputDeclaration<'a>);
  visit!(visit_local_declaration, decl, LocalDeclaration<'a>);
  visit!(visit_reserved_statement, stmt, ReservedStatement<'a>);
  visit!(visit_complex_message_body, body, ComplexMessageBody<'a>);
  visit!(visit_quoted_pattern, pattern, QuotedPattern<'a>);
  visit!(visit_matcher, matcher, Matcher<'a>);
  visit!(visit_variant, variant, Variant<'a>);
  visit!(visit_key, key, Key<'a>);
  visit!(visit_star, star, Star);
}

pub trait Visitable<'a> {
  fn apply_visitor<V: Visit<'a> + ?Sized>(&'a self, visitor: &mut V);

  fn apply_visitor_to_children<V: Visit<'a> + ?Sized>(
    &'a self,
    visitor: &mut V,
  );
}

pub struct AnyNodeVisitor<'a, F>
where
  F: FnMut(AnyNode<'a>),
{
  callback: F,
  phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, F> AnyNodeVisitor<'a, F>
where
  F: FnMut(AnyNode<'a>),
{
  pub fn new(callback: F) -> Self {
    AnyNodeVisitor {
      callback,
      phantom: std::marker::PhantomData,
    }
  }
}

macro_rules! any_visit {
  ($fn:ident, $param:ident, $type:ident) => {
    fn $fn(&mut self, $param: &'a ast::$type<'a>) {
      (self.callback)(AnyNode::$type($param));
      $param.apply_visitor_to_children(self);
    }
  };
}

impl<'a, F> Visit<'a> for AnyNodeVisitor<'a, F>
where
  F: FnMut(AnyNode<'a>),
{
  any_visit!(visit_message, message, Message);
  any_visit!(visit_pattern, msg, Pattern);
  any_visit!(visit_pattern_part, part, PatternPart);
  any_visit!(visit_text, text, Text);
  fn visit_escape(&mut self, escape: &'a ast::Escape) {
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
  any_visit!(visit_private_use_annotation, ann, PrivateUseAnnotation);
  any_visit!(visit_reserved_body_part, part, ReservedBodyPart);
  any_visit!(visit_reserved_annotation, ann, ReservedAnnotation);
  any_visit!(visit_variable_expression, expr, VariableExpression);
  any_visit!(visit_annotation_expression, expr, AnnotationExpression);
  any_visit!(visit_markup, markup, Markup);
  any_visit!(visit_complex_message, msg, ComplexMessage);
  any_visit!(visit_declaration, decl, Declaration);
  any_visit!(visit_input_declaration, decl, InputDeclaration);
  any_visit!(visit_local_declaration, decl, LocalDeclaration);
  any_visit!(visit_reserved_statement, stmt, ReservedStatement);
  any_visit!(visit_complex_message_body, body, ComplexMessageBody);
  any_visit!(visit_quoted_pattern, pattern, QuotedPattern);
  any_visit!(visit_matcher, matcher, Matcher);
  any_visit!(visit_variant, variant, Variant);
  any_visit!(visit_key, key, Key);
  fn visit_star(&mut self, star: &'a ast::Star) {
    (self.callback)(AnyNode::Star(star));
    star.apply_visitor_to_children(self);
  }
}
