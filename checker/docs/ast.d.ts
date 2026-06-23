/**
 * JSON data model (AST) for the Message Format Types (`.mft`) declaration language.
 *
 * This is the parsed, serializable form of a `.mft` file — the shape a registry takes
 * once loaded. The default registry and any project-supplied registries deserialize into
 * a `Document`. The surface grammar is in `grammar.md`.
 *
 * Several positions reuse the same `Bound` type expression (field bounds, operand/option
 * bounds, `when` conditions, impl member values, `returns` constructions). Context restricts
 * which `Bound` variants are meaningful in each position; the restrictions are noted at the
 * relevant fields and correspond to the "Static rules" in `grammar.md`.
 */

/** A parsed `.mft` document: an ordered list of top-level declarations. */
export interface Document {
  declarations: Declaration[];
}

export type Declaration = TraitDecl | TypeDecl | FnDecl;

// ===========================================================================
// Traits
// ===========================================================================

/** A capability that types may implement (`trait ToString { value: PrimitiveString }`). */
export interface TraitDecl {
  kind: "trait";
  name: string;
  /**
   * Associated members an implementer must provide. Per static rule T1, every member's
   * `bound` denotes a *trait* (never a concrete type).
   */
  members: MemberDecl[];
  /**
   * `needs` constraints (`needs PrimitiveString;`, `needs ToString { value: .value };`). Per
   * rule I3 these are *requirements*, not provisions: every implementer of this trait must
   * also implement each constraint's trait, and — if refined with a struct — that
   * implementer's members must satisfy the given bounds, which may reference this trait's
   * associated members via a `FieldRef` (`.value`). A `Needs` mirrors `Impl` structurally,
   * but each `MemberBinding.value` is a *bound to check*, not a value to provide.
   */
  needs: Needs[];
  /** Leading `#` comment(s), if attached. */
  doc?: string;
}

export interface MemberDecl {
  name: string;
  bound: Bound;
  doc?: string;
}

// ===========================================================================
// Types
// ===========================================================================

/** A concrete value shape: data fields plus trait implementations. */
export interface TypeDecl {
  kind: "type";
  /**
   * The type's name — always an identifier. Per rule L1, literal types (the singleton types
   * of literal tokens like `"1"`) are synthesized by the checker from message text and are
   * never declared, so a `TypeDecl` name is never a literal.
   */
  name: string;
  /** Data fields carried by values of this type. Bounds may be types or traits. */
  fields: FieldDecl[];
  /** Trait implementations (provisions), possibly conditional (`when …`). */
  impls: Impl[];
  doc?: string;
}

export interface FieldDecl {
  name: string;
  bound: Bound;
  doc?: string;
}

/**
 * A type's trait implementation — a *provision*: the member values realize the trait.
 * Optionally guarded by a `when` clause. See `Needs` for the trait-side *constraint* form.
 */
export interface Impl {
  /** Name of the trait being implemented. */
  trait: string;
  /**
   * Associated-member values. Empty for marker-trait impls written without a body
   * (`impl PrimitiveString;`). Per static rule I2, each value must be assignable to the
   * trait's declared bound for that member.
   */
  members: MemberBinding[];
  /**
   * Present for conditional impls (`when <bound> impl …`). Per rule I1, when
   * several impls of the same trait are present they are evaluated in source order and the
   * first whose condition holds wins; they need not be exhaustive or disjoint.
   */
  when?: WhenClause;
  doc?: string;
}

/**
 * A trait's `needs` constraint (rule I3): a *requirement* on implementers, not a provision.
 * Structurally identical to `Impl`, but each `MemberBinding.value` is a bound the
 * implementer's corresponding member must satisfy (and may reference the trait's own
 * associated members via `FieldRef`), rather than a value supplied here.
 */
export interface Needs {
  /** Name of the required trait. */
  trait: string;
  /** Member bounds the implementer's `trait` impl must satisfy. Empty for marker traits. */
  members: MemberBinding[];
  /** Present for conditional constraints (`when <bound> needs …`). */
  when?: WhenClause;
  doc?: string;
}

