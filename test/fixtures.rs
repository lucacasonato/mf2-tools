use std::fmt::Write;
use std::panic;
use std::panic::AssertUnwindSafe;
use std::path::Path;

use file_test_runner::collect_and_run_tests;
use file_test_runner::collection::strategies::TestPerFileCollectionStrategy;
use file_test_runner::collection::CollectOptions;
use file_test_runner::collection::CollectedTest;
use file_test_runner::RunOptions;
use file_test_runner::TestResult;
use mf2_parser::ast;
use mf2_parser::ast::Message;
use mf2_parser::parse;
use mf2_parser::Location;
use mf2_parser::SourceTextInfo;
use mf2_parser::Span;
use mf2_parser::Spanned;
use mf2_parser::Visit;
use mf2_parser::Visitable;
use mf2_printer::print;
use unicode_width::UnicodeWidthStr;

mod utils;
use utils::generate_actual_diagnostics;
use utils::normalize_message;
use utils::parse_fixture;

fn main() {
  collect_and_run_tests(
    CollectOptions {
      base: Path::new("fixtures").into(),
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

  let spans_marker = "\n=== spans ===\n";
  let diagnostics_marker = "\n=== diagnostics ===\n";
  let formatted_marker = "\n=== formatted ===\n";
  let ast_marker = "\n=== ast ===\n";

  let cannot_format = "(cannot format due to fatal errors)".to_string();

  let mut parts = parse_fixture(
    &file_text,
    vec![
      spans_marker,
      diagnostics_marker,
      formatted_marker,
      ast_marker,
    ],
  );
  let message = parts.next().unwrap_or("");
  let expected_spans = parts.next().unwrap_or("");
  let expected_diagnostics = parts.next().unwrap_or("");
  let expected_formatted = parts.next().unwrap_or("");
  let expected_ast_dbg = parts.next().unwrap_or("");

  if test
    .path
    .file_name()
    .and_then(|f| f.to_str())
    .map(|s| s.ends_with(".panic"))
    .unwrap_or(false)
  {
    let result = panic::catch_unwind(|| parse(message));
    if result.is_ok() {
      panic!("expected panic, but parsing didn't");
    }
    return;
  }

  let normalized_message = normalize_message(message);

  let (actual_ast, diagnostics, info) = parse(message);
  let has_fatal_diag = diagnostics.iter().any(|d| d.fatal());

  let actual_ast_dbg = generated_actual_ast_dbg(&actual_ast);
  let actual_spans =
    generate_actual_spans(&actual_ast, message, &normalized_message, &info);
  let actual_diags =
    generate_actual_diagnostics(&diagnostics, message, &normalized_message);
  let actual_formatted = if has_fatal_diag {
    cannot_format
  } else {
    print(&actual_ast, Some(&info))
  };

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
    if expected_ast_dbg.is_empty() {
      need_update = true;
    } else {
      pretty_assertions::assert_eq!(
        actual_ast_dbg,
        expected_ast_dbg,
        "AST matches expected"
      );
    }
    if expected_spans.is_empty() {
      need_update = true;
    } else {
      pretty_assertions::assert_eq!(
        actual_spans,
        expected_spans,
        "Spans match expected"
      );
    }
    if expected_formatted.is_empty() {
      need_update = true;
    } else {
      pretty_assertions::assert_eq!(
        actual_formatted,
        expected_formatted,
        "Formatted code matches expected"
      );
    }
  }

  if need_update {
    std::fs::write(
      &test.path,
      format!(
        "{message}{spans_marker}{actual_spans}{diagnostics_marker}{actual_diags}{formatted_marker}{actual_formatted}{ast_marker}{actual_ast_dbg}"
      ),
    )
    .unwrap();
  }

  if has_fatal_diag {
    // test that it does not panic
    print(&actual_ast, Some(&info));
  } else {
    let (new_ast, new_diagnostics, new_info) = parse(&actual_formatted);

    let new_ast_dbg = generated_actual_ast_dbg(&new_ast);
    let re = regex::Regex::new(r"(span|start): @[\d\.]+").unwrap();
    let ast_before = re.replace_all(&actual_ast_dbg, "$1:@???");
    let ast_after = re.replace_all(&new_ast_dbg, "$1:@???");

    pretty_assertions::assert_str_eq!(
      ast_before,
      ast_after,
      "Formatting preserved the AST"
    );

    pretty_assertions::assert_eq!(
      diagnostics.len(),
      new_diagnostics.len(),
      "Formtting preserves the number of diagnostics"
    );
    for (old, new) in diagnostics.iter().zip(new_diagnostics.iter()) {
      assert_eq!(
        std::mem::discriminant(old),
        std::mem::discriminant(new),
        "Formatting preserves the diagnostics"
      );
    }

    let new_formatted = print(&new_ast, Some(&new_info));
    pretty_assertions::assert_eq!(
      actual_formatted,
      new_formatted,
      "Formatting is stable"
    );
  }
}

fn generated_actual_ast_dbg(actual_ast: &Message) -> String {
  format!("{actual_ast:#?}")
}

fn generate_actual_spans(
  actual_ast: &Message,
  input_message: &str,
  normalized_message: &str,
  source_text_info: &SourceTextInfo<'_>,
) -> String {
  const SPAN_LABEL_WIDTH: usize = 20;
  struct SpanDebuggerVisitor<'text> {
    input_message: &'text str,
    output: &'text mut String,
    source_text_info: &'text SourceTextInfo<'text>,
    last_start: Location,
  }

  impl SpanDebuggerVisitor<'_> {
    fn print(&mut self, name: &str, span: Span) {
      assert!(name.len() <= SPAN_LABEL_WIDTH);

      if span.start < self.last_start {
        panic!(
          "Item {} starting at {:?} is before the last span start {:?} - the visitor did not visit in source text order!",
          name, span.start, self.last_start
        );
      }
      self.last_start = span.start;

      let span_start = span.start.inner_byte_index_for_test() as usize;
      let span_end = span.end.inner_byte_index_for_test() as usize;

      let prefix = &self.input_message[0..span_start];
      let contents = &self.input_message[span_start..span_end];
      let suffix = &self.input_message[span_end..];

      let span_start_pos = self.source_text_info.utf8_line_col(span.start);
      let span_end_pos = self.source_text_info.utf8_line_col(span.end);

      write!(
        self.output,
        "\n{:<SPAN_LABEL_WIDTH$}{}{}{} {:?}-{:?}",
        name,
        " ".repeat(prefix.width_cjk()),
        "^".repeat(contents.width_cjk()),
        " ".repeat(suffix.width_cjk()),
        span_start_pos,
        span_end_pos
      )
      .unwrap();
    }
  }

  macro_rules! impl_visit_mut_for_span_debugger {
      {
        $( $ast:ident : $visit:ident, )*
      } => {
          $( fn $visit(&mut self, ast: &ast::$ast) {
            self.print(stringify!($ast), ast.span());
            ast.apply_visitor_to_children(self);
          } )*
        }
  }

  impl Visit<'_, '_> for SpanDebuggerVisitor<'_> {
    impl_visit_mut_for_span_debugger! {
      Pattern: visit_pattern,
      Text: visit_text,
      Escape: visit_escape,
      LiteralExpression: visit_literal_expression,
      Quoted: visit_quoted,
      Annotation: visit_annotation,
      Identifier: visit_identifier,
      FnOrMarkupOption: visit_fn_or_markup_option,
      Variable: visit_variable,
      Attribute: visit_attribute,
      VariableExpression: visit_variable_expression,
      AnnotationExpression: visit_annotation_expression,
      Markup: visit_markup,
      ComplexMessage: visit_complex_message,
      InputDeclaration: visit_input_declaration,
      LocalDeclaration: visit_local_declaration,
      QuotedPattern: visit_quoted_pattern,
      Matcher: visit_matcher,
      Variant: visit_variant,
      Star: visit_star,
    }

    fn visit_number(&mut self, num: &ast::Number) {
      self.print("Number", num.span());
      self.print("Number.integral", num.integral_span());
      if let Some(frac) = num.fractional_span() {
        self.print("Number.fractional", frac);
      }
      if let Some(exp) = num.exponent_span() {
        self.print("Number.exponent", exp);
      }

      num.apply_visitor_to_children(self);
    }
  }

  let mut output = " ".repeat(SPAN_LABEL_WIDTH);
  output.push_str(normalized_message);

  actual_ast.apply_visitor(&mut SpanDebuggerVisitor {
    input_message,
    output: &mut output,
    source_text_info,
    last_start: Location::new_for_test(0),
  });

  output
}
