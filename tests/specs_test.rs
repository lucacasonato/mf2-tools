use std::panic;
use std::panic::AssertUnwindSafe;

use file_test_runner::collect_and_run_tests;
use file_test_runner::collection::strategies::TestPerFileCollectionStrategy;
use file_test_runner::collection::CollectOptions;
use file_test_runner::collection::CollectedTest;
use file_test_runner::RunOptions;
use file_test_runner::TestResult;
use mf2_parser::parse;

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

  let (message, expected_ast_dbg) = file_text
    .split_once("\n=== ast ===\n")
    .unwrap_or((&*file_text, ""));
  assert!(!message.is_empty());

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

  if std::env::var("UPDATE").is_ok() || expected_ast_dbg.is_empty() {
    std::fs::write(
      &test.path,
      format!("{}\n=== ast ===\n{}", message, actual_ast_dbg),
    )
    .unwrap();
    return;
  }

  pretty_assertions::assert_eq!(actual_ast_dbg, expected_ast_dbg);
}
