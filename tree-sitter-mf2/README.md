# tree-sitter-mf2

Tree-sitter grammar for MessageFormat 2 (MF2).

## Scope

This grammar is aligned with the token model used by the VS Code TextMate
grammar in this repository (`tools/vscode/grammar.ts`) and is intended for
editor syntax highlighting and incremental parsing.

It focuses on the core MF2 constructs:

- Simple messages with text, escapes, placeholders
- Complex messages with `.input`, `.local`, and `.match`
- Expressions, annotations, options, and attributes
- Markup tags (`{#tag ...}`, `{/tag}`)
- Quoted patterns (`{{ ... }}`)
