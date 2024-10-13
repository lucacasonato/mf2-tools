mod printer;

use mf2_parser::ast::Message;
use mf2_parser::SourceTextInfo;
use printer::Printer;

pub fn print(ast: &Message, info: Option<&SourceTextInfo>) -> String {
  Printer::new(ast, info).print()
}
