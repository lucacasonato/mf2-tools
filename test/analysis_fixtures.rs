use std::panic::AssertUnwindSafe;
use std::path::Path;

use file_test_runner::collect_and_run_tests;
use file_test_runner::collection::strategies::TestPerFileCollectionStrategy;
use file_test_runner::collection::CollectOptions;
use file_test_runner::collection::CollectedTest;
use file_test_runner::RunOptions;
use file_test_runner::TestResult;
use mf2_parser::analyze_semantics;
use mf2_parser::parse;

mod utils;
use utils::generate_actual_diagnostics;
use utils::normalize_message;
use utils::parse_fixture;

fn main() {
  collect_and_run_tests(
    CollectOptions {
      base: Path::new("analysis_fixtures").into(),
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

  let diagnostics_marker = "\n=== diagnostics ===\n";

  let mut parts = parse_fixture(&file_text, vec![diagnostics_marker]);
  let message = parts.next().unwrap_or("");
  let expected_diagnostics = parts.next().unwrap_or("");

  let normalized_message = normalize_message(message);

  let (actual_ast, mut diagnostics, _info) = parse(message);
  let _scope = analyze_semantics(&actual_ast, &mut diagnostics);
  let actual_diags =
    generate_actual_diagnostics(&diagnostics, message, &normalized_message);

  let mut need_update = std::env::var("UPDATE").is_ok();
  if !need_update {
    if expected_diagnostics.is_empty() {
      need_update = true;
    } else {
      pretty_assertions::assert_eq!(
        actual_diags,
        expected_diagnostics,
        "Diagnostics match expected"
      );
    }
  }

  if need_update {
    std::fs::write(
      &test.path,
      format!("{message}{diagnostics_marker}{actual_diags}"),
    )
    .unwrap();
  }
}