export interface WhenClause {
  /**
   * The bound the surrounding value must satisfy for the clause to apply, typically a
   * `Structural` bound naming the surrounding type/trait and refining a discriminant field
   * (`BoxNumber { select: "exact", .. }`). The condition holds when the value is assignable
   * to this bound.
   */
  bound: Bound;
}

export interface MemberBinding {
  /** The trait associated member being filled (in an `Impl`) or constrained (in a `Needs`). */
  name: string;
  /**
   * In an `Impl`, the value provided for the member; in a `Needs`, the bound the
   * implementer's member must satisfy. May reference the surrounding type's field or the
   * surrounding trait's associated member via a `FieldRef` (`.value`), or be any other
   * bound/type expression.
   */
  value: Bound;
}

// ===========================================================================
// Functions
// ===========================================================================

/** A function invoked by a `:name` annotation in a message. */
export interface FnDecl {
  kind: "fn";
  name: string;
  /**
   * The operand bound (always present). Operand-optionality is expressed in the *type*,
   * not syntax: a fn callable with no operand (`{:fn}`) admits `Unset` (`ToString | Unset`),
   * and one that takes no operand at all has bound `Unset`. Field matchers inside may
   * introduce bindings (`operand ToNumber { value @ $num }`) used in `returns`.
   */
  operand: Bound;
  options: OptionDecl[];
  /**
   * The result type, expressed as a `StructuralBound` that names a type and fills its
   * fields, typically from bindings (`returns BoxNumber { value: $num, select: $select }`).
   */
  returns: Bound;
  doc?: string;
}

export interface OptionDecl {
  name: string;
  /** `@ $var` capture, making the option's resolved value available to `returns`. */
  binding?: string;
  bound: Bound;
  /** `= literal` default applied when the option is omitted. Per F1, must satisfy `bound`. */
  default?: Bound;
  doc?: string;
}

// ===========================================================================
// Bounds / type expressions
// ===========================================================================

/**
 * A bound / type expression. Used for field bounds, operand/option bounds, `when`
 * conditions, impl member values, and `returns` constructions.
 *
 * - `RefBound` / `LiteralBound` / `UnionBound` / `StructuralBound` are valid everywhere.
 * - `FieldRef` is valid only inside the member values of a type's `impl` or a trait's `needs`
 *   (rule B1).
 * - `VarRef` is valid only within a `fn` signature, after the binding is introduced (B1).
 */
export type Bound =
  | RefBound
  | LiteralBound
  | UnionBound
  | StructuralBound
  | FieldRef
  | VarRef;

/** A reference to a named type or trait (`AnyString`, `ToNumber`). */
export interface RefBound {
  kind: "ref";
  name: string;
}

/** A literal type (`"foo"`). */
export interface LiteralBound {
  kind: "literal";
  value: string;
}

/** `A | B | C`. */
export interface UnionBound {
  kind: "union";
  options: Bound[];
}

/** `Base { matchers, .. }`, where `base` is a named type or trait (never a literal). */
export interface StructuralBound {
  kind: "structural";
  base: RefBound;
  fields: FieldMatcher[];
  /**
   * Whether `..` is present (match all remaining fields with any bound). Per rule B2, every
   * declared field of `base` must be covered by an explicit matcher or by `rest`; otherwise
   * unlisted fields are required to be absent.
   */
  rest: boolean;
}

export interface FieldMatcher {
  name: string;
  /**
   * `@ $var` capture, if present. Per rule B4, all branches of a `union` must capture the
   * same set of bindings, so a downstream reference is defined whichever branch matches.
   */
  binding?: string;
  /**
   * `: bound` constraint, if present. In a `returns` construction this carries the value
   * assigned to the field (commonly a `VarRef`), rather than a constraint to check.
   */
  bound?: Bound;
}

/**
 * `.field` — references a field of the surrounding type (in an `Impl`) or an
 * associated member of the surrounding trait (in a `Needs`).
 */
export interface FieldRef {
  kind: "field-ref";
  field: string;
}

/** `$name` — references a variable bound earlier in the same `fn` signature. */
export interface VarRef {
  kind: "var-ref";
  name: string;
}
