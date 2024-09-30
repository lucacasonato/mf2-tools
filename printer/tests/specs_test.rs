use std::panic;
use std::panic::AssertUnwindSafe;
use std::path::Path;

use file_test_runner::collect_and_run_tests;
use file_test_runner::collection::strategies::TestPerFileCollectionStrategy;
use file_test_runner::collection::CollectOptions;
use file_test_runner::collection::CollectedTest;
use file_test_runner::RunOptions;
use file_test_runner::TestResult;
use mf2_parser::parse;
use mf2_printer::print;

fn main() {
  collect_and_run_tests(
    CollectOptions {
      base: Path::new("tests").join("printer"),
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

  let output_marker = "\n=== output ===\n";

  let (input, expected) = file_text
    .split_once(output_marker)
    .unwrap_or((&*file_text, ""));

  let (ast, diag, ..) = parse(input);
  pretty_assertions::assert_eq!(diag.len(), 0);

  if test
    .path
    .file_name()
    .and_then(|f| f.to_str())
    .map(|s| s.ends_with(".panic"))
    .unwrap_or(false)
  {
    let result = panic::catch_unwind(|| print(&ast));
    if result.is_ok() {
      panic!("expected panic, but printing didn't");
    }
    return;
  }

  let actual = print(&ast);

  let mut need_update = std::env::var("UPDATE").is_ok();
  if !need_update {
    if expected.is_empty() {
      need_update = true;
    } else {
      pretty_assertions::assert_eq!(actual, expected);
    }
  }

  if need_update {
    std::fs::write(&test.path, format!("{input}{output_marker}{actual}"))
      .unwrap();
  }
}
