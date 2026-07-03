use crate::Location;
use crate::Span;

pub struct Resource<'text> {
  pub span: Span,
  pub comments: Vec<Comment<'text>>,
  pub meta: Vec<Metadata<'text>>,
  pub sections: Vec<Section<'text>>,
}

pub struct Comment<'text> {
  pub start: Location,
  pub text: &'text str,
}

pub struct Name<'text> {
  pub parts: Vec<NamePart<'text>>,
}

pub enum NamePart<'text> {
  Text(NameText<'text>),
  Escape(NameEscape),
}

pub struct NameText<'text> {
  pub start: Location,
  pub text: &'text str,
}

pub struct NameEscape {
  pub start: Location,
  pub escaped_char: char,
}

pub struct Value<'text> {
  pub parts: Vec<ValuePart<'text>>,
}

pub enum ValuePart<'text> {
  Text(ValueText<'text>),
  Escape(ValueEscape),
}

pub struct ValueText<'text> {
  pub start: Location,
  pub text: &'text str,
}

pub struct ValueEscape {
  pub start: Location,
  pub escaped_char: char,
}

pub struct Metadata<'text> {
  pub start: Location,
  pub key: Name<'text>,
  pub value: Value<'text>,
}

pub struct Section<'text> {
  /// Section has no head if it is the first section in the resource.
  pub head: Option<SectionHead<'text>>,
  pub entries: Vec<Entry<'text>>,
}

pub struct SectionHead<'text> {
  pub comments: Vec<Comment<'text>>,
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
  pub key: Id<'text>,
  pub value: Value<'text>,
}
