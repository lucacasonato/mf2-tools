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
use mf2_parser::ast;
use mf2_parser::parse;
use mf2_parser::Span;
use mf2_parser::Spanned;
use mf2_parser::Visit;
use mf2_parser::Visitable;
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

  let spans_marker = "\n=== spans ===\n";
  let diagnostics_marker = "\n=== diagnostics ===\n";
  let ast_marker = "\n=== ast ===\n";

  let (message, rest_str) = file_text
    .split_once(spans_marker)
    .unwrap_or((&*file_text, ""));
  assert!(!message.is_empty());
  let (expected_diagnostics, rest_str) = rest_str
    .split_once(diagnostics_marker)
    .unwrap_or((&*rest_str, ""));
  let (expected_spans, rest_str) =
    rest_str.split_once(ast_marker).unwrap_or((&*rest_str, ""));
  let expected_ast_dbg = rest_str;

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

  let (actual_ast, diagnostics) = parse(message);
  let actual_ast_dbg = format!("{actual_ast:#?}");
  let actual_diags_dbg = format!("{diagnostics:#?}");

  const SPAN_LABEL_WIDTH: usize = 20;
  struct SpanDebuggerVisitor<'a> {
    input_message: &'a str,
    output: &'a mut String,
  }

  impl SpanDebuggerVisitor<'_> {
    fn print(&mut self, name: &str, span: Span) -> () {
      assert!(name.len() <= SPAN_LABEL_WIDTH);

      let span_start = span.start.inner_byte_index_for_test() as usize;
      let span_end = span.end.inner_byte_index_for_test() as usize;

      let prefix = &self.input_message[0..span_start];
      let contents = &self.input_message[span_start..span_end];

      write!(
        self.output,
        "\n{:<SPAN_LABEL_WIDTH$}{}{}",
        name,
        " ".repeat(prefix.width_cjk()),
        "^".repeat(contents.width_cjk())
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

  impl Visit for SpanDebuggerVisitor<'_> {
    impl_visit_mut_for_span_debugger! {
      SimpleMessage: visit_simple_message,
      Text: visit_text,
      Escape: visit_escape,
      LiteralExpression: visit_literal_expression,
      Quoted: visit_quoted,
      Function: visit_function,
      Identifier: visit_identifier,
      FnOrMarkupOption: visit_fn_or_markup_option,
      Variable: visit_variable,
      Attribute: visit_attribute,
      PrivateUseAnnotation: visit_private_use_annotation,
      ReservedAnnotation: visit_reserved_annotation,
      VariableExpression: visit_variable_expression,
      AnnotationExpression: visit_annotation_expression,
      Markup: visit_markup,
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

  let actual_spans = {
    let mut normalized_message = iter::repeat(' ')
      .take(SPAN_LABEL_WIDTH)
      .chain(message.chars().map(|c| match c {
        '\n' => '↵',
        '\t' => '⇥',
        c => c,
      }))
      .collect::<String>();

    actual_ast.apply_visitor(&mut SpanDebuggerVisitor {
      input_message: message,
      output: &mut normalized_message,
    });

    normalized_message
  };

  let mut need_update = std::env::var("UPDATE").is_ok();
  if !need_update {
    if expected_diagnostics.is_empty() {
      need_update = true;
    } else {
      pretty_assertions::assert_eq!(actual_diags_dbg, expected_diagnostics);
    }
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
        "{message}{spans_marker}{actual_spans}{diagnostics_marker}{actual_diags_dbg}{ast_marker}{actual_ast_dbg}"
      ),
    )
    .unwrap();
  }
}
