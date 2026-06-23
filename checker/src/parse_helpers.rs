use crate::ast::{Bound, FieldDecl, Impl, MemberDecl, Needs, Span, UnionBound};

pub enum TraitItem {
  Member(MemberDecl),
  Needs(Needs),
}

pub enum TypeItem {
  Field(FieldDecl),
  Impl(Impl),
}

pub fn split_trait_items(items: Vec<TraitItem>) -> (Vec<MemberDecl>, Vec<Needs>) {
  let mut members = Vec::new();
  let mut needs = Vec::new();
  for item in items {
    match item {
      TraitItem::Member(m) => members.push(m),
      TraitItem::Needs(n) => needs.push(n),
    }
  }
  (members, needs)
}

pub fn split_type_items(items: Vec<TypeItem>) -> (Vec<FieldDecl>, Vec<Impl>) {
  let mut fields = Vec::new();
  let mut impls = Vec::new();
  for item in items {
    match item {
      TypeItem::Field(f) => fields.push(f),
      TypeItem::Impl(i) => impls.push(i),
    }
  }
  (fields, impls)
}

/// Internal: build a [`Bound`] from a head term and any `| term` alternatives,
/// collapsing a single-element union to the term itself.
pub fn make_bound(first: Bound, mut rest: Vec<Bound>) -> Bound {
  if rest.is_empty() {
    return first;
  }
  let mut options = Vec::with_capacity(rest.len() + 1);
  options.push(first);
  options.append(&mut rest);
  let span = Span {
    start: options.first().unwrap().span().start,
    end: options.last().unwrap().span().end,
  };
  Bound::Union(UnionBound { span, options })
}

pub fn unescape(raw: &str) -> String {
  let inner = &raw[1..raw.len() - 1];
  let mut out = String::with_capacity(inner.len());
  let mut chars = inner.chars();
  while let Some(c) = chars.next() {
    if c == '\\' {
      match chars.next() {
        Some('"') => out.push('"'),
        Some('\\') => out.push('\\'),
        Some(other) => {
          out.push('\\');
          out.push(other);
        }
        None => out.push('\\'),
      }
    } else {
      out.push(c);
    }
  }
  out
}
