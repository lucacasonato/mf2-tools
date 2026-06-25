# Message Format Types (`.mft`) — grammar

`.mft` is the declaration language for the MessageFormat 2 type system. A `.mft`
file declares **traits** (capabilities), **types** (the values that flow through
a message), and **functions** (the typed transforms invoked by `:func`
annotations). The default registry ships as a `.mft` file; projects may supply
additional ones.

The grammar below is [ABNF](https://www.rfc-editor.org/rfc/rfc5234) (RFC 5234
core rules `ALPHA`, `DIGIT`, `SP`, `HTAB`, `CR`, `LF`, `DQUOTE` are assumed).
Some constraints are not expressible in a context-free grammar and are listed
under **Static rules** below.

```abnf
; ===== Document =====
document      = *( osp ( comment / declaration ) ) osp
declaration   = trait-decl / type-decl / fn-decl

; ===== Comments & whitespace =====
comment       = "#" *( %x09 / %x20-10FFFF ) newline   ; `#` to end of line
newline       = CR LF / LF
osp           = *( SP / HTAB / newline )              ; optional spacing
rws           = 1*( SP / HTAB / newline )             ; required spacing

; ===== Traits =====
; A trait is a capability; its members are "associated members" an impl must fill.
; `needs` lines state CONSTRAINTS on implementers (see I3) — they never provide members.
trait-decl    = "trait" rws name osp "{" *( osp trait-item ) osp "}"
trait-item    = member-decl / needs-decl
member-decl   = name osp ":" osp bound osp ";"        ; associated member; see T1
needs-decl    = [ when-clause rws ] "needs" rws name [ osp impl-body ] osp ";"

; ===== Types =====
; A type is a concrete value shape: data fields + trait implementations.
type-decl     = "type" rws name osp "{" *( osp type-item ) osp "}"
type-item     = field-decl / impl-decl
field-decl    = name osp ":" osp bound osp ";"        ; a data field
impl-decl     = [ when-clause rws ] "impl" rws name [ osp impl-body ] osp ";"
when-clause   = "when" rws bound
impl-body     = "{" osp member-binding *( osp "," osp member-binding ) osp "}"
member-binding= name osp ":" osp bound       ; comma-separated; NO trailing ";"

; ===== Functions =====
fn-decl       = "fn" rws name osp "{"
                  osp "operand" rws bound osp ";"   ; optionality lives in the bound (`… | Unset`)
                  [ osp "options" osp "{" *( osp option-decl ) osp "}" ]
                  osp "returns" rws bound osp ";"
                osp "}"
option-decl   = name [ osp "@" osp var ] osp ":" osp bound
                [ osp "=" osp literal ] osp ";"

; ===== Bounds / type expressions =====
; One surface form is reused for: field bounds, operand/option bounds, `when`
; conditions, impl member values, and `returns` constructions. See Static rule B1.
bound         = bound-term *( osp "|" osp bound-term ) ; union
bound-term    = ref [ osp struct ] / string / field-ref / var  ; only a `ref` takes a struct
ref           = name                                  ; a type or trait name
field-ref     = "." name                              ; `.value` — a field of the surrounding type
var           = "$" name                              ; `$num` — a binding
struct        = "{" [ osp matcher-list ] osp "}"
matcher-list  = ".." / ( matcher *( osp "," osp matcher ) [ osp "," osp ".." ] )
matcher       = name [ osp "@" osp var ] [ osp ":" osp bound ]
literal       = string

; ===== Lexical =====
name          = ( ALPHA / "_" ) *( ALPHA / DIGIT / "_" )
string        = DQUOTE *char DQUOTE
char          = %x20-21 / %x23-5B / %x5D-10FFFF / escape   ; any but `"` and `\`
escape        = "\" ( DQUOTE / "\" )
```

## Static rules (not expressible in ABNF)

- **T1 — trait positions are traits.** Inside a `trait` declaration, every
  **member bound** and every trait named by a **`needs` constraint** (with any
  `struct` refining it) must denote a _trait_ (`needs PrimitiveString;`,
  `needs ToString { value: PrimitiveString };`). Inside `type` field
  declarations and anywhere in a `fn` signature, a bound may be a _type or a
  trait_, freely mixed and nested (a type bound may appear in a field of a trait
  bound, and vice versa).
