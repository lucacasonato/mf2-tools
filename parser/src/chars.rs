macro_rules! content {
  () => {
    '\x01'..='\x08' | '\x0B'..='\x0C' | '\x0E'..='\x1F' | '\x21'..='\x2D' |
    '\x2F'..='\x3F' | '\x41'..='\x5B' | '\x5D'..='\x7A' | '\x7E'..='\u{2FFF}' |
    '\u{3001}'..='\u{D7FF}' | '\u{E000}'..='\u{10FFFF}'
  };
}
pub(crate) use content;

macro_rules! whitespace {
  () => {
    ' ' | '\t' | '\r' | '\n' | '\u{3000}'
  };
}
pub(crate) use whitespace;

macro_rules! bidi {
  () => {
    '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{2066}'..='\u{2069}'
  };
}
pub(crate) use bidi;

macro_rules! optional_space {
  () => {
    crate::chars::whitespace!() | crate::chars::bidi!()
  };
}
pub(crate) use optional_space;

macro_rules! name_start {
  () => {
    'a'..='z' | 'A'..='Z' | '_' |
    '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}' |
    '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{61B}' | '\u{61D}'..='\u{1FFF}' |
    '\u{200C}'..='\u{200D}' | '\u{2070}'..='\u{218F}' |'\u{2C00}'..='\u{2FEF}' |
    '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' |
    '\u{FDF0}'..='\u{FFFC}' | '\u{10000}'..='\u{EFFFF}'
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
      | crate::parser::chars::whitespace!()
      | '.'
      | '@'
      | '{'
      | '}'
  };
}
pub(crate) use quoted;
