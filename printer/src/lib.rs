mod printer;

use mf2_parser::ast::Message;
use mf2_parser::SourceTextInfo;
use printer::Printer;

/// Print the given message as a string. If [SourceTextInfo] is provided, the
/// printer will use it to attempt to preserve some original empty line
/// placements.
pub fn print(ast: &Message, info: Option<&SourceTextInfo>) -> String {
  Printer::new(ast, info).print()
}
