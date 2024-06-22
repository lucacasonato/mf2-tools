use crate::ast;

pub trait Visit {
  fn visit_simple_message(&self, msg: &ast::SimpleMessage) {
    msg.apply_visitor_to_children(self);
  }

  fn visit_message_part(&self, part: &ast::MessagePart) {
    part.apply_visitor_to_children(self);
  }

  fn visit_text(&self, text: &ast::Text) {
    text.apply_visitor_to_children(self);
  }

  fn visit_escape(&self, escape: &ast::Escape) {
    escape.apply_visitor_to_children(self);
  }

  fn visit_expression(&self, expr: &ast::Expression) {
    expr.apply_visitor_to_children(self);
  }

  fn visit_literal_expression(&self, expr: &ast::LiteralExpression) {
    expr.apply_visitor_to_children(self);
  }

  fn visit_literal(&self, lit: &ast::Literal) {
    lit.apply_visitor_to_children(self);
  }

  fn visit_quoted(&self, quoted: &ast::Quoted) {
    quoted.apply_visitor_to_children(self);
  }

  fn visit_quoted_part(&self, part: &ast::QuotedPart) {
    part.apply_visitor_to_children(self);
  }

  fn visit_number(&self, num: &ast::Number) {
    num.apply_visitor_to_children(self);
  }

  fn visit_annotation(&self, ann: &ast::Annotation) {
    ann.apply_visitor_to_children(self);
  }

  fn visit_function(&self, func: &ast::Function) {
    func.apply_visitor_to_children(self);
  }

  fn visit_identifier(&self, ident: &ast::Identifier) {
    ident.apply_visitor_to_children(self);
  }

  fn visit_fn_or_markup_option(&self, opt: &ast::FnOrMarkupOption) {
    opt.apply_visitor_to_children(self);
  }

  fn visit_literal_or_variable(&self, lit_or_var: &ast::LiteralOrVariable) {
    lit_or_var.apply_visitor_to_children(self);
  }

  fn visit_variable(&self, var: &ast::Variable) {
    var.apply_visitor_to_children(self);
  }

  fn visit_attribute(&self, attr: &ast::Attribute) {
    attr.apply_visitor_to_children(self);
  }

  fn visit_private_use_annotation(&self, ann: &ast::PrivateUseAnnotation) {
    ann.apply_visitor_to_children(self);
  }

  fn visit_reserved_body_part(&self, part: &ast::ReservedBodyPart) {
    part.apply_visitor_to_children(self);
  }

  fn visit_reserved_annotation(&self, ann: &ast::ReservedAnnotation) {
    ann.apply_visitor_to_children(self);
  }

  fn visit_variable_expression(&self, expr: &ast::VariableExpression) {
    expr.apply_visitor_to_children(self);
  }

  fn visit_annotation_expression(&self, expr: &ast::AnnotationExpression) {
    expr.apply_visitor_to_children(self);
  }

  fn visit_markup(&self, markup: &ast::Markup) {
    markup.apply_visitor_to_children(self);
  }
}

pub trait Visitable {
  fn apply_visitor<V: Visit + ?Sized>(&self, visitor: &V);

  fn apply_visitor_to_children<V: Visit + ?Sized>(&self, visitor: &V);
}
