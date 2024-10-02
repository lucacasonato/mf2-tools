# mf2-language-server

This repo contains a language server for the
[Message Format 2](https://messageformat.dev) message localization system from
Unicode.

Also contained in this repo is a Rust parser for the Message Format 2 syntax.
This parser has error recovery, and can parse any sequence of Unicode scalar
values (valid UTF-8) into an AST representing the Message Format 2 syntax.

> **Note**: This project is still in the early stages of development and the
> language server is still missing many features.

## Usage

To use the language server, you will need to have a language client that can
communicate with it via the Language Server Protocol. The language server itself
is implemented in Rust, and can be run as a standalone executable.

For VS Code, you can use the
[vscode-mf2](https://marketplace.visualstudio.com/items?itemName=nicolo-ribaudo.vscode-mf2)
extension.

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

To regenerate expected test output after modifying the parser / ast, run:

```sh
UPDATE=1 cargo test
```
