use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::ops::Add;
use std::ops::Range;
use std::str::Chars;

type Peek = Option<(Location, char)>;

enum Peeked {
  None,
  Single(Peek),
  Double((Location, char), Peek),
}

/// The source text, represented as a series of `char`s.
///
/// The source text maintains the offset of characters in the original string
/// and provides methods to iterate over the characters. It provides a method
/// to advance the iterator, and a method to peek the next character.
pub struct SourceTextIterator<'a> {
  original: &'a str,
  front_loc: Location,
  iter_offset: u32,
  iter: Chars<'a>,
  peeked: Peeked,
  utf8_line_starts: Vec<u32>,
}

impl<'a> SourceTextIterator<'a> {
  pub fn new(s: &'a str) -> Self {
    assert!(
      s.len() <= u32::MAX as usize,
      "source text is longer than u32::MAX"
    );
    SourceTextIterator {
      original: s,
      front_loc: Location(0),
      iter_offset: 0,
      iter: s.chars(),
      peeked: Peeked::None,
      utf8_line_starts: vec![0],
    }
  }

  /// Resets the iterator to the given location.
  ///
  /// ## Panics
  ///
  /// Panics if the location falls outside of the source text, or if the
  /// location is not at a character boundary.
  pub fn reset_to(&mut self, loc: Location) {
    assert!(loc.0 <= self.end_location().0);
    self.front_loc = loc;
    self.iter_offset = loc.0;
    self.peeked = Peeked::None;
    self.iter = self.original[self.iter_offset as usize..].chars();
  }

  fn iter_next(&mut self) -> Option<char> {
    self.iter.next().map(|ch| {
      self.iter_offset += ch.len_utf8() as u32;
      if ch == '\n' {
        if *self.utf8_line_starts.last().unwrap() < self.iter_offset {
          self.utf8_line_starts.push(self.iter_offset);
        }
      }
      ch
    })
  }

  pub fn next(&mut self) -> Option<(Location, char)> {
    match self.peeked {
      Peeked::None => self.iter_next().map(|ch| {
        let loc = self.front_loc;
        self.front_loc = Location(self.iter_offset);
        (loc, ch)
      }),
      Peeked::Single(None) => None,
      Peeked::Single(Some(peek)) | Peeked::Double(peek, None) => {
        self.front_loc = Location(self.iter_offset);
        self.peeked = Peeked::None;
        Some(peek)
      }
      Peeked::Double(peek1, peek2 @ Some((loc, _))) => {
        self.front_loc = loc;
        self.peeked = Peeked::Single(peek2);
        Some(peek1)
      }
    }
  }

  pub fn peek(&mut self) -> Peek {
    match &self.peeked {
      Peeked::Single(peek) => *peek,
      Peeked::Double(peek, _) => Some(*peek),
      Peeked::None => {
        let peeked = self.iter_next().map(|ch| (self.front_loc, ch));
        self.peeked = Peeked::Single(peeked);
        peeked
      }
    }
  }

  pub fn peek2(&mut self) -> Peek {
    if let Peeked::Double(_, peek) = self.peeked {
      return peek;
    }
    match self.peek() {
      None => None,
      Some(peek1) => {
        let loc = Location(self.iter_offset);
        let peek2 = self.iter_next().map(|ch2| (loc, ch2));
        self.peeked = Peeked::Double(peek1, peek2);
        peek2
      }
    }
  }

  pub fn current_location(&self) -> Location {
    self.front_loc
  }

  pub fn start_location(&self) -> Location {
    Location(0)
  }

  pub fn end_location(&self) -> Location {
    Location(self.original.len() as u32)
  }

  pub fn slice(&self, range: Range<Location>) -> &'a str {
    &self.original[range.start.0 as usize..range.end.0 as usize]
  }

  pub fn into_info(self) -> SourceTextInfo<'a> {
    SourceTextInfo {
      text: self.original,
      utf8_line_starts: self.utf8_line_starts,
    }
  }
}

pub struct SourceTextInfo<'a> {
  text: &'a str,
  utf8_line_starts: Vec<u32>,
}

impl SourceTextInfo<'_> {
  pub fn utf8_line_col(&self, loc: Location) -> (u32, u32) {
    let result = self.utf8_line_starts.binary_search_by(|&x| x.cmp(&loc.0));
    match result {
      Ok(line) => (line as u32, 0),
      Err(line) => {
        let line = line - 1;
        let col = loc.0 - self.utf8_line_starts[line];
        (line as u32, col)
      }
    }
  }

  pub fn utf16_line_col(&self, loc: Location) -> (u32, u32) {
    let result = self.utf8_line_starts.binary_search_by(|&x| x.cmp(&loc.0));
    match result {
      Ok(line) => (line as u32, 0),
      Err(line) => {
        let line = line - 1;
        let line_text =
          &self.text[self.utf8_line_starts[line] as usize..loc.0 as usize];
        let col = line_text
          .chars()
          .fold(0, |acc, c| acc + c.len_utf16() as u32);
        (line as u32, col)
      }
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub struct Location(u32);

impl Location {
  pub(crate) fn dummy() -> Location {
    Location(0)
  }

  pub fn inner_byte_index_for_test(&self) -> u32 {
    self.0
  }

  pub(crate) fn inner(&self) -> u32 {
    self.0
  }
}

impl Debug for Location {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "@{}", self.0)
  }
}

impl Add<&'_ str> for Location {
  type Output = Location;

  fn add(self, rhs: &'_ str) -> Self::Output {
    Location(self.0 + rhs.len() as u32)
  }
}

impl Add<char> for Location {
  type Output = Location;

  fn add(self, rhs: char) -> Self::Output {
    Location(self.0 + rhs.len_utf8() as u32)
  }
}

impl Add<LengthShort> for Location {
  type Output = Location;

  fn add(self, rhs: LengthShort) -> Self::Output {
    Location(self.0 + rhs.0 as u32)
  }
}

#[derive(Clone, Copy)]
pub struct Span {
  pub start: Location,
  pub end: Location,
}

impl Span {
  pub fn new(range: Range<Location>) -> Self {
    Span {
      start: range.start,
      end: range.end,
    }
  }
}

impl Debug for Span {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "@{}..{}", self.start.0, self.end.0)
  }
}

pub trait Spanned {
  fn span(&self) -> Span;
}

/// A short length (maximum u16)
#[derive(Clone, Copy)]
pub struct LengthShort(u16);

impl Debug for LengthShort {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl LengthShort {
  pub fn new(range: Range<Location>) -> LengthShort {
    LengthShort((range.end.0 - range.start.0) as u16)
  }

  pub fn new_from_str(str: &str) -> LengthShort {
    LengthShort(str.len() as u16)
  }

  pub fn inner(&self) -> u16 {
    self.0
  }
}
