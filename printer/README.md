# mf2_printer

The `mf2_printer` crate provides a pretty-printer for the Message Format 2
syntax. It can pretty-print an AST representing the Message Format 2 syntax back
into the human-readable MessageFormat 2 syntax.

Use the `mf2_parser` crate to parse a MessageFormat 2 string into an AST.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
mf2_printer = "0.2"
```

Then you can parse a string like this:

```rust
use mf2_parser::parse;
use mf2_printer::print;

let (ast, diagnostics, source_text_info) = parse("Hello, {$name}!");
if !diagnostics.is_empty() {
  panic!("Failed to parse message: {:?}", diagnostics);
}

let pretty_printed = print(&ast, None);
println!("Pretty-printed: {}", pretty_printed);
```

## License

This project is licensed under GPL-3.0-or-later.
