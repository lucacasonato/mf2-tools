use crate::ast;

pub trait Visit {
  fn visit_simple_message(&self, msg: &ast::SimpleMessage) {
    msg.visit_children_with(self);
  }

  fn visit_message_part(&self, part: &ast::MessagePart) {
    part.visit_children_with(self);
  }

  fn visit_text(&self, text: &ast::Text) {
    text.visit_children_with(self);
  }

  fn visit_escape(&self, escape: &ast::Escape) {
    escape.visit_children_with(self);
  }

  fn visit_expression(&self, expr: &ast::Expression) {
    expr.visit_children_with(self);
  }

  fn visit_literal_expression(&self, expr: &ast::LiteralExpression) {
    expr.visit_children_with(self);
  }

  fn visit_literal(&self, lit: &ast::Literal) {
    lit.visit_children_with(self);
  }

  fn visit_quoted(&self, quoted: &ast::Quoted) {
    quoted.visit_children_with(self);
  }

  fn visit_quoted_part(&self, part: &ast::QuotedPart) {
    part.visit_children_with(self);
  }

  fn visit_number(&self, num: &ast::Number) {
    num.visit_children_with(self);
  }

  fn visit_annotation(&self, ann: &ast::Annotation) {
    ann.visit_children_with(self);
  }

  fn visit_function(&self, func: &ast::Function) {
    func.visit_children_with(self);
  }

  fn visit_identifier(&self, ident: &ast::Identifier) {
    ident.visit_children_with(self);
  }

  fn visit_fn_or_markup_option(&self, opt: &ast::FnOrMarkupOption) {
    opt.visit_children_with(self);
  }

  fn visit_literal_or_variable(&self, lit_or_var: &ast::LiteralOrVariable) {
    lit_or_var.visit_children_with(self);
  }

  fn visit_variable(&self, var: &ast::Variable) {
    var.visit_children_with(self);
  }

  fn visit_attribute(&self, attr: &ast::Attribute) {
    attr.visit_children_with(self);
  }

  fn visit_private_use_annotation(&self, ann: &ast::PrivateUseAnnotation) {
    ann.visit_children_with(self);
  }

  fn visit_reserved_body_part(&self, part: &ast::ReservedBodyPart) {
    part.visit_children_with(self);
  }

  fn visit_reserved_annotation(&self, ann: &ast::ReservedAnnotation) {
    ann.visit_children_with(self);
  }

  fn visit_variable_expression(&self, expr: &ast::VariableExpression) {
    expr.visit_children_with(self);
  }

  fn visit_annotation_expression(&self, expr: &ast::AnnotationExpression) {
    expr.visit_children_with(self);
  }

  fn visit_markup(&self, markup: &ast::Markup) {
    markup.visit_children_with(self);
  }
}

pub trait VisitWith {
  fn visit_with<V: Visit + ?Sized>(&self, visitor: &V);

  fn visit_children_with<V: Visit + ?Sized>(&self, visitor: &V);
}
