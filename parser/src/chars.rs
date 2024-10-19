macro_rules! content {
  () => {
    '\x01'..='\x08' | '\x0B'..='\x0C' | '\x0E'..='\x1F' | '\x21'..='\x2D' |
    '\x2F'..='\x3F' | '\x41'..='\x5B' | '\x5D'..='\x7A' | '\x7E'..='\u{2FFF}' |
    '\u{3001}'..='\u{D7FF}' | '\u{E000}'..='\u{10FFFF}'
  };
}
pub(crate) use content;

macro_rules! space {
  () => {
    ' ' | '\t' | '\r' | '\n' | '\u{3000}'
  };
}
pub(crate) use space;

macro_rules! name_start {
  () => {
    'a'..='z' | 'A'..='Z' | '_' |
    '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}' |
    '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}' |
    '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' |
    '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFC}' | '\u{10000}'..='\u{EFFFF}'
  };
}
pub(crate) use name_start;

macro_rules! name {
  () => {
    crate::chars::name_start!() |
    '0'..='9' | '-' | '.' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
  };
}
pub(crate) use name;

macro_rules! quoted {
  () => {
    crate::parser::chars::content!()
      | crate::parser::chars::space!()
      | '.'
      | '@'
      | '{'
      | '}'
  };
}
pub(crate) use quoted;
