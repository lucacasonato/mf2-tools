//! A library for pretty-printing MessageFormat 2 ASTs. Use in combination with
//! the `mf2_parser` crate to parse MessageFormat 2 strings and then print them
//! back out in a human-readable format.
//!
//! # Example
//!
//! ```rust
//! use mf2_parser::parse;
//! use mf2_printer::print;
//!
//! let input = "Hello, {   name  }!";
//! let (ast, diagnostics, info) = parse(input);
//! if !diagnostics.is_empty() {
//!   panic!("Failed to parse input: {:?}", diagnostics);
//! }
//! let pretty = print(&ast, None);
//! assert_eq!(pretty, "Hello, { name }!");
//! ```

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
