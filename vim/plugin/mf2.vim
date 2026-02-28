" Ensure this repository's ./vim runtime is visible to Vim/Neovim.
if exists('g:loaded_mf2_vim_runtime')
  finish
endif
let g:loaded_mf2_vim_runtime = 1

let s:repo_root = fnamemodify(expand('<sfile>:p'), ':h:h')
let s:vim_runtime = s:repo_root . '/vim'
if isdirectory(s:vim_runtime)
  execute 'set runtimepath^=' . fnameescape(s:vim_runtime)
endif
