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
pub use visitor::{AnyNodeVisitor, Visit, Visitable};

pub fn parse(message: &str) -> (Message, Vec<Diagnostic>, SourceTextInfo) {
  Parser::new(message).parse()
}
