# Type checking & editor features

This document specifies how the checker assigns types to a MessageFormat 2
message against a registry (a set of `.mft` declarations), where each diagnostic
is surfaced, and how every editor feature (autocomplete, hover) is computed. It
assumes the language in `grammar.md` and the AST in `ast.d.ts`, and the concepts
in `introduction.md`.

Throughout, a _value_ is anything that resolves at runtime (a literal, a
variable, an expression). The checker gives each value a **type** drawn from the
registry (or a synthesized literal type), then asks **assignability** questions
against the **bounds** declared by functions, fields, and traits.

---

## 1. The type universe

Everything the checker reasons about is a **domain** — a set of values. A "type"
and a "bound" are not different kinds of thing; they are both domains,
distinguished only by _role_ (the value you have vs. the constraint you check it
against). The check-time representation is one recursive enum:

```rust
enum Domain {
  Head  { id: TypeId,  fields:  Map<FieldId,  Domain> }, // BoxNumber { select: "plural", value: AnyNumber }
  Trait { id: TraitId, members: Map<MemberId, Domain> }, // ToNumber { value: NumberLike } — "any impl, refined"
  Literal(String),                                       // "plural"
  Union(Vec<Domain>),
  Error,                                                 // recovery: a value whose type couldn't be determined
}
```

- A **`Trait` domain** means "any value implementing this trait," optionally
  **refined** by member domains (a struct body): `NumberLike` is
  `Trait { NumberLike, {} }`; `ToNumber { value: NumberLike }` refines the
  `value` member. `Head` and `Trait` are symmetric — both carry a refinement
  map; unlisted members/fields are unconstrained.
- A **`Head` domain** is a concrete value shape (a type) with its fields filled
  by domains.
- `Error` is engine-internal — assigned after a reported error and treated as
  compatible both ways so one mistake doesn't cascade. It is never written in a
  registry.

The registry vocabulary that produces these:

- **Types** are concrete value shapes (data **fields** + trait **impls**,
  possibly conditional). Types never extend types; relationships run through
  traits.
- **Traits** are capabilities with **associated members**. The core traits the
  checker depends on (for literal synthesis):
  - `PrimitiveString` — a bare primitive string value (literals, keys).
  - `ToString { value }` — can be rendered to a string (formatted); `value` is
    that string.
  - `NumberLike` (`needs PrimitiveString`) — a primitive string that is a
    MessageFormat number.
  - `ToNumber { value }` — can be used as a number; `value` is a `NumberLike`.
  - `Select { key }` — can drive a `.match`; `key` is the domain of legal
    variant keys (often itself a trait, e.g. `NumberLike` — see §1.0).
- **`Unknown` and `Unset` are ordinary types**, not special domains. `Unknown`
  (the type of an unannotated external input) implements `ToString` and
  `ToNumber`, so it can be formatted or numbered, but is _not_ `Select`-able and
  is _not_ accepted by a function whose operand trait it doesn't implement (so a
  bare external input isn't blanket-lenient — annotate it). `Unset` (the type of
  an omitted option/operand) implements nothing, so it is accepted only where a
  bound unions it in (`… | Unset`). The checker requires both to be declared
  (§1.1).
- **Boxes** (`BoxString`, `BoxNumber`, …) are the results of functions and the
  only values that implement `Select`.

### 1.0 Assignability — there is no "a trait isn't a value" rule

Because a domain plays "value" or "constraint" purely by _position_, a trait may
appear in any domain slot — including a member or matcher value, where it means
"any value implementing it." So `impl Select { key: NumberLike }` and the plural
key domains `NumberLike | "zero" | …` are exactly right: `key` is the _domain of
legal keys_, and "a legal key is anything `NumberLike`" is a perfectly good
domain. (Earlier drafts had a "value position rejects traits" rule; it was wrong
— the registry itself relies on traits as key domains.)

There is one operation, `assignable(value: &Domain, bound: &Domain) -> bool`
(subset-checking):

- **bound is a `Trait`** → `value`'s head implements the trait (resolve the
  impl, which yields the realized member domains), then each refinement is
  checked recursively (`assignable(realized_member, constraint)`), plus the
  trait's transitive `needs`.
- **bound is a `Head`** → same head (nominal), and each field's value-domain
  assignable to the field's bound-domain.
- literal / union / `Error` as expected (`Error` is compatible either way).

Operand/option checking is the _same_ recursion with capture: where a matcher
carries `@ $x`, it records `$x =` the realized member domain, for use in
`returns`. Pure subset checks (a variant key against a key domain) use
`assignable` with no capture.

The **only** kind constraints are therefore nominal: a trait-member bound and a
`needs`/`impl` target must be a **trait**, and a `returns` head must be a
**type** (you construct a concrete value, never "a trait"). Member, matcher,
field, operand, option, and key domains may be either — a trait there is just
the "any impl" domain.

