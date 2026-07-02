use crate::Location;
use crate::SourceTextInfo;
use crate::resources::ast::Name;
use crate::resources::ast::Resource;
use crate::resources::chars;
use crate::resources::diagnostic::Diagnostic;
use crate::text::SourceTextIterator;

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
      comments: vec![],
      meta: vec![],
      frontmatter: None,
      sections: vec![],
    };

    loop {
      let has_leading_whitespace = self.skip_spaces();
      let Some(_, c) = self.peek() else { break };
    }

    while let Some((_, c)) = self.peek() {
      match c {
        '-' => {
          if self.peek2() == Some((_, '-')) {
          } else {
            break;
          }
        }
        _ => {}
      }
    }

    (resource, self.diagnostics, self.text.into_info())
  }

  fn parse_name(&mut self) -> Name<'text> {
    let mut parts = vec![];

    while let Some((_, c)) = self.peek() {
      match c {
        _ => break,
      }
    }

    
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
}
