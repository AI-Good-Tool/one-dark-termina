" One Dark Pro colorscheme for vim
" 由 One Dark Terminal 内置提供，通过 VIMINIT 注入
set background=dark
highlight clear
if exists("syntax_on")
  syntax reset
endif
let g:colors_name = "onedark"

if has("termguicolors")
  set termguicolors
endif

hi Normal        guifg=#abb2bf guibg=#282c34 ctermfg=145 ctermbg=235
hi Comment       guifg=#5c6370 gui=italic ctermfg=59 cterm=italic
hi Constant      guifg=#d19a66 ctermfg=173
hi String        guifg=#98c379 ctermfg=114
hi Character     guifg=#98c379 ctermfg=114
hi Number        guifg=#d19a66 ctermfg=173
hi Boolean       guifg=#d19a66 ctermfg=173
hi Float         guifg=#d19a66 ctermfg=173
hi Identifier    guifg=#e06c75 ctermfg=168
hi Function      guifg=#61afef ctermfg=75
hi Statement     guifg=#c678dd ctermfg=176
hi Conditional   guifg=#c678dd ctermfg=176
hi Repeat        guifg=#c678dd ctermfg=176
hi Label         guifg=#c678dd ctermfg=176
hi Keyword       guifg=#c678dd ctermfg=176
hi Exception     guifg=#c678dd ctermfg=176
hi Operator      guifg=#56b6c2 ctermfg=73
hi PreProc       guifg=#e5c07b ctermfg=180
hi Include       guifg=#c678dd ctermfg=176
hi Define        guifg=#e5c07b ctermfg=180
hi Macro         guifg=#61afef ctermfg=75
hi PreCondit     guifg=#e5c07b ctermfg=180
hi Type          guifg=#e5c07b ctermfg=180
hi StorageClass  guifg=#c678dd ctermfg=176
hi Structure     guifg=#e5c07b ctermfg=180
hi Typedef       guifg=#e5c07b ctermfg=180
hi Special       guifg=#56b6c2 ctermfg=73
hi SpecialChar   guifg=#d19a66 ctermfg=173
hi Delimiter     guifg=#abb2bf ctermfg=145
hi SpecialComment guifg=#5c6370 ctermfg=59
hi Debug         guifg=#e06c75 ctermfg=168
hi Error         guifg=#e06c75 guibg=#282c34 ctermfg=168 ctermbg=235
hi Todo          guifg=#c678dd guibg=#282c34 gui=bold ctermfg=176 ctermbg=235
hi Underlined    guifg=#61afef gui=underline ctermfg=75 cterm=underline
hi Ignore        guifg=#5c6370 ctermfg=59

" 界面元素
hi Cursor        guifg=#282c34 guibg=#528bff ctermfg=235 ctermbg=69
hi CursorLine    guibg=#2c313c ctermbg=236 cterm=NONE gui=NONE
hi CursorColumn  guibg=#2c313c ctermbg=236
hi LineNr        guifg=#4b5263 ctermfg=240
hi CursorLineNr  guifg=#abb2bf ctermfg=145
hi Visual        guibg=#3e4451 ctermbg=238
hi VisualNOS     guibg=#3e4451 ctermbg=238
hi Search        guifg=#282c34 guibg=#e5c07b ctermfg=235 ctermbg=180
hi IncSearch     guifg=#282c34 guibg=#d19a66 ctermfg=235 ctermbg=173
hi MatchParen    guifg=#61afef gui=underline ctermfg=75 cterm=underline
hi StatusLine    guifg=#abb2bf guibg=#21252b ctermfg=145 ctermbg=235
hi StatusLineNC  guifg=#5c6370 guibg=#21252b ctermfg=59 ctermbg=235
hi VertSplit     guifg=#181a1f ctermfg=234
hi WinSeparator  guifg=#181a1f ctermfg=234
hi Pmenu         guifg=#abb2bf guibg=#21252b ctermfg=145 ctermbg=235
hi PmenuSel      guifg=#ffffff guibg=#3e4451 ctermfg=231 ctermbg=238
hi PmenuSbar     guibg=#3e4451 ctermbg=238
hi PmenuThumb    guibg=#5c6370 ctermbg=59
hi Folded        guifg=#5c6370 guibg=#21252b ctermfg=59 ctermbg=235
hi FoldColumn    guifg=#5c6370 guibg=#282c34 ctermfg=59 ctermbg=235
hi SignColumn    guibg=#282c34 ctermbg=235
hi ColorColumn   guibg=#2c313c ctermbg=236
hi NonText       guifg=#3e4451 ctermfg=238
hi SpecialKey    guifg=#3e4451 ctermfg=238
hi Whitespace    guifg=#3e4451 ctermfg=238
hi EndOfBuffer   guifg=#282c34 ctermfg=235
hi Directory     guifg=#61afef ctermfg=75
hi Title         guifg=#98c379 ctermfg=114
hi WarningMsg    guifg=#e5c07b ctermfg=180
hi ErrorMsg      guifg=#e06c75 ctermfg=168
hi MoreMsg       guifg=#98c379 ctermfg=114
hi Question      guifg=#98c379 ctermfg=114
hi DiffAdd       guifg=#98c379 guibg=#2c3a2e ctermfg=114 ctermbg=236
hi DiffChange    guifg=#e5c07b guibg=#3a352c ctermfg=180 ctermbg=236
hi DiffDelete    guifg=#e06c75 guibg=#3a2c30 ctermfg=168 ctermbg=236
hi DiffText      guifg=#61afef guibg=#2c3340 ctermfg=75 ctermbg=236

