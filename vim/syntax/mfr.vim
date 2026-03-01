" MessageFormat Resource syntax for Classic Vim.
if exists("b:current_syntax")
  finish
endif

" Reuse MF2 syntax groups first.
runtime! syntax/mf2.vim
unlet! b:current_syntax

syn case match

" Resource structure.
syn match mfrFrontmatter /^\s*---\s*$/
syn match mfrEntryKey /^\s*\zs[^#@\[\]\t ][^=]*\ze\s*=/

hi def link mfrFrontmatter Delimiter
hi def link mfrEntryKey Identifier

let b:current_syntax = "mfr"