### 1.1 Literal synthesis

The checker requires a registry that declares the **core traits and types** it
fabricates: `PrimitiveString`, `ToString` (with a `value` member), `NumberLike`,
`ToNumber` (with a `value` member), and the types `Unknown` and `Unset`. If any
is missing or malformed, registry resolution reports it (there is no built-in
default).

The checker assigns every literal token in a message a **synthesized singleton
type** named by the literal's string value, with auto-derived impls:

- always: `impl PrimitiveString` and `impl ToString { value: <self> }`.
- additionally, **iff** the literal matches the MessageFormat number grammar:
  `impl NumberLike` and `impl ToNumber { value: <self> }`.

The number grammar is:

```abnf
number-literal = ["-"] (%x30 / (%x31-39 *DIGIT)) ["." 1*DIGIT] [%i"e" ["-" / "+"] 1*DIGIT]
```

So `1`, `-3`, `2.50`, `1e3`, `6.02e-23` synthesize a `NumberLike`/`ToNumber`
type; `foo`, `01`, `+1`, `1.`, `.5`, `1` (trailing space) do **not** (they are
string-only). This single rule is what makes `{1 :number}` accepted and
`{foo :number}` rejected, and what makes `1` a legal key under `select=exact`
while `foo` is not.

---

## 2. Assignability

`assignable(V, B)` asks whether a value of type `V` is acceptable where bound
`B` is required. It is the one relation behind operand checking, option
checking, variant-key validity, `returns` construction, and `when` resolution.

Definitions, by the shape of `B`:

- **Union** `B = B₁ | … | Bₙ`: `assignable(V, B)` iff `V` is assignable to
  **some** `Bᵢ`.
- **`V` is itself a union** `V = V₁ | … | Vₘ`: assignable to `B` iff **every**
  `Vⱼ` is assignable to `B`. (Conservative: a value that _might_ be outside `B`
  is rejected.)
- **Literal bound** `B = "x"` (a literal type): iff `V` is the literal type
  `"x"`.
- **Type bound** `B = T { matchers }` (T a type name): **nominal** — `V`'s type
  is `T` — and every matcher holds (§2.1). Types do not subtype other types, so
  the head must match by name (`"1"` is _not_ assignable to the type
  `AnyNumber`; it relates to numbers through the `NumberLike` _trait_, not
  nominally).
- **Trait bound** `B = Tr { matchers }` (Tr a trait name): `V`'s type
  **implements** `Tr` (§2.2), and every matcher holds against the resolved
  impl's associated members.
- **`Unknown`** on the value side: assignable to a bound iff the bound is
  satisfiable by _some_ value — in practice `Unknown` is accepted by any trait
  bound it implements (`ToString`, `ToNumber`) and rejected by ones it doesn't
  (`PrimitiveString`, `Select`, nominal type bounds). It is never a false
  positive source for the traits it carries.

### 2.1 Matching `{ … }` (structural matchers)

For a bound `Head { m₁: B₁, m₂ @ $v, .. }`:

- Each named matcher selects a field (type bound) or associated member (trait
  bound) of the resolved head and requires it assignable to the matcher's bound
  (if a `: bound` is given).
- `@ $v` **captures** that member's type into binding `$v` for later use (e.g.
  in `returns`).
- `..` matches all remaining members with any bound.
- Per **rule B2**, every declared member must be covered by a matcher or `..`. A
  struct with no `..` requires the unlisted members to be absent; `{}` (or no
  braces) means "no members," which is only valid against a head that declares
  none.

### 2.2 Implementing a trait (with conditional impls)

`V`'s type implements `Tr` if:

1. it has an unconditional `impl Tr`, **or**
2. it has one or more `when B impl Tr` clauses, and — evaluating them **in
   source order** — the first whose condition holds supplies the impl. The
   condition holds iff `V` is assignable to the bound `B` (§2), which typically
   names `V`'s own type refined on a discriminant field
   (`BoxNumber { select: "exact", .. }`). Clauses need not be exhaustive or
   disjoint (rule I1); if none hold, `V` does **not** implement `Tr`.
3. additionally, `V` must satisfy each of `Tr`'s `needs` constraints (rule I3):
   for every `needs B { … }` declared on `Tr`, `V` must implement `B`, and `V`'s
   `B` members must satisfy the constraint's bounds — which may reference `Tr`'s
   associated members as realized on `V`. These requirements compose
   transitively (a cycle is an error).

The resolved impl's associated members are then available for matching (§2.1)
and for the selector key domain (§5). Note `needs` only _constrains_; it never
supplies an impl, so `V` must still carry its own `impl` of each required trait.

---

## 3. Declarations and the type environment

