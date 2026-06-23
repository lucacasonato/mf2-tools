//! Golden-file spec tests for the `.mft` parser.
//!
//! Each fixture under `tests/fixtures/` holds the source, then either a
//! `=== ast ===` section with a compact pretty-print of the parsed [`Document`],
//! or an `=== error ===` section with the parse error. Run with `UPDATE=1` to
//! (re)generate the expected sections; a fixture with no section yet is filled
//! in on first run.

use std::fmt::Write as _;
use std::panic::AssertUnwindSafe;
use std::path::Path;

use file_test_runner::RunOptions;
use file_test_runner::TestResult;
use file_test_runner::collect_and_run_tests;
use file_test_runner::collection::CollectOptions;
use file_test_runner::collection::CollectedTest;
use file_test_runner::collection::strategies::TestPerFileCollectionStrategy;

use mf2_checker::ast;
use mf2_checker::parse;

const AST_MARKER: &str = "\n=== ast ===\n";
const ERROR_MARKER: &str = "\n=== error ===\n";

fn main() {
  collect_and_run_tests(
    CollectOptions {
      base: Path::new("tests/parsing").into(),
      strategy: Box::new(TestPerFileCollectionStrategy { file_pattern: None }),
      filter_override: None,
    },
    RunOptions { parallel: true },
    |test| TestResult::from_maybe_panic(AssertUnwindSafe(|| run_test(test))),
  )
}

fn run_test(test: &CollectedTest) {
  let file_text = test.read_to_string().unwrap();

  let (source, expected) = file_text
    .split_once(AST_MARKER)
    .or_else(|| file_text.split_once(ERROR_MARKER))
    .unwrap_or((file_text.as_str(), ""));

  let (marker, actual) = match parse(source) {
    Ok(doc) => (AST_MARKER, pretty(&doc)),
    Err(err) => (ERROR_MARKER, err),
  };

  let need_update = std::env::var("UPDATE").is_ok() || expected.is_empty();
  if need_update {
    std::fs::write(&test.path, format!("{source}{marker}{actual}")).unwrap();
  } else {
    pretty_assertions::assert_eq!(actual, expected, "parse output matches");
  }
}

// ===========================================================================
// Compact AST pretty-printer
//
// Renders the AST source-like, one declaration member per indented line, with a
// `@start..end` span on each structural node. Bounds are rendered inline.
// ===========================================================================

fn pretty(doc: &ast::Document) -> String {
  let mut out = String::new();
  for (i, decl) in doc.declarations.iter().enumerate() {
    if i > 0 {
      out.push('\n');
    }
    match decl {
      ast::Declaration::Trait(t) => pretty_trait(t, &mut out),
      ast::Declaration::Type(t) => pretty_type(t, &mut out),
      ast::Declaration::Fn(f) => pretty_fn(f, &mut out),
    }
  }
  out.truncate(out.trim_end().len());
  out
}

fn span(s: ast::Span) -> String {
  format!("@{}..{}", s.start, s.end)
}

fn pretty_trait(t: &ast::TraitDecl, out: &mut String) {
  let _ = writeln!(out, "trait {} {}", t.name.name, span(t.span));
  for m in &t.members {
    let _ = writeln!(
      out,
      "  {}: {} {}",
      m.name.name,
      bound(&m.bound),
      span(m.span)
    );
  }
  for n in &t.needs {
    let _ = writeln!(
      out,
      "  {}needs {}{} {}",
      when_prefix(&n.when),
      n.trait_.name,
      body(&n.members),
      span(n.span)
    );
  }
}

fn pretty_type(t: &ast::TypeDecl, out: &mut String) {
  let _ = writeln!(out, "type {} {}", t.name.name, span(t.span));
  for f in &t.fields {
    let _ = writeln!(
      out,
      "  {}: {} {}",
      f.name.name,
      bound(&f.bound),
      span(f.span)
    );
  }
  for i in &t.impls {
    let _ = writeln!(
      out,
      "  {}impl {}{} {}",
      when_prefix(&i.when),
      i.trait_.name,
      body(&i.members),
      span(i.span)
    );
  }
}

fn pretty_fn(f: &ast::FnDecl, out: &mut String) {
  let _ = writeln!(out, "fn {} {}", f.name.name, span(f.span));
  let _ = writeln!(
    out,
    "  operand {} {}",
    bound(&f.operand),
    span(f.operand.span())
  );
  for o in &f.options {
    let mut line = format!("  option {}", o.name.name);
    if let Some(b) = &o.binding {
      let _ = write!(line, " @ ${}", b.name);
    }
    let _ = write!(line, ": {}", bound(&o.bound));
    if let Some(d) = &o.default {
      let _ = write!(line, " = {}", bound(d));
    }
    let _ = writeln!(out, "{line} {}", span(o.span));
  }
  let _ = writeln!(
    out,
    "  returns {} {}",
    bound(&f.returns),
    span(f.returns.span())
  );
}

fn when_prefix(when: &Option<ast::WhenClause>) -> String {
  match when {
    Some(w) => format!("when {} ", bound(&w.bound)),
    None => String::new(),
  }
}

fn body(members: &[ast::MemberBinding]) -> String {
  if members.is_empty() {
    return String::new();
  }
  let parts: Vec<String> = members
    .iter()
    .map(|m| format!("{}: {}", m.name.name, bound(&m.value)))
    .collect();
  format!(" {{ {} }}", parts.join(", "))
}

fn bound(b: &ast::Bound) -> String {
  match b {
    ast::Bound::Ref(r) => r.name.clone(),
    ast::Bound::Literal(l) => format!("{:?}", l.value),
    ast::Bound::Union(u) => u.options.iter().map(bound).collect::<Vec<_>>().join(" | "),
    ast::Bound::Structural(s) => {
      let mut parts: Vec<String> = s.fields.iter().map(matcher).collect();
      if s.rest {
        parts.push("..".to_string());
      }
      if parts.is_empty() {
        format!("{} {{}}", s.base.name)
      } else {
        format!("{} {{ {} }}", s.base.name, parts.join(", "))
      }
    }
    ast::Bound::FieldRef(f) => format!(".{}", f.field),
    ast::Bound::VarRef(v) => format!("${}", v.name),
  }
}

fn matcher(m: &ast::FieldMatcher) -> String {
  let mut s = m.name.name.clone();
  if let Some(b) = &m.binding {
    let _ = write!(s, " @ ${}", b.name);
  }
  if let Some(b) = &m.bound {
    let _ = write!(s, ": {}", bound(b));
  }
  s
}
