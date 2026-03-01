# mfr_parser

`mfr_parser` parses MessageFormat Resource (`.mfr`) files using the repository `abnf` as the structural source of truth.

It provides:
- Resource line classification (`frontmatter`, `section`, `entry`, `metadata`, `comment`, `empty`, `invalid`)
- Structural diagnostics with spans
- Extracted entry/metadata values suitable for MF2 value parsing

Primary API:

```rust
use mfr_parser::parse_resource;

let (doc, diagnostics, info) = parse_resource(source_text);
```
