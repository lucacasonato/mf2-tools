use mf2_parser::ast;
use mf2_parser::ast::AnyNode;
use mf2_parser::ast::Message;
use mf2_parser::Diagnostic;
use mf2_parser::Location;
use mf2_parser::Scope;
use mf2_parser::Span;
use mf2_parser::Spanned;
use mf2_parser::Visit as _;
use mf2_parser::VisitAny;

#[derive(Debug)]
pub enum CompletionAction {
  Insert,
  Replace(Span),
}

#[derive(Debug)]
pub enum CompletionKind {
  Variable,
  Snippet,
}

#[derive(Debug)]
pub struct Completion {
  /// The text shown in the completion list.
  pub label: String,
  /// The text inserted when the completion is accepted. For
  /// [CompletionKind::Snippet] completions this is an LSP snippet (with `$1`
  /// tab stops and `\$` for literal dollars).
  pub text: String,
  pub action: CompletionAction,
  pub kind: CompletionKind,
}

#[derive(Debug)]
enum AllowedCompletionType<'text> {
  None,
  /// A variable reference is allowed here. `None` means there is no existing
  /// variable token, so a name is inserted at the cursor. `Some((span, name))`
  /// means a (possibly partial) variable `name` already occupies `span`, which
  /// is replaced by the chosen name.
  Variable(Option<(Span, &'text str)>),
  /// A declaration or matcher is allowed here (root of a complex or empty
  /// message). `allow_match` is false when the message already has a body (a
  /// matcher or a quoted pattern), since there can only be one. `replace` is the
  /// span of a partially-typed declaration keyword at the cursor (e.g. `.` or
  /// `.lo`) that the snippet should replace, if any.
  Declaration {
    allow_match: bool,
    replace: Option<Span>,
  },
}

pub struct CompletionsProvider<'scope: 'text, 'text> {
  scope: &'scope Scope<'text>,
  completion_type: AllowedCompletionType<'text>,
}

impl<'scope, 'text> CompletionsProvider<'scope, 'text> {
  pub fn new<'ast>(
    ast: &'ast Message<'text>,
    loc: Location,
    scope: &'scope Scope<'text>,
    diagnostics: &[Diagnostic<'text>],
  ) -> Self {
    Self {
      scope,
      completion_type: get_completion_type(ast, loc, diagnostics),
    }
  }

  pub fn has_completions(&self) -> bool {
    !matches!(self.completion_type, AllowedCompletionType::None)
  }

  pub fn get_completions(&self) -> Vec<Completion> {
    match self.completion_type {
      AllowedCompletionType::None => vec![],
      AllowedCompletionType::Declaration {
        allow_match,
        replace,
      } => declaration_completions(allow_match, replace),
      AllowedCompletionType::Variable(None) => self
        .scope
        .get_names()
        .map(|n| {
          let text = format!("${}", n);
          Completion {
            label: text.clone(),
            text,
            action: CompletionAction::Insert,
            kind: CompletionKind::Variable,
          }
        })
        .collect(),
      AllowedCompletionType::Variable(Some((span, name))) => {
        let include_self =
          name.len() > 1 && self.scope.get_spans(name).unwrap().len() > 1;

        let all_names = self
          .scope
          .get_names()
          .filter(|n| include_self || *n != name)
          .map(|n| {
            let text = format!("${}", n);
            Completion {
              label: text.clone(),
              text,
              action: CompletionAction::Replace(span),
              kind: CompletionKind::Variable,
            }
          });

        all_names.collect()
      }
    }
  }
}

fn declaration_completions(
  allow_match: bool,
  replace: Option<Span>,
) -> Vec<Completion> {
  let mut snippets = vec![
    (".input", ".input {\\$${1:var} :${2:string}$0"),
    (".local", ".local \\$${1:var} = {${2:value}}$0"),
  ];
  if allow_match {
    snippets.push((
      ".match",
      ".match \\$${1:var}\n${2:key} {{${3:value}}}\n* {{${4:other}}}$0",
    ));
  }

  snippets
    .into_iter()
    .map(|(label, text)| Completion {
      label: label.to_string(),
      text: text.to_string(),
      action: match replace {
        // Replace a partially-typed keyword (e.g. `.lo`) so we don't end up
        // with `..local`.
        Some(span) => CompletionAction::Replace(span),
        None => CompletionAction::Insert,
      },
      kind: CompletionKind::Snippet,
    })
    .collect()
}

