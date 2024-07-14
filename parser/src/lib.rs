use ast::Pattern;
use parser::Parser;

pub mod ast;
mod chars;
mod diagnostic;
mod parser;
mod util;
mod visitor;

pub use diagnostic::Diagnostic;
pub use util::{Location, SourceTextInfo, Span, Spanned};
pub use visitor::{Visit, Visitable};

pub fn parse(message: &str) -> (Pattern, Vec<Diagnostic>, SourceTextInfo) {
  Parser::new(message).parse()
}

#[cfg(test)]
mod tests {

  #[test]
  fn test() {}
}
