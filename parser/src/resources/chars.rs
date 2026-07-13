macro_rules! space {
  () => {
    ' ' | '\t'
  };
}
pub(crate) use space;

macro_rules! newline {
  () => {
    '\n' | '\r'
  };
}
pub(crate) use newline;

macro_rules! id_safe {
  () => {
    'a'..='z' | 'A'..='Z' | '0'..='9' | '_' |
    '\u{00A1}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}' | '\u{2030}'..='\u{205E}' |
    '\u{2070}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' |
    '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}'
  };
}
pub(crate) use id_safe;

macro_rules! symbols {
  () => {
    '\x21'..='\x2F' | '\x3A'..='\x40' | '\x5B'..='\x60' | '\x7B'..='\x7E' |
    '\u{00A1}'..='\u{00BF}' | '\u{00D7}' | '\u{00F7}' | '\u{2010}'..='\u{2027}' |
    '\u{2030}'..='\u{205E}' | '\u{2190}'..='\u{2BFF}'
  };
}
pub(crate) use symbols;

macro_rules! value_start {
  () => {
    '\x21'..='\x5B' | '\x5D'..='\x7E' | '\u{00A0}'..='\u{2027}' |
    '\u{202A}'..='\u{D7FF}' | '\u{E000}'..='\u{10FFFF}'
  };
}
pub(crate) use value_start;

macro_rules! content {
  () => {
    ' ' | '\t' | crate::resources::chars::value_start!()
  };
}
pub(crate) use content;
