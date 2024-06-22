use ast::SimpleMessage;
use parser::Parser;

pub mod ast;
mod parser;
mod util;
mod visitor;

pub use util::{Location, Span, Spanned};
pub use visitor::{Visit, VisitWith};

pub fn parse(message: &str) -> SimpleMessage {
  Parser::new(message).parse()
}

#[cfg(test)]
mod tests {

  #[test]
  fn test() {}
}