- **L1 — literal types are synthesized, not declared.** A type name is always an
  identifier. The singleton types of literal tokens (`"foo"`, `"1"`) are created
  by the checker from the message text (see type-checking.md §1.1); they cannot
  be written as `type "foo" { … }` and never appear in a `.mft` file. Literals
  still appear freely in _bound_ positions (`value:
"1"`, `key: "zero"`).
- **B1 — context of `field-ref` and `var`.** `.field` (`field-ref`) is valid
  inside the member values of a `type`'s `impl` (referring to that type's
  fields) _or_ a `trait`'s `needs` (referring to that trait's associated
  members). A `when` condition instead names the surrounding type/trait and
  refines it with a `struct` (`when BoxNumber { select: "exact",
.. }`). `$name`
  (`var`) is only valid in a `fn` signature: it is _introduced_ by an
  `@`-binding in the operand or an option, and _referenced_ later (typically in
  `returns`).
- **B2 — `struct` field coverage.** Within a `struct`, every field declared by
  the matched type/trait must be covered, either by an explicit `matcher` or by
  `..` (which matches all remaining fields with any bound). A `struct` with no
  `..` requires the unlisted fields to be _absent_. Therefore `{}` (and a bare
  `bound-term` with no `struct`) means "no fields"; it is an error to write `{}`
  against a type/trait that has fields — use `{ .. }`.
- **B3 — `..` placement.** `..` may appear only as the **final** entry of a
  `matcher-list` (alone, as `{ .. }`, or after other matchers, as
  `BoxNumber { select: "exact", .. }`); at most once.
- **B4 — unions capture uniformly.** Within a bound, a `matcher`'s `@ $var`
  _captures_ a binding. At every `union`, all branches must introduce the **same
  set** of captured bindings, so that whichever branch matches, every binding
  referenced downstream (typically in `returns`) is defined. Thus
  `ToNumber { value @ $num } | ToString { value @ $num }` is legal, but
  `ToNumber { value @ $num } | ToString { .. }` is not (the `$num` binding would
  be undefined when the second branch matches). A binding name may not be
  captured twice on the same side of a union either.
- **I1 — conditional clauses are ordered.** Multiple `when … impl T` (in a type)
  or `when … needs T` (in a trait) clauses for the same trait `T` are evaluated
  in source order; the first whose condition holds determines the
  impl/constraint. They need **not** be exhaustive (an uncovered discriminant
  value simply does not implement `T`) and need **not** be disjoint (first match
  wins). An unconditional clause always holds.
- **I2 — associated members are covariant.** An impl's member value must be
  assignable to the trait's declared bound for that member (e.g.
  `impl Select { key: NumberLike }` is legal because `NumberLike` is a subtype
  of the declared `key: PrimitiveString`).
- **I3 — `needs` is a constraint, `impl` is a provision.** An `impl B [{ … }];`
  in a _type_ **provides** `B` (its member values realize `B`). A
  `needs B [{ … }];` in a _trait_ is a **constraint**: every implementer of the
  trait must independently implement `B`, and (if a `struct` is given) that
  implementer's `B` members must satisfy the bounds — which may reference the
  trait's own associated members via `.`. A `needs` never supplies members.
  Requirements compose transitively; a requirement cycle is an error.
- **F1 — defaults satisfy bounds.** An option's `= literal` default must satisfy
  the option's `bound`.

## Worked snippet

```mft
trait PrimitiveString {}
trait ToString { value: PrimitiveString; }
trait NumberLike { needs PrimitiveString; }
trait Select { key: PrimitiveString; }

type AnyNumber {
  impl PrimitiveString;
  impl NumberLike;
  impl ToString { value: AnyNumber };
}

type BoxNumber {
  value: NumberLike;
  select: "exact" | "plural" | "ordinal";

  impl ToString { value: .value };
  when BoxNumber { select: "exact", .. }  impl Select { key: NumberLike };
  when BoxNumber { select: "plural", .. } impl Select { key: NumberLike | "zero" | "one" | "two" | "few" | "many" | "other" };
}

fn number {
  operand ToNumber { value @ $num };
  options {
    select @ $select: "exact" | "plural" | "ordinal" = "plural";
    minimumFractionDigits: ToNumber { .. } | Unset;
  }
  returns BoxNumber { value: $num, select: $select };
}
```
