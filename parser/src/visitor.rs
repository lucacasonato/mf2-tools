use crate::ast;

pub trait Visit {
  fn visit_simple_message(&mut self, msg: &ast::SimpleMessage) {
    msg.apply_visitor_to_children(self);
  }

  fn visit_message_part(&mut self, part: &ast::MessagePart) {
    part.apply_visitor_to_children(self);
  }

  fn visit_text(&mut self, text: &ast::Text) {
    text.apply_visitor_to_children(self);
  }

  fn visit_escape(&mut self, escape: &ast::Escape) {
    escape.apply_visitor_to_children(self);
  }

  fn visit_expression(&mut self, expr: &ast::Expression) {
    expr.apply_visitor_to_children(self);
  }

  fn visit_literal_expression(&mut self, expr: &ast::LiteralExpression) {
    expr.apply_visitor_to_children(self);
  }

  fn visit_literal(&mut self, lit: &ast::Literal) {
    lit.apply_visitor_to_children(self);
  }

  fn visit_quoted(&mut self, quoted: &ast::Quoted) {
    quoted.apply_visitor_to_children(self);
  }

  fn visit_quoted_part(&mut self, part: &ast::QuotedPart) {
    part.apply_visitor_to_children(self);
  }

  fn visit_number(&mut self, num: &ast::Number) {
    num.apply_visitor_to_children(self);
  }

  fn visit_annotation(&mut self, ann: &ast::Annotation) {
    ann.apply_visitor_to_children(self);
  }

  fn visit_function(&mut self, func: &ast::Function) {
    func.apply_visitor_to_children(self);
  }

  fn visit_identifier(&mut self, ident: &ast::Identifier) {
    ident.apply_visitor_to_children(self);
  }

  fn visit_fn_or_markup_option(&mut self, opt: &ast::FnOrMarkupOption) {
    opt.apply_visitor_to_children(self);
  }

  fn visit_literal_or_variable(&mut self, lit_or_var: &ast::LiteralOrVariable) {
    lit_or_var.apply_visitor_to_children(self);
  }

  fn visit_variable(&mut self, var: &ast::Variable) {
    var.apply_visitor_to_children(self);
  }

  fn visit_attribute(&mut self, attr: &ast::Attribute) {
    attr.apply_visitor_to_children(self);
  }

  fn visit_private_use_annotation(&mut self, ann: &ast::PrivateUseAnnotation) {
    ann.apply_visitor_to_children(self);
  }

  fn visit_reserved_body_part(&mut self, part: &ast::ReservedBodyPart) {
    part.apply_visitor_to_children(self);
  }

  fn visit_reserved_annotation(&mut self, ann: &ast::ReservedAnnotation) {
    ann.apply_visitor_to_children(self);
  }

  fn visit_variable_expression(&mut self, expr: &ast::VariableExpression) {
    expr.apply_visitor_to_children(self);
  }

  fn visit_annotation_expression(&mut self, expr: &ast::AnnotationExpression) {
    expr.apply_visitor_to_children(self);
  }

  fn visit_markup(&mut self, markup: &ast::Markup) {
    markup.apply_visitor_to_children(self);
  }
}

pub trait Visitable {
  fn apply_visitor<V: Visit + ?Sized>(&self, visitor: &mut V);

  fn apply_visitor_to_children<V: Visit + ?Sized>(&self, visitor: &mut V);
}