The checker walks declarations in source order, building a map from variable
name to type. This complements the existing scope analysis
(duplicate-declaration and use-before-declaration checks are unchanged).

- **`.input {$x ANNOTATION}`** — the expression's operand is the _external_
  value, typed `Unknown`. Apply the annotation (§4) with operand `Unknown`; bind
  `$x` to the result. With no annotation, `$x : Unknown`.
- **`.local $y = {EXPR}`** — evaluate `EXPR` (§4); bind `$y` to its type.
  `.local $y = {$z}` (bare variable) aliases `$z`'s type.

A variable used before/without a declaration is `Unknown` (subject to the
existing use-before-declaration diagnostic).

---

## 4. Expressions (operand + function)

An expression is `{ operand? annotation? }`. The annotation is a function
`:name`, markup (§7), or absent.

**No annotation.** The expression's type is the operand's type (a literal's
synthesized type or a variable's type). In a placeholder it must be formattable
(§6).

**Function `:name`.**

1. **Resolve the function.** Look up `fn name`. Not found → `UnknownFunction` on
   the name.
2. **Operand.** The operand's type is the value's type if one is present, or
   `Unset` if the expression has no operand (`{:name}`). Check
   `assignable(operandType, fn.operand)`, capturing any `@`-bindings. A failure
   is `BadOperand`, specialized to `MissingOperand` when the operand is absent
   (`Unset`) and the bound does not admit `Unset`, and to `UnexpectedOperand`
   when an operand is supplied but the bound is `Unset`-only. (Operand
   optionality is thus entirely a property of the bound — `ToString | Unset` is
   optional, `Unset` is no-operand — never a separate syntactic concept.)
3. **Options.** For each supplied `key=value`:
   - `key` not declared by `fn` → `UnknownOption` on the key.
   - duplicate `key` → `DuplicateOption` on the key.
   - resolve `value`'s type (literal synth or variable type) and check
     `assignable(value, option.bound)`, capturing `@`-bindings. Fail →
     `InvalidOptionValue` on the value.

   For each declared option **not** supplied:
   - it has a `= default` → use it (the default is pre-checked, rule F1).
   - else its value is `Unset`. If `option.bound` admits `Unset` → fine.
     Otherwise the option is **required**: `MissingRequiredOption` on the
     function call.

4. **Result.** Construct `fn.returns`, substituting captured bindings. The
   `returns` head is a type (resolution guarantees this); each field value is a
   domain (§1.0), checked assignable to the result type's declared field bound.
   The expression's type is this constructed (boxed) type.

> Reserved and private-use annotations (`{$x !…}`, `{$x ^…}`) are ignored by the
> type system (not reported as unknown functions). Expression **attributes**
> (`@attr`) are likewise ignored.

---

## 5. Selection (`.match`)

A matcher is `.match $s₁ … $sₖ` followed by variants, each
`key₁ … keyₖ {{pattern}}` (a key per selector, where a key is a literal or `*`).

1. **Each selector** `$sᵢ` must be an in-scope variable, and its type must
   implement `Select` (§2.2). If not → `SelectorNotSelectable` on the selector.
   (This is the type-level form of the spec rule that a selector must reference
   a declaration with a selecting function.) Resolve each selector's **key
   domain** = the `key` member of its resolved `Select` impl.
2. **Each variant**:
   - key count must equal the number of selectors, else
     `VariantKeyCountMismatch` on the variant.
   - for each non-`*` key `keyᵢ`: synthesize its literal type and check
     `assignable(keyᵢ, keyDomainᵢ)`. Fail → `InvalidVariantKey` on the key. `*`
     is always accepted.
   - if a key's literal type can never be produced by the selector — i.e. the
     key domain is a closed/narrowed set the literal is outside of even though
     it is the right _kind_ — this is the same `InvalidVariantKey`. When the key
     domain is narrowed to specific values (e.g. a `BoxString` from a literal
     `gold` has key domain `"gold"`), a key like `silver` is reported as a key
     that can never match.
3. **Exhaustiveness.** There must be a variant whose keys are all `*`; otherwise
   `NonExhaustiveMatch` on the matcher (a MessageFormat requirement).
4. **Duplicates.** Two variants with the same key tuple → `DuplicateVariant`.

**Key domains differ by options.** Because `Select` is implemented conditionally
on a box's discriminant, the legal keys follow the function's options. For
`BoxNumber`: `select=exact` → `key: NumberLike` (only number keys);
`select=plural`/`ordinal` →
`key: NumberLike | "zero" | "one" | "two" | "few" | "many" | "other"`. A literal
selector narrows: `{|gold| :string}` gives `BoxString { value: "gold" }`, whose
`Select` impl is `key: .value` = `"gold"`, so only `gold` (and `*`) match.

**Multiple selectors.** Validity and exhaustiveness are per-position; the
catch-all requirement is the all-`*` tuple. (Full cross-product _reachability_ —
which tuples can co-occur — and MessageFormat's specificity-based variant
ordering are out of scope for now; key-validity, the all-`*` check, and
duplicate detection are in.)

---

## 6. Formatting

A value placed in a **placeholder** (a `{…}` in pattern text) is formatted, so
its type must implement `ToString`; otherwise `NotFormattable` on the
placeholder. `Unset` and any value lacking `ToString` are not formattable.
(Every box and `Unknown` implement `ToString`, so this mainly catches misuse of
values that have no string form.)

---

## 7. Markup and other constructs

- **Markup** (`{#tag …}`, `{/tag}`, `{#tag/}`) is not a function and does not
  participate in the value/type flow. Its open/close balance is a structural
  concern (handled outside this system); its options are not validated against
  the function registry. A parallel markup registry could be added later; until
  then markup options are accepted as-is.
- **Text and escapes** carry no type.
- **`.input` / `.local`** are covered in §3; **selectors/variants** in §5.

---

## 8. Diagnostic catalog

| Diagnostic                | Meaning                                                      | Surfaced on       |
| ------------------------- | ------------------------------------------------------------ | ----------------- |
| `UnknownFunction`         | `:name` has no `fn` in the registry                          | the function name |
| `MissingOperand`          | function requires an operand; none given                     | the expression    |
| `UnexpectedOperand`       | function takes no operand; one given                         | the operand       |
| `BadOperand`              | operand type not assignable to the operand bound             | the operand       |
| `UnknownOption`           | option not declared by the function                          | the option key    |
| `DuplicateOption`         | same option given twice                                      | the option key    |
| `InvalidOptionValue`      | option value not assignable to its bound                     | the option value  |
| `MissingRequiredOption`   | required option (no default, bound excludes `Unset`) omitted | the function call |
| `SelectorNotSelectable`   | selector's type does not implement `Select`                  | the selector      |
| `InvalidVariantKey`       | key not assignable to the selector's key domain              | the key           |
| `VariantKeyCountMismatch` | variant key count ≠ selector count                           | the variant       |
| `NonExhaustiveMatch`      | no all-`*` variant                                           | the matcher       |
| `DuplicateVariant`        | two variants with identical keys                             | the later variant |
| `NotFormattable`          | placeholder value lacks `ToString`                           | the placeholder   |

Planned (needs option annotations, deferred): a **dead-option** lint for options
that cannot affect the way a value is used (e.g. a format-only option on a value
only ever selected on).

---

## 9. Autocomplete

Each editor position maps to a query over the registry and the current type
environment.

### 9.1 Function name — after `:` in an annotation

Offer all `fn` names. If the expression has an operand, **rank/filter** by
`assignable(operandType, fn.operand)` so applicable functions come first (e.g.
`:number` for a number-like operand). For a no-operand expression (`{:`), prefer
functions that allow no operand.

### 9.2 Option name — at a name slot inside a function call

Offer the function's option names **not already present**. Detail each with its
bound (rendered as its allowed values) and doc. Mark required options.

### 9.3 Option value — after `opt=`

Derive suggestions from the option's bound:

- enumerate the bound's **literal arms** (e.g. `signDisplay` → `auto`, `always`,
  `exceptZero`, `negative`, `never`); these are the primary completions.