struct CompletionLocationVisitor<'ast, 'text> {
  loc: Location,
  parent_node: AnyNode<'ast, 'text>,
  current_node: AnyNode<'ast, 'text>,
  previous_node: Option<AnyNode<'ast, 'text>>,
}

impl<'ast, 'text> VisitAny<'ast, 'text>
  for CompletionLocationVisitor<'ast, 'text>
{
  fn before(&mut self, node: AnyNode<'ast, 'text>) {
    let span = node.span();
    if span.start < self.loc && self.loc <= span.end {
      self.parent_node = std::mem::replace(&mut self.current_node, node);
      assert!(!self.parent_node.same(&self.current_node));
      self.previous_node = None;
    }
  }

  fn after(&mut self, node: AnyNode<'ast, 'text>) {
    if node.span().end < self.loc && !node.span().is_empty() {
      self.previous_node = Some(node);
    }
  }
}

fn get_completion_type<'text>(
  ast: &Message<'text>,
  loc: Location,
  diagnostics: &[Diagnostic<'text>],
) -> AllowedCompletionType<'text> {
  let mut visitor = CompletionLocationVisitor {
    loc,
    current_node: AnyNode::Message(ast),
    parent_node: AnyNode::Message(ast),
    previous_node: None,
  };
  visitor.visit_message(ast);

  let CompletionLocationVisitor {
    current_node,
    parent_node,
    previous_node,
    ..
  } = visitor;

  use ast::*;
  use AnyNode as X;

  let get_invalid_statement_span = || {
    diagnostics.iter().find_map(|diagnostic| match diagnostic {
      Diagnostic::InvalidStatement { span, .. }
        if span.start < loc && loc <= span.end =>
      {
        Some(*span)
      }
      _ => None,
    })
  };

  // An empty (or whitespace-only) message is parsed as a simple message with a
  // single empty text part. Offer declaration snippets at its root.
  if let Message::Simple(pattern) = ast {
    if let [PatternPart::Text(text)] = &pattern.parts[..] {
      if text.content.trim().is_empty() {
        return AllowedCompletionType::Declaration {
          allow_match: true,
          replace: get_invalid_statement_span(),
        };
      }
    }
  }

  let has_body = |message: &ComplexMessage| match &message.body {
    ComplexMessageBody::Matcher(_) => true,
    ComplexMessageBody::QuotedPattern(pattern) => !pattern.span().is_empty(),
  };

  // The cursor can be in whitespace just outside the complex message's span
  // (e.g. on a fresh line after the last declaration). As long as there is no
  // body yet, that is still a valid spot for another declaration.
  if let (X::Message(_), Message::Complex(message)) = (&current_node, ast) {
    if !has_body(message) {
      return AllowedCompletionType::Declaration {
        allow_match: true,
        replace: get_invalid_statement_span(),
      };
    }
  }

  match (current_node, parent_node, previous_node) {
    (X::ComplexMessage(message), _, _) => {
      // At the root of a complex message, in between declarations or before the
      // body: `.local ...` <here> `{{...}}`.
      AllowedCompletionType::Declaration {
        replace: get_invalid_statement_span(),
        allow_match: !has_body(message),
      }
    }
    (X::Variable(var), _, _) => {
      // $f|
      AllowedCompletionType::Variable(Some((var.span(), var.name)))
    }
    (X::LiteralExpression(literal_expression), _, None)
      if literal_expression.literal.span().is_empty() =>
    {
      // { | }
      AllowedCompletionType::Variable(None)

      // if excludes:
      //  { | 1 }
      //  { 1 | }
    }
    (
      X::FnOrMarkupOption(FnOrMarkupOption { value, .. }),
      _,
      Some(X::Identifier(_)),
    ) if value.span().is_empty() && value.span().start == loc => {
      // :fn param=|
      AllowedCompletionType::Variable(None)

      // if excludes:
      //  :fn param=f|
      //  :fn param |=
    }
    (
      X::VariableExpression(_)
      | X::AnnotationExpression(_)
      | X::LiteralExpression(_),
      _,
      Some(X::FnOrMarkupOption(opt)),
    ) => {
      #[allow(clippy::collapsible_match)]
      if let LiteralOrVariable::Literal(Literal::Text(text)) = &opt.value {
        if text.span().is_empty() && text.span().start != opt.key.span().end {
          // { $a :fn param= | }
          return AllowedCompletionType::Variable(None);
        }
      }
      AllowedCompletionType::None
    }
    (X::AnnotationExpression(_), _, None) => {
      // { | :fn }
      AllowedCompletionType::Variable(None)
    }
    _ => AllowedCompletionType::None,
  }
}

