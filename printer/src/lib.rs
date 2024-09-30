mod printer;

use mf2_parser::ast::Message;
use printer::Printer;

pub fn print(ast: &Message) -> String {
  Printer::new(ast).print()
}