- if the bound includes a trait/type that admits variables (e.g. `ToNumber`),
  also offer in-scope **variables whose type satisfies the bound**, and (for
  `ToNumber`) note that a number literal is accepted.
- if the bound admits `Unset`, the option is optional (don't force a value).

### 9.4 Variant key — at a key slot in `.match`

Resolve the selector's key domain (§5) and offer:

- the **literal arms** of the domain — e.g. plural categories
  `zero one two few many other`, or the specific values of a narrowed string
  selector (`gold`).
- always `*`.
- skip keys already used at that position.
- open arms (`NumberLike`, `AnyString`) are not enumerable; surface them as a
  hint ("any number" / "any string" is also valid) rather than concrete items. A
  convenience action can scaffold a full set of variants from the enumerable
  arms plus `*`.

### 9.5 Variable — after `$`, and operand position

Offer in-scope variables (existing behavior), now annotated with their
**types**. In an operand position whose function expects a particular trait
(e.g. `:number` → `ToNumber`), rank variables whose type satisfies it.

### 9.6 Hover

- **Variable** → its type (and, for a box, its discriminant, e.g. `select`),
  plus where it was declared.
- **Function name** → its signature: operand bound, options (with
  bounds/defaults), and result type.
- **Option** → its bound (allowed values) and doc.
- **Variant key** → which selector key domain it belongs to and whether it is a
  narrowed or open match.
