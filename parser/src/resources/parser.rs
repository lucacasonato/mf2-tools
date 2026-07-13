use crate::Location;
use crate::SourceTextInfo;
use crate::Span;
use crate::Spanned as _;
use crate::resources::ast::Comment;
use crate::resources::ast::Name;
use crate::resources::ast::NamePart;
use crate::resources::ast::NameText;
use crate::resources::ast::Resource;
use crate::resources::chars;
use crate::resources::diagnostic::Diagnostic;
use crate::text::SourceTextIterator;

macro_rules! eat_line {
  ($parser:expr, $valid:pat, $report:expr) => {
    let mut start_of_invalid = None;
    while let Some((loc, c)) = $parser.peek() {
      $parser.next();
      match c {
        chars::newline!() => break,
        $valid => {
          if let Some(start_of_invalid) = start_of_invalid.take() {
            let span = Span::new(start_of_invalid..loc);
            $report(span);
          }
        }
        _ => {
          if start_of_invalid.is_none() {
            start_of_invalid = Some(loc);
          }
        }
      }
      if let Some(start_of_invalid) = start_of_invalid.take() {
        let span = Span::new(start_of_invalid..$parser.text.current_location());
        $report(span);
      }
    }
  };
}

pub struct Parser<'text> {
  text: SourceTextIterator<'text>,
  diagnostics: Vec<Diagnostic<'text>>,
}

impl<'text> Parser<'text> {
  pub fn new(input: &'text str) -> Self {
    Self {
      text: SourceTextIterator::new(input),
      diagnostics: vec![],
    }
  }

  pub fn parse(
    mut self,
  ) -> (
    Resource<'text>,
    Vec<Diagnostic<'text>>,
    SourceTextInfo<'text>,
  ) {
    let mut resource = Resource {
      span: Span::new(self.text.start_location()..self.text.end_location()),
      comments: vec![],
      meta: vec![],
      sections: vec![],
    };

    let mut comments = vec![];
    let mut meta = vec![];
    let mut has_encounted_frontmatter_seperator = false;

    loop {
      let has_leading_whitespace = self.skip_spaces();
      let Some((start, c)) = self.peek() else { break };
      match c {
        chars::newline!() => {
          // empty line (which is allowed to have whitespace)
          self.next();
        }
        '#' => {
          let comment = self.parse_comment();
          if has_leading_whitespace {
            self.report(Diagnostic::CommentHasLeadingWhitespace {
              span: comment.span(),
            });
          }
          if meta.len() == 0 {
            comments.push(comment);
          } else {
            // TODO: consider also adding the comment, but requires Entry::span() to be more complex
            self.report(Diagnostic::CommentAfterMetadata {
              span: comment.span(),
            });
          }
        }
        '@' => todo!("metadata"),
        '[' => todo!("section"),
        '-'
          if let Some((_, '-')) = self.peek2()
            && let Some((_, '-')) = self.peek3() =>
        {
          self.parse_frontmatter_separator();
          if !has_encounted_frontmatter_seperator {
            has_encounted_frontmatter_seperator = true;
            resource.comments = std::mem::take(&mut comments);
            resource.meta = std::mem::take(&mut meta);
          } else {
            self.report(Diagnostic::DuplicateFrontmatterSeparator {
              duplicate: Span::new(start..start + "---"),
            });
          }
        }
        _ => todo!("entry"),
      }
    }

    (resource, self.diagnostics, self.text.into_info())
  }

  fn parse_name(&mut self) -> Name<'text> {
    let mut parts = vec![];
    let text_start_location = self.text.current_location();
    while let Some((loc, c)) = self.peek() {
      match c {
        chars::id_safe!() => {
          self.next();
        }
        '\\' => {
          if text_start_location != loc {
            let text = self.text.slice(text_start_location..loc);
            parts.push(NamePart::Text(NameText {
              start: text_start_location,
              text,
            }));
          }
          let escape = self.parse_escape(EscapeType::Name);
          parts.push(NamePart::Escape(escape));
          text_start_location = self.text.current_location();
        }
        _ => break,
      }
    }
  }

  fn parse_comment(&mut self) -> Comment<'text> {
    let Some((start, _)) = self.next() else {
      unreachable!()
    };
    eat_line!(self, chars::content!(), |span| self
      .report(Diagnostic::InvalidCommentContent { span }));
    let text = self.text.slice(start..self.text.current_location());
    Comment { start, text }
  }

  fn parse_frontmatter_separator(&mut self) {
    self.next().unwrap(); // consume '-'
    self.next().unwrap(); // consume '-'
    self.next().unwrap(); // consume '-'
    let Some((_, chars::space!())) = self.peek() else {
      // This is actually an invalid identifier for an entry - let's parse it like that, and then report an error for the invalid identifier.
      todo!("invalid entry id?");
    };
    // After the `---`, there can be whitespace up until a newline.
    // Any non-whitespace content after the `---` is invalid, and should be reported as an error.
    eat_line!(self, chars::space!(), |span| self
      .report(Diagnostic::NonWhitespaceAfterFrontmatterSeperator { span }));
  }

  fn report(&mut self, diagnostic: Diagnostic<'text>) {
    self.diagnostics.push(diagnostic);
  }

  fn next(&mut self) -> Option<(Location, char)> {
    self.text.next()
  }

  fn peek(&mut self) -> Option<(Location, char)> {
    self.text.peek()
  }

  fn peek2(&mut self) -> Option<(Location, char)> {
    self.text.peek2()
  }

  fn peek3(&mut self) -> Option<(Location, char)> {
    self.text.peek3()
  }

  fn eat(&mut self, c: char) -> Option<Location> {
    if let Some((loc, ch)) = self.text.peek()
      && ch == c
    {
      self.text.next();
      return Some(loc);
    }
    None
  }

  fn skip_spaces(&mut self) -> bool {
    let mut any_spaces = false;
    while let Some((_, chars::space!())) = self.peek() {
      any_spaces = true;
      self.next();
    }
    any_spaces
  }

  fn parse_escape(&self, name: EscapeType) -> (_, super::ast::NameEscape) {}
}

enum EscapeType {
  Name,
  Value,
}
