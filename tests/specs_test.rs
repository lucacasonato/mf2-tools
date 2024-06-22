use std::fmt::Write;
use std::iter;
use std::panic;
use std::panic::AssertUnwindSafe;

use file_test_runner::collect_and_run_tests;
use file_test_runner::collection::strategies::TestPerFileCollectionStrategy;
use file_test_runner::collection::CollectOptions;
use file_test_runner::collection::CollectedTest;
use file_test_runner::RunOptions;
use file_test_runner::TestResult;
use mf2_parser::parse;
use mf2_parser::Span;
use mf2_parser::Spanned;
use unicode_width::UnicodeWidthStr;

fn main() {
  collect_and_run_tests(
    CollectOptions {
      base: "tests/parser".into(),
      strategy: Box::new(TestPerFileCollectionStrategy { file_pattern: None }),
      filter_override: None,
    },
    RunOptions { parallel: true },
    |test| {
      TestResult::from_maybe_panic(AssertUnwindSafe(|| {
        run_test(test);
      }))
    },
  )
}

fn run_test(test: &CollectedTest) {
  let file_text = test.read_to_string().unwrap();

  let ast_marker = "\n=== ast ===\n";
  let spans_marker = "\n=== spans ===\n";

  let (message, rest_str) = file_text
    .split_once(ast_marker)
    .unwrap_or((&*file_text, ""));
  assert!(!message.is_empty());
  let (expected_ast_dbg, rest_str) = rest_str
    .split_once(spans_marker)
    .unwrap_or((&*rest_str, ""));
  let expected_spans = rest_str;

  if test
    .path
    .file_name()
    .and_then(|f| f.to_str())
    .map(|s| s.ends_with(".panic"))
    .unwrap_or(false)
  {
    let result = panic::catch_unwind(|| parse(message));
    if !result.is_err() {
      panic!("expected panic, but parsing didn't");
    }
    return;
  }

  let actual_ast = parse(message);
  let actual_ast_dbg = format!("{actual_ast:#?}");

  let actual_spans = {
    let mut normalized_message = iter::repeat(' ')
      .take(20)
      .chain(message.chars().map(|c| match c {
        '\n' => '↵',
        '\t' => '⇥',
        c => c,
      }))
      .collect::<String>();
    {
      print_span(
        &mut normalized_message,
        "TheSpanName",
        &message,
        actual_ast.span(),
      );
    }
    normalized_message
  };

  let mut need_update = std::env::var("UPDATE").is_ok();
  if !need_update {
    if expected_ast_dbg.is_empty() {
      need_update = true;
    } else {
      pretty_assertions::assert_eq!(actual_ast_dbg, expected_ast_dbg);
    }
    if expected_spans.is_empty() {
      need_update = true;
    } else {
      pretty_assertions::assert_eq!(actual_spans, expected_spans);
    }
  }

  if need_update {
    std::fs::write(
      &test.path,
      format!(
        "{message}{ast_marker}{actual_ast_dbg}{spans_marker}{actual_spans}"
      ),
    )
    .unwrap();
  }
}

fn print_span(output: &mut String, name: &str, text: &str, span: Span) -> () {
  let span_start = span.start.inner_byte_index_for_test() as usize;
  let span_end = span.end.inner_byte_index_for_test() as usize;

  let prefix = &text[0..span_start];
  let contents = &text[span_start..span_end];

  write!(
    output,
    "\n{:<20}{}{}",
    name,
    " ".repeat(prefix.width_cjk()),
    "^".repeat(contents.width_cjk())
  )
  .unwrap();
}
