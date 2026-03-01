# Vim Support

This directory contains classic Vim support for MessageFormat 2:

- `plugin/mf2.vim`
- `ftdetect/mf2.vim`
- `ftdetect/mfr.vim`
- `syntax/mf2.vim`
- `syntax/mfr.vim`

For Neovim, prefer the Tree-sitter grammar in `../tree-sitter-mf2/`.

## Install in Classic Vim

Using `vim-plug` (common setup), add this to your `.vimrc`:

```vim
call plug#begin('~/.vim/plugged')
Plug 'lucacasonato/mf2-tools', { 'rtp': 'vim' }
call plug#end()
```

Then run `:PlugInstall`.

If you use another plugin manager, point to this repository (or a fork) and
make sure the `vim/` directory is included in runtimepath.

For a manual install, copy these files into your user config:

- `plugin/mf2.vim` -> `~/.vim/plugin/mf2.vim`
- `syntax/mf2.vim` -> `~/.vim/syntax/mf2.vim`
- `syntax/mfr.vim` -> `~/.vim/syntax/mfr.vim`
- `ftdetect/mf2.vim` -> `~/.vim/ftdetect/mf2.vim`
- `ftdetect/mfr.vim` -> `~/.vim/ftdetect/mfr.vim`

After that, `*.mf2` files will use `filetype=mf2`, and `*.mfr` files will use
`filetype=mfr`, both with syntax
highlighting automatically.

To regenerate syntax after changing token patterns, run from repo root:

```sh
deno task vim:build
```

## Use `mf2lsp` and `mfrlsp` with `coc.nvim`

`mf2lsp` provides LSP features for `.mf2`, and `mfrlsp` provides diagnostics
for `.mfr`. Keep the Vim syntax files above for classic syntax highlighting.

Build both language servers:

```sh
cargo build -p mf2lsp --release
cargo build -p mfrlsp --release
```

Install the binary to `/usr/local/bin` (recommended), or `~/.local/bin` if you
prefer a user-local install:

```sh
cp target/release/mf2lsp /usr/local/bin/mf2lsp
cp target/release/mfrlsp /usr/local/bin/mfrlsp
# fallback:
# mkdir -p ~/.local/bin && cp target/release/mf2lsp ~/.local/bin/mf2lsp
# mkdir -p ~/.local/bin && cp target/release/mfrlsp ~/.local/bin/mfrlsp
```

Then add this to `coc-settings.json`:

```json
{
  "languageserver": {
    "mf2": {
      "command": "mf2lsp",
      "filetypes": ["mf2"],
      "rootPatterns": [".git"],
      "trace.server": "off"
    },
    "mfr": {
      "command": "mfrlsp",
      "filetypes": ["mfr"],
      "rootPatterns": [".git"],
      "trace.server": "off"
    }
  }
}
```

Restart coc (`:CocRestart`) and open a `.mf2` or `.mfr` file.
