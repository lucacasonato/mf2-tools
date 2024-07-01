use ast::SimpleMessage;
use parser::Parser;

pub mod ast;
mod diagnostic;
mod parser;
mod util;
mod visitor;

pub use diagnostic::Diagnostic;
pub use util::{Location, SourceTextInfo, Span, Spanned};
pub use visitor::{Visit, Visitable};

pub fn parse(
  message: &str,
) -> (SimpleMessage, Vec<Diagnostic>, SourceTextInfo) {
  Parser::new(message).parse()
}

#[cfg(test)]
mod tests {

  #[test]
  fn test() {}
}
