use ast::SimpleMessage;
use diagnostic::Diagnostic;
use parser::Parser;

pub mod ast;
mod diagnostic;
mod parser;
mod util;
mod visitor;

pub use util::{Location, Span, Spanned};
pub use visitor::{Visit, Visitable};

pub fn parse(message: &str) -> (SimpleMessage, Vec<Diagnostic>) {
  Parser::new(message).parse()
}

#[cfg(test)]
mod tests {

  #[test]
  fn test() {}
}
