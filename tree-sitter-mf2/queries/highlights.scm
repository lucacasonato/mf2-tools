"{{" @punctuation.bracket
"}}" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"|" @punctuation.delimiter
":" @punctuation.delimiter
"=" @operator
"@" @punctuation.special
"$" @punctuation.special
"*" @character.special

[(input_declaration)
 (local_declaration)
 (matcher_statement)] @keyword

(function_annotation) @function
(identifier) @variable
(variable (identifier) @variable)
(number) @number
(quoted_literal) @string
(quoted_literal_text) @string
(text) @string
(quoted_text) @string
(escape) @escape
(private_use_annotation) @punctuation.special
(markup_open (identifier) @tag)
(markup_close (identifier) @tag)
(option (identifier) @property)
(attribute (identifier) @property)
