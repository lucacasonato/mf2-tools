//! AST for the Message Format Types (`.mft`) declaration language.
//!
//! This mirrors `checker/docs/ast.d.ts`, with byte-offset [`Span`]s attached to
//! every node. It is produced by the `lalrpop` grammar in `src/grammar.lalrpop`.

/// A byte-offset range into the `.mft` source text. Offsets are `u16`: registry
/// files are small, and [`parse`](crate::parse) rejects sources longer than
/// `u16::MAX`, so the casts in [`Span::new`] never truncate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
  pub start: u16,
  pub end: u16,
}

impl Span {
  pub fn new(start: usize, end: usize) -> Span {
    Span {
      start: start as u16,
      end: end as u16,
    }
  }
}

/// An identifier (a type/trait/field/option name), with its span.
#[derive(Debug, Clone)]
pub struct Ident {
  pub span: Span,
  pub name: String,
}

/// A parsed `.mft` document: an ordered list of top-level declarations.
#[derive(Debug, Clone)]
pub struct Document {
  pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub enum Declaration {
  Trait(TraitDecl),
  Type(TypeDecl),
  Fn(FnDecl),
}

// ===========================================================================
// Traits
// ===========================================================================

#[derive(Debug, Clone)]
pub struct TraitDecl {
  pub span: Span,
  pub name: Ident,
  /// Associated members; each member `bound` must denote a trait (rule T1).
  pub members: Vec<MemberDecl>,
  /// `needs` constraints (rule I3) — requirements on implementers, never provisions.
  pub needs: Vec<Needs>,
}

#[derive(Debug, Clone)]
pub struct MemberDecl {
  pub span: Span,
  pub name: Ident,
  pub bound: Bound,
}

/// A trait's `needs` constraint. Structurally like [`Impl`], but its members are
/// bounds the implementer must satisfy, not values provided here (rule I3).
#[derive(Debug, Clone)]
pub struct Needs {
  pub span: Span,
  pub when: Option<WhenClause>,
  pub trait_: Ident,
  pub members: Vec<MemberBinding>,
}

// ===========================================================================
// Types
// ===========================================================================

#[derive(Debug, Clone)]
pub struct TypeDecl {
  pub span: Span,
  pub name: Ident,
  pub fields: Vec<FieldDecl>,
  pub impls: Vec<Impl>,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
  pub span: Span,
  pub name: Ident,
  pub bound: Bound,
}

/// A type's trait implementation (a provision), optionally guarded by `when`.
#[derive(Debug, Clone)]
pub struct Impl {
  pub span: Span,
  pub when: Option<WhenClause>,
  pub trait_: Ident,
  pub members: Vec<MemberBinding>,
}

#[derive(Debug, Clone)]
pub struct WhenClause {
  pub span: Span,
  /// The bound the surrounding value must satisfy for the clause to apply,
  /// e.g. `BoxNumber { select: "exact", .. }`.
  pub bound: Bound,
}

#[derive(Debug, Clone)]
pub struct MemberBinding {
  pub span: Span,
  pub name: Ident,
  pub value: Bound,
}

// ===========================================================================
// Functions
// ===========================================================================

#[derive(Debug, Clone)]
pub struct FnDecl {
  pub span: Span,
  pub name: Ident,
  /// The operand bound. Operand-optionality lives in the *type*, not syntax: a
  /// function callable with no operand (`{:fn}`) admits `Unset` in this bound
  /// (`ToString | Unset`); one that takes no operand at all has bound `Unset`.
  pub operand: Bound,
  pub options: Vec<OptionDecl>,
  pub returns: Bound,
}

#[derive(Debug, Clone)]
pub struct OptionDecl {
  pub span: Span,
  pub name: Ident,
  /// `@ $var` capture, making the option's value available to `returns`.
  pub binding: Option<Ident>,
  pub bound: Bound,
  /// `= literal` default applied when the option is omitted.
  pub default: Option<Bound>,
}

// ===========================================================================
// Bounds / type expressions
// ===========================================================================

#[derive(Debug, Clone)]
pub enum Bound {
  Ref(RefBound),
  Literal(LiteralBound),
  Union(UnionBound),
  Structural(StructuralBound),
  FieldRef(FieldRefBound),
  VarRef(VarRefBound),
}

impl Bound {
  pub fn span(&self) -> Span {
    match self {
      Bound::Ref(b) => b.span,
      Bound::Literal(b) => b.span,
      Bound::Union(b) => b.span,
      Bound::Structural(b) => b.span,
      Bound::FieldRef(b) => b.span,
      Bound::VarRef(b) => b.span,
    }
  }
}

/// A reference to a named type or trait (`AnyString`, `ToNumber`).
#[derive(Debug, Clone)]
pub struct RefBound {
  pub span: Span,
  pub name: String,
}

/// A literal type (`"foo"`).
#[derive(Debug, Clone)]
pub struct LiteralBound {
  pub span: Span,
  pub value: String,
}

/// `A | B | C`.
#[derive(Debug, Clone)]
pub struct UnionBound {
  pub span: Span,
  pub options: Vec<Bound>,
}

/// `Base { matchers, .. }`, where `base` is a named type or trait.
#[derive(Debug, Clone)]
pub struct StructuralBound {
  pub span: Span,
  pub base: Ident,
  pub fields: Vec<FieldMatcher>,
  /// Whether `..` (match all remaining fields) is present.
  pub rest: bool,
}

/// `.field` — references a field of the surrounding type / member of the trait.
#[derive(Debug, Clone)]
pub struct FieldRefBound {
  pub span: Span,
  pub field: String,
}

/// `$name` — references a variable bound earlier in the same `fn` signature.
#[derive(Debug, Clone)]
pub struct VarRefBound {
  pub span: Span,
  pub name: String,
}

#[derive(Debug, Clone)]
pub struct FieldMatcher {
  pub span: Span,
  pub name: Ident,
  /// `@ $var` capture, if present.
  pub binding: Option<Ident>,
  /// `: bound` constraint, if present.
  pub bound: Option<Bound>,
}
