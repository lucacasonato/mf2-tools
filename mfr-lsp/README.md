# mfrlsp

`mfrlsp` is a diagnostics-first Language Server for MessageFormat Resource (`.mfr`) files.

Pipeline:
1. Parse full document structure with `mfr_parser`.
2. Parse each extracted resource value with `mf2_parser`.
3. Merge structural and value diagnostics and publish them at document coordinates.

Build:

```sh
cargo build -p mfrlsp --release
```
