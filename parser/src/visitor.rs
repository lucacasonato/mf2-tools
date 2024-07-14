use crate::ast;

macro_rules! visit {
  ($fn:ident, $param:ident, $type:ident) => {
    fn $fn(&mut self, $param: &ast::$type) {
      $param.apply_visitor_to_children(self);
    }
  };
}

pub trait Visit {
  visit!(visit_message, message, Message);
  visit!(visit_pattern, msg, Pattern);
  visit!(visit_message_part, part, MessagePart);
  visit!(visit_text, text, Text);
  visit!(visit_escape, escape, Escape);
  visit!(visit_expression, expr, Expression);
  visit!(visit_literal_expression, expr, LiteralExpression);
  visit!(visit_literal, literal, Literal);
  visit!(visit_quoted, quoted, Quoted);
  visit!(visit_quoted_part, part, QuotedPart);
  visit!(visit_number, num, Number);
  visit!(visit_annotation, ann, Annotation);
  visit!(visit_function, func, Function);
  visit!(visit_identifier, ident, Identifier);
  visit!(visit_fn_or_markup_option, opt, FnOrMarkupOption);
  visit!(visit_literal_or_variable, lit_or_var, LiteralOrVariable);
  visit!(visit_variable, var, Variable);
  visit!(visit_attribute, attr, Attribute);
  visit!(visit_private_use_annotation, ann, PrivateUseAnnotation);
  visit!(visit_reserved_body_part, part, ReservedBodyPart);
  visit!(visit_reserved_annotation, ann, ReservedAnnotation);
  visit!(visit_variable_expression, expr, VariableExpression);
  visit!(visit_annotation_expression, expr, AnnotationExpression);
  visit!(visit_markup, markup, Markup);
  visit!(visit_complex_message, msg, ComplexMessage);
  visit!(visit_declaration, decl, Declaration);
  visit!(visit_input_declaration, decl, InputDeclaration);
  visit!(visit_local_declaration, decl, LocalDeclaration);
  visit!(visit_reserved_statement, stmt, ReservedStatement);
  visit!(visit_complex_message_body, body, ComplexMessageBody);
  visit!(visit_quoted_pattern, pattern, QuotedPattern);
  visit!(visit_matcher, matcher, Matcher);
  visit!(visit_variant, variant, Variant);
  visit!(visit_key, key, Key);
  visit!(visit_star, star, Star);}

pub trait Visitable {
  fn apply_visitor<V: Visit + ?Sized>(&self, visitor: &mut V);

  fn apply_visitor_to_children<V: Visit + ?Sized>(&self, visitor: &mut V);
}
