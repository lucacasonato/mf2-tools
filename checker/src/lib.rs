//! Type system and checker for MessageFormat 2 functions.

pub mod ast;
mod parse_helpers;

use lalrpop_util::lalrpop_mod;

lalrpop_mod!(
  #[allow(clippy::all)]
  grammar
);

use ast::Document;

/// Parse a `.mft` source string into a [`Document`].
///
/// Returns a human-readable error string on failure.
pub fn parse(src: &str) -> Result<Document, String> {
  if src.len() > u16::MAX as usize {
    return Err(format!(".mft source is longer than {} bytes", u16::MAX));
  }
  grammar::DocumentParser::new()
    .parse(src)
    .map_err(|e| e.to_string())
}
