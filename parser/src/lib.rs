use ast::Message;
use parser::Parser;

pub mod ast;
mod chars;
mod diagnostic;
mod parser;
mod util;
mod visitor;

pub use diagnostic::Diagnostic;
pub use util::{
  LineColUtf16, LineColUtf8, Location, SourceTextInfo, Span, Spanned,
};
pub use visitor::{Visit, VisitAny, Visitable};

pub fn parse(message: &str) -> (Message, Vec<Diagnostic>, SourceTextInfo) {
  Parser::new(message).parse()
}

pub fn is_valid_name(name: &str) -> bool {
  let mut ch_it = name.chars();

  matches!(ch_it.next(), Some(chars::name_start!()))
    && ch_it.all(|c| matches!(c, chars::name!()))
}
