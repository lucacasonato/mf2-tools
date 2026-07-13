use crate::Location;
use crate::Span;
use crate::Spanned;

pub struct Resource<'text> {
  pub span: Span,
  pub comments: Vec<Comment<'text>>,
  pub meta: Vec<Metadata<'text>>,
  pub sections: Vec<Section<'text>>,
}

impl Spanned for Resource<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

pub struct Comment<'text> {
  pub start: Location,
  pub text: &'text str,
}

impl Spanned for Comment<'_> {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + self.text)
  }
}

pub struct Name<'text> {
  // A name must have at least one part, if the name is empty, it is represented as a `NameText` with an empty string.
  pub parts: Vec<NamePart<'text>>,
}

impl Spanned for Name<'_> {
  fn span(&self) -> Span {
    let start = self.parts.first().map(|p| p.span().start).unwrap();
    let end = self.parts.last().map(|p| p.span().end).unwrap();
    Span::new(start..end)
  }
}

pub enum NamePart<'text> {
  Text(NameText<'text>),
  Escape(NameEscape),
}

impl Spanned for NamePart<'_> {
  fn span(&self) -> Span {
    match self {
      NamePart::Text(t) => t.span(),
      NamePart::Escape(e) => e.span(),
    }
  }
}

pub struct NameText<'text> {
  pub start: Location,
  pub text: &'text str,
}

impl Spanned for NameText<'_> {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + self.text)
  }
}

pub struct NameEscape {
  pub span: Span,
  pub escaped: Escaped,
}

enum Escaped {
  NewlineCRLF,
  NewlineLF,
  Char(char),
  Hex(u8),
  UnicodeShort(u16),
  UnicodeLong(u32),
}

impl Spanned for NameEscape {
  fn span(&self) -> Span {
    self.span
  }
}

pub struct Value<'text> {
  // A value must have at least one part, if the value is empty, it is represented as a `ValueText` with an empty string.
  pub parts: Vec<ValuePart<'text>>,
}

impl Spanned for Value<'_> {
  fn span(&self) -> Span {
    let start = self.parts.first().map(|p| p.span().start).unwrap();
    let end = self.parts.last().map(|p| p.span().end).unwrap();
    Span::new(start..end)
  }
}

pub enum ValuePart<'text> {
  Text(ValueText<'text>),
  Escape(ValueEscape),
}

impl Spanned for ValuePart<'_> {
  fn span(&self) -> Span {
    match self {
      ValuePart::Text(t) => t.span(),
      ValuePart::Escape(e) => e.span(),
    }
  }
}

pub struct ValueText<'text> {
  pub start: Location,
  pub text: &'text str,
}

impl Spanned for ValueText<'_> {
  fn span(&self) -> Span {
    Span::new(self.start..self.start + self.text)
  }
}

pub struct ValueEscape {
  pub span: Span,
  pub escaped: Escaped,
}

impl Spanned for ValueEscape {
  fn span(&self) -> Span {
    self.span
  }
}

pub struct Metadata<'text> {
  pub start: Location,
  pub key: Name<'text>,
  pub value: Value<'text>,
}

impl Spanned for Metadata<'_> {
  fn span(&self) -> Span {
    Span::new(self.start..self.value.span().end)
  }
}

pub struct Section<'text> {
  /// Section has no head if it is the first section in the resource.
  pub head: Option<SectionHead<'text>>,
  pub entries: Vec<Entry<'text>>,
}

impl Spanned for Section<'_> {
  fn span(&self) -> Span {
    let start = self
      .head
      .as_ref()
      .map(|h| h.span().start)
      .unwrap_or_else(|| self.entries.first().map(|e| e.span().start).unwrap());
    let end = self.entries.last().map(|e| e.span().end).unwrap();
    Span::new(start..end)
  }
}

pub struct SectionHead<'text> {
  pub comments: Vec<Comment<'text>>,
  pub meta: Vec<Metadata<'text>>,
  pub id: SectionId<'text>,
}

impl SectionHead<'_> {
  pub fn span(&self) -> Span {
    let start = self
      .comments
      .first()
      .map(|c| c.span().start)
      .unwrap_or_else(|| {
        self
          .meta
          .first()
          .map(|m| m.span().start)
          .unwrap_or(self.id.span.start)
      });
    let end = self.id.span.end;
    Span::new(start..end)
  }
}

pub struct SectionId<'text> {
  pub span: Span,
  pub id: Id<'text>,
  pub has_closing_brace: bool,
}

impl Spanned for SectionId<'_> {
  fn span(&self) -> Span {
    self.span
  }
}

pub struct Id<'text> {
  /// The parts of the ID, in order. Each part is either a name or a dot.
  /// This is always non-empty.
  pub parts: Vec<IdPart<'text>>,
}

impl Spanned for Id<'_> {
  fn span(&self) -> Span {
    let start = self.parts.first().map(|p| p.span().start).unwrap();
    let end = self.parts.last().map(|p| p.span().end).unwrap();
    Span::new(start..end)
  }
}

pub enum IdPart<'text> {
  Name(Name<'text>),
  Dot(Dot),
}

impl Spanned for IdPart<'_> {
  fn span(&self) -> Span {
    match self {
      IdPart::Name(n) => n.span(),
      IdPart::Dot(d) => d.span(),
    }
  }
}

pub struct Dot {
  pub location: Location,
}

impl Spanned for Dot {
  fn span(&self) -> Span {
    Span::new(self.location..self.location + '.')
  }
}

pub struct Entry<'text> {
  pub key: Id<'text>,
  pub value: Value<'text>,
}

impl Spanned for Entry<'_> {
  fn span(&self) -> Span {
    Span::new(self.key.span().start..self.value.span().end)
  }
}
