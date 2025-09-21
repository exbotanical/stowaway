" Vim configuration for @user@
" Theme: @theme@

" Basic settings
set nocompatible
set number
set relativenumber
set ruler
set showcmd
set showmatch
set hlsearch
set incsearch
set ignorecase
set smartcase
set autoindent
set smartindent
set tabstop=4
set shiftwidth=4
set expandtab
set wrap
set linebreak
set scrolloff=3
set sidescrolloff=5

" Theme-specific settings
if "@theme@" == "dark"
    set background=dark
    colorscheme desert
else
    set background=light
    colorscheme default
endif

" File type detection
filetype on
filetype plugin on
filetype indent on

" Syntax highlighting
syntax on

" Status line
set laststatus=2
set statusline=%F%m%r%h%w\ [FORMAT=%{&ff}]\ [TYPE=%Y]\ [POS=%l,%v][%p%%]\ %{strftime(\"%d/%m/%y\ -\ %H:%M\")}

" Key mappings
let mapleader = ","
nnoremap <leader>w :w<CR>
nnoremap <leader>q :q<CR>
nnoremap <leader>x :x<CR>

" Search and replace
nnoremap <leader>s :%s/\<<C-r><C-w>\>/

" Clear search highlighting
nnoremap <leader>/ :nohlsearch<CR>

" User-specific settings
" Editor: @editor@
" User: @user@
" Email: @email@