#[cfg(test)]
mod tests {
  use mf2_parser::parse;
  use mf2_parser::Location;

  use super::get_completion_type;
  use super::AllowedCompletionType;

  macro_rules! assert_completion_type {
    ($source:expr, $expected:pat) => {
      let loc = Location::new_for_test(
        $source.find('┋').expect("Cursor not found") as u32,
      );
      let message = $source.replace('┋', "");
      let (ast, diagnostics, _) = parse(&message);
      let result = get_completion_type(&ast, loc, &diagnostics);

      assert!(
        matches!(result, $expected),
        "expected: {}\nactual: {:?}",
        stringify!($expected),
        result
      );
    };
  }

  #[test]
  #[rustfmt::skip]
  fn works() {
    assert_completion_type!("{$┋}", AllowedCompletionType::Variable(Some((_, ""))));
    assert_completion_type!("{$f┋}", AllowedCompletionType::Variable(Some((_, "f"))));
    assert_completion_type!("{$┋f}", AllowedCompletionType::Variable(Some((_, "f"))));
    assert_completion_type!("{┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ ┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{┋ }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ ┋ }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{1┋}", AllowedCompletionType::None);
    assert_completion_type!("{1 ┋}", AllowedCompletionType::None);
    assert_completion_type!("{┋1}", AllowedCompletionType::None);
    assert_completion_type!("{ ┋1}", AllowedCompletionType::None);
    assert_completion_type!("{:fn ┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{:fn param┋=}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param ┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param ┋=}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param= ┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{:fn param=$f┋}", AllowedCompletionType::Variable(Some((_, "f"))));
    assert_completion_type!("{:fn param=f┋}", AllowedCompletionType::None);
    assert_completion_type!("{:fn param=┋f}", AllowedCompletionType::None);
    assert_completion_type!("{:fn p1=a p2=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{:fn p1=a p2=b┋}", AllowedCompletionType::None);
    assert_completion_type!("{ 1 :fn param=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ $x :fn param=┋}", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ ┋ :fn }", AllowedCompletionType::Variable(None));
    assert_completion_type!("{ $x┋ :fn }", AllowedCompletionType::Variable(Some((_, "x"))));

    // Declaration snippets at the root of an empty or complex message. A
    // partially-typed keyword is reported as a replace span.
    assert_completion_type!("┋", AllowedCompletionType::Declaration { allow_match: true, replace: None });
    assert_completion_type!("  ┋  ", AllowedCompletionType::Declaration { allow_match: true, replace: None });
    assert_completion_type!(".┋", AllowedCompletionType::Declaration { allow_match: true, replace: Some(_) });
    assert_completion_type!(".in┋", AllowedCompletionType::Declaration { allow_match: true, replace: Some(_) });
    assert_completion_type!(".input {$x}\n┋", AllowedCompletionType::Declaration { allow_match: true, replace: None });
    // A partial keyword after an earlier declaration is still replaced.
    assert_completion_type!(".local $x = {1}\n.ma┋", AllowedCompletionType::Declaration { allow_match: true, replace: Some(_) });
    // A body already exists, so `.match` is not offered.
    assert_completion_type!(".local $x = {1}\n┋\n{{hi}}", AllowedCompletionType::Declaration { allow_match: false, replace: None });
    assert_completion_type!(".local $x = {1}\n┋\n.match $x\n* {{a}}", AllowedCompletionType::Declaration { allow_match: false, replace: None });
    // Inside a matcher variant's pattern, declarations are not offered.
    assert_completion_type!(".match $x\n* {{┋}}", AllowedCompletionType::None);
    // Not at the root of a simple message with content.
    assert_completion_type!("┋Hello", AllowedCompletionType::None);
    assert_completion_type!("Hello┋", AllowedCompletionType::None);
    assert_completion_type!("{{┋}}", AllowedCompletionType::None);
  }
}
