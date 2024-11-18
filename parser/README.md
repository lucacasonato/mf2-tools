# mf2_parser

The `mf2_parser` crate provides a parser for the Message Format 2 syntax. It can
parse any sequence of Unicode scalar values (valid UTF-8) into an AST
representing the Message Format 2 syntax. The parser has very strong error
recovery, so it can parse even very broken or incomplete input (like is common
in editors).

Use the `mf2_printer` crate to pretty-print the AST back into the human-readable
MessageFormat 2 syntax.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
mf2_parser = "0.2"
```

Then you can parse a string like this:

```rust
use mf2_parser::parse;

let (ast, diagnostics, source_text_info) = parse("Hello, {$name}!");
if !diagnostics.is_empty() {
  panic!("Failed to parse message: {:?}", diagnostics);
}

println!("AST: {:?}", ast);
```

## License

This project is licensed under GPL-3.0-or-later.
