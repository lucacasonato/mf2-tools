# The MessageFormat 2 type system — introduction

MessageFormat 2 messages call **functions** to format and select on values:

```
.input {$count :number}
.match $count
  one {{You have {$count :number} new message.}}
  *   {{You have {$count :number} new messages.}}
```

Each function (`:number`, `:string`, `:datetime`, …) has rules: which options it
accepts, what values those options take, whether it can format, whether it can
be used to _select_ (in `.match`), and — if it can — which variant keys are
legal. Today an editor has no idea about any of that. It cannot tell you that
`:numbr` is a typo, that `minimumFractionDigit` is misspelled, that
`signDisplay=upward` is not a real value, that you put `:datetime` in a `.match`
where it can't select, or that `seven` is not a plural keyword.

This type system makes those rules **machine-readable**, and a checker flows
types through a message so the editor can answer them. It has two halves:

- A small **declaration language** (`.mft`, see `grammar.md`) in which functions
  and the value types they produce are described. A default registry ships built
  in; a project can add its own functions.
- A **checker** that, given a parsed message and a registry, assigns a type to
  every value as it flows out of declarations, through functions, and into
  placeholders and selectors — and reports where the rules are violated.

## What it lets the LSP do

### Diagnostics (errors and warnings)

- **Unknown function** — `{$x :numbr}` → no such function.
- **Bad operand** — `{$date :number}` where `$date` is a date → the operand
  can't be used as a number.
- **Unknown option** — `:number minimumFractionDigit=2` → no such option.
- **Invalid option value** — `:number signDisplay=upward` → not one of the
  allowed values; also `:number minimumFractionDigits=$x` where `$x` is the
  wrong kind of value.
- **Missing required option** — e.g. `:currency` without `currency=…`.
- **Not selectable** — `.match $d` where `$d` came from a format-only function
  like `:datetime` → that value can't drive a selection.
- **Invalid / unreachable variant key** — under `:number select=plural`, `seven`
  is not a legal key; and a key that can never match the selector (e.g.
  `|silver|` when the value is known to be `gold`) is flagged.
- **Non-exhaustive `.match`** — missing the required catch-all (`*`) variant.
- **Not formattable** — a value placed in a placeholder that has no string form.
- **Dead option** _(planned)_ — an option that does nothing in the context the
  value is used (e.g. `compactDisplay` on a value that is only ever selected
  on).

### Autocomplete

- **Function names** after `:` — optionally filtered to functions whose operand
  the current value satisfies.
- **Option names** inside a function call — those not already given.
- **Option values** — the allowed members of an enum option (`signDisplay` →
  `auto`, `always`, …).
- **Variant keys** in `.match` — the legal keys for the selector (e.g. the
  plural categories `zero one two few many other`, plus `*`).
- **Variables** after `$` — in-scope declarations (already supported; the type
  system adds type information to the suggestions).

### Hover

- The **type** of a variable (what a declaration produced).
- A **function's signature** — its operand, options, and result.
- An **option's** documentation and allowed values.

## The idea in one page

Values flowing through a message have **types**. A function is a typed
transform: it constrains the operand it accepts, the options it takes, and it
produces a result type.

Rather than a fixed lattice, capabilities are described with **traits** — small
interfaces a type can implement:

- `ToString` / `ToNumber` — "this value can be used as a string / number." This
  is how a function says what operands it accepts: `:number` takes anything that
  is `ToNumber`.
- `Select` — "this value can drive a `.match`," and its associated `key` says
  which variant keys are legal.

Literal text in a message is given a **synthesized type** automatically: every
literal is a string (`PrimitiveString` + `ToString`), and additionally
number-like (`NumberLike` + `ToNumber`) when it matches the MessageFormat number
grammar — so `1` can be numbered but `foo` cannot.

Functions return **boxed** values (`BoxString`, `BoxNumber`, …). The box is what
carries selection capability: a bare literal can be formatted, but only a value
that has been through a selecting function is `Select`-able — which is exactly
the MessageFormat rule that a `.match` selector must reference a declaration
with a function. A box can record a discriminant (e.g. a number's `select` mode)
and implement `Select` **conditionally** on it, so `:number select=exact` and
`:number select=plural` offer different legal keys.

The whole vocabulary — traits, types, functions — is data, authored in `.mft`.
For the precise checking rules and how each editor feature is computed, see
`type-checking.md`; for the language, `grammar.md` and `ast.d.ts`.