" HTML / Vue 模板
hi htmlTag       guifg=#abb2bf ctermfg=145
hi htmlEndTag    guifg=#abb2bf ctermfg=145
hi htmlTagName   guifg=#e06c75 ctermfg=168
hi htmlArg       guifg=#d19a66 ctermfg=173
hi htmlTitle     guifg=#abb2bf ctermfg=145
hi htmlH1        guifg=#abb2bf ctermfg=145

" CSS / SCSS（对齐 VS Code One Dark Pro）
hi cssProp            guifg=#d19a66 ctermfg=173
hi cssAttr            guifg=#d19a66 ctermfg=173
hi cssAttrComma       guifg=#c678dd ctermfg=176
hi cssClassName       guifg=#e5c07b ctermfg=180
hi cssClassNameDot    guifg=#e5c07b ctermfg=180
hi cssIdentifier      guifg=#61afef ctermfg=75
hi cssTagName         guifg=#e06c75 ctermfg=168
hi cssFunctionName    guifg=#61afef ctermfg=75
hi cssImportant       guifg=#c678dd ctermfg=176
hi cssBraces          guifg=#abb2bf ctermfg=145
hi cssDefinition      guifg=#c678dd ctermfg=176
hi cssPseudoClassId   guifg=#e5c07b ctermfg=180
hi cssPseudoClass     guifg=#e5c07b ctermfg=180
hi cssVendor          guifg=#abb2bf ctermfg=145
hi cssMedia           guifg=#c678dd ctermfg=176
hi cssInclude         guifg=#c678dd ctermfg=176
hi cssIncludeKeyword  guifg=#c678dd ctermfg=176
hi cssColor           guifg=#d19a66 ctermfg=173
hi cssValueNumber     guifg=#d19a66 ctermfg=173
hi cssValueLength     guifg=#d19a66 ctermfg=173
hi cssUnitDecorators  guifg=#d19a66 ctermfg=173
hi scssVariable       guifg=#e06c75 ctermfg=168
hi scssVariableValue  guifg=#d19a66 ctermfg=173
hi scssInterpolation  guifg=#e06c75 ctermfg=168
hi scssSelectorChar   guifg=#e5c07b ctermfg=180
hi scssSelectorName   guifg=#e5c07b ctermfg=180
hi scssAmpersand      guifg=#e06c75 ctermfg=168
hi scssMixin          guifg=#c678dd ctermfg=176
hi scssExtend         guifg=#c678dd ctermfg=176
hi scssImport         guifg=#c678dd ctermfg=176
hi scssInclude        guifg=#c678dd ctermfg=176
hi scssFunctionName   guifg=#61afef ctermfg=75
hi scssDefinition     guifg=#c678dd ctermfg=176
hi scssProperty       guifg=#d19a66 ctermfg=173
hi sassVariable       guifg=#e06c75 ctermfg=168
hi lessVariable       guifg=#e06c75 ctermfg=168
hi lessClass          guifg=#e5c07b ctermfg=180

" JavaScript / TypeScript
hi jsVariableDef guifg=#e06c75 ctermfg=168
hi jsFuncCall    guifg=#61afef ctermfg=75
hi jsObjectKey   guifg=#e06c75 ctermfg=168
hi jsThis        guifg=#c678dd ctermfg=176
hi typescriptBraces guifg=#abb2bf ctermfg=145
