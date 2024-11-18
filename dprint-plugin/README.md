# dprint-plugin-mf2

[dprint](https://dprint.dev) is a pluggable and configurable code formatting
platform written in Rust.

This plugin adds MF2 support to dprint.

## Usage

To add the MF2 plugin to your dprint configuration, just run:

```
dprint add lucacasonato/mf2-tools
```

Alternatively, add the plugin URL into your `dprint.json` config file:

```json
{
  ...,
  "plugins": [
    "https://plugins.dprint.dev/lucacasonato/mf2-tools-0.1.0.wasm"
  ]
}
```

## License

This project is licensed under GPL-3.0-or-later.
