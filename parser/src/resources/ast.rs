use crate::Location;
use crate::Span;

pub struct Resource<'text> {
  pub comments: Vec<Comment<'text>>,
  pub meta: Vec<Metadata<'text>>,
  pub frontmatter: Option<Frontmatter>,
  pub sections: Vec<Section<'text>>,
}

pub struct Frontmatter {
  pub span: Span,
}

pub struct Comment<'text> {
  pub span: Span,
  pub text: &'text str,
}

pub struct Name<'text> {
  pub parts: Vec<NamePart<'text>>,
}

pub enum NamePart<'text> {
  Text(Text<'text>),
  Escape(Escape),
}

pub struct Text<'text> {
  pub start: Location,
  pub text: &'text str,
}

pub struct Escape {
  pub start: Location,
  pub escaped_char: char,
}

pub struct Value<'text> {
  pub span: Span,
  pub lines: Vec<ValueLine<'text>>,
}

pub struct ValueLine<'text> {
  pub parts: Vec<ValuePart<'text>>,
}

pub enum ValuePart<'text> {
  Text(Text<'text>),
  Escape(Escape),
}

pub struct Metadata<'text> {
  pub at_sign: Location,
  pub key: Name<'text>,
  pub value: Value<'text>,
}

pub struct Section<'text> {
  pub span: Span,
  /// Section has no head if it is the first section in the resource.
  pub head: Option<SectionHead<'text>>,
  pub entries: Vec<Entry<'text>>,
}

pub struct SectionHead<'text> {
  pub comments: Option<Comment<'text>>,
  pub meta: Vec<Metadata<'text>>,
  pub id: SectionId<'text>,
}

pub struct SectionId<'text> {
  pub span: Span,
  pub id: Id<'text>,
  pub has_closing_brace: bool,
}

pub struct Id<'text> {
  /// The parts of the ID, in order. Each part is either a name or a dot.
  pub parts: Vec<IdPart<'text>>,
}

pub enum IdPart<'text> {
  Name(Name<'text>),
  Dot(Dot),
}

pub struct Dot {
  pub location: Location,
}

pub struct Entry<'text> {
  pub span: Span,
  pub key: Id<'text>,
  pub value: Value<'text>,
}
