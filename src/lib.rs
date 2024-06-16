use ast::SimpleMessage;
use parser::Parser;

pub mod ast;
mod parser;
mod types;
mod util;

pub use util::{Location, Span, Spanned};

pub fn parse(message: &str) -> SimpleMessage {
  Parser::new(message).parse()
}

#[cfg(test)]
mod tests {

  #[test]
  fn test() {}
}
