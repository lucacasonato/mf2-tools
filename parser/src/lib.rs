#![doc = include_str!("../README.md")]

use ast::Message;
use parser::Parser;

pub mod ast;
mod chars;
mod diagnostic;
mod parser;
mod scope;
mod text;
mod visitor;

pub use diagnostic::Diagnostic;
pub use scope::Scope;
pub use text::{
  LineColUtf16, LineColUtf8, Location, SourceTextInfo, Span, Spanned,
};
pub use visitor::{Visit, VisitAny, Visitable};

/// Parse a message and return the AST, diagnostics, and source text info.
///
/// The parser is able to successfully parse any string into an AST, even if the
/// string contains invalid syntax. The diagnostics will contain any errors that
/// were found during parsing. Some diagnostics are considered "fatal", because
/// they signal that the parser was unable to recover from the error and the AST
/// may be incomplete or incorrect. Other non-fatal diagnostics will result in
/// an AST that fully represents the input string, but may be invalid in some
/// other way (like escaping a character that can not be escaped).
///
/// The source text info contains the original source text and the line and
/// column information for each character in the source text. This is useful for
/// mapping locations in the AST or the diagnostics back to actual locations in
/// the source text.
///
/// ### Example
///
/// ```rust
/// use mf2_parser::parse;
///
/// let (ast, diagnostics, source_text_info) = parse("Hello, {$name}!");
/// if !diagnostics.is_empty() {
///   panic!("Failed to parse message: {:?}", diagnostics);
/// }
///
/// println!("AST: {:?}", ast);
/// ```
pub fn parse(message: &str) -> (Message, Vec<Diagnostic>, SourceTextInfo) {
  Parser::new(message).parse()
}

pub fn analyse_semantics<'text>(
  message: &Message<'text>,
  diagnostics: &mut Vec<Diagnostic<'text>>,
) -> Scope<'text> {
  Scope::analyse(message, diagnostics)
}

/// Check if a string is a syntactically valid name in MF2.
pub fn is_valid_name(name: &str) -> bool {
  let mut ch_it = name.chars();

  matches!(ch_it.next(), Some(chars::name_start!()))
    && ch_it.all(|c| matches!(c, chars::name!()))
}
