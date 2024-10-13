# mf2-tools

This repository contains various tools for working with the
[Message Format 2](https://messageformat.dev) localization system from Unicode.

- [mf2lsp](#mf2lsp): A language server for Message Format 2.
- [vscode-mf2](#vscode-mf2): A VS Code extension for Message Format 2.
- [mf2_parser](#rust-crates): A Rust parser for Message Format 2.
- [mf2_printer](#rust-crates): A Rust pretty-printer for Message Format 2.

## mf2lsp

The `mf2lsp` language server provides language support for Message Format 2 in
editors that support the Language Server Protocol. It is implemented in Rust and
can be run as a standalone executable or via Wasm (for use in web-based
editors).

`mf2lsp` is still in early development, but it already has a relatively rich
feature set:

- Semantic highlighting
- Diagnostics (syntax errors, early errors)
- Variable completion
- Quick fixes for some errors
- Formatting
- Hover information
- Go to definition for variables

To use `mf2lsp` in VS Code, you can install the [vscode-mf2](#vscode-mf2)
extension.

For use in other editors with LSP support, you can run `mf2lsp` as a standalone
executable. You can find the latest release on the
[releases page](https://github.com/lucacasonato/mf2-tools/releases).

## vscode-mf2

The `vscode-mf2` extension provides support for Message Format 2 in Visual
Studio Code. It uses the `mf2lsp` language server to provide all the features
mentioned above, but also includes a language grammar that enables syntax
highlighting and bracket matching.

You can install the extension from the Visual Studio Code marketplace:
https://marketplace.visualstudio.com/items?itemName=nicolo-ribaudo.vscode-mf2.

## Rust Crates

This repository also contains two Rust crates for working with Message Format 2.

The `mf2_parser` crate provides a parser for the Message Format 2 syntax. It can
parse any sequence of Unicode scalar values (valid UTF-8) into an AST
representing the Message Format 2 syntax. The parser has very strong error
recovery, so it can parse even very broken or incomplete input (like is common
in editors).

The `mf2_printer` crate provides a pretty-printer for the Message Format 2 AST.
It can take an AST and convert it back to a string, preserving some of the
original formatting (like empty lines).

## Development

To build the language server, you will need to have Rust installed. You can
build the language server by running:

```sh
cargo build
```

To use your local build of the language server in VS Code, you can set the
`mf2.server.path` option to the path of the built executable. It will be located
at `<path-to-this-repo>/target/debug/mf2lsp`.

You can run tests by running:

```sh
cargo test && deno task test
```

To regenerate expected test output after modifying the parser / ast / printer,
run:

```sh
UPDATE=1 cargo test
```
