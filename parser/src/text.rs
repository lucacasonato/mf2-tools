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
pub struct SourceTextIterator<'text> {
  original: &'text str,
  front_loc: Location,
  str_index: u32,
  iter: Chars<'text>,
  peeked: Peeked,
  utf8_line_starts: Vec<u32>,
  prev_char_was_cr: bool,
}

impl<'text> SourceTextIterator<'text> {
  pub fn new(s: &'text str) -> Self {
    assert!(
      s.len() <= u32::MAX as usize,
      "source text is longer than u32::MAX"
    );
    SourceTextIterator {
      original: s,
      front_loc: Location(0),
      str_index: 0,
      iter: s.chars(),
      peeked: Peeked::None,
      utf8_line_starts: vec![0],
      prev_char_was_cr: false,
    }
  }

  /// Resets the iterator to the given location.
  ///
  /// The location reset to must be before the current location to ensure line
  /// start tracking is correct.
  ///
  /// ## Panics
  ///
  /// Panics if the location falls outside of the source text, or if the
  /// location is not at a character boundary.
  pub fn reset_to(&mut self, loc: Location) {
    assert!(loc <= self.front_loc);
    assert!(loc.0 <= self.end_location().0);
    self.front_loc = loc;
    self.str_index = loc.0;
    self.peeked = Peeked::None;
    self.iter = self.original[self.str_index as usize..].chars();
    self.prev_char_was_cr =
      self.original[..self.str_index as usize].ends_with('\r');
  }

  fn iter_next(&mut self) -> Option<char> {
    self.iter.next().map(|ch| {
      match ch {
        '\n' => {
          if *self.utf8_line_starts.last().unwrap() < self.str_index + 1 {
            self.utf8_line_starts.push(self.str_index + 1);
          }
          self.prev_char_was_cr = false;
        }
        _ => {
          if self.prev_char_was_cr
            && *self.utf8_line_starts.last().unwrap() < self.str_index
          {
            self.utf8_line_starts.push(self.str_index);
          }
          self.prev_char_was_cr = ch == '\r';
        }
      }
      self.str_index += ch.len_utf8() as u32;
      ch
    })
  }

  pub fn next(&mut self) -> Option<(Location, char)> {
    match self.peeked {
      Peeked::None => self.iter_next().map(|ch| {
        let loc = self.front_loc;
        self.front_loc = Location(self.str_index);
        (loc, ch)
      }),
      Peeked::Single(None) => None,
      Peeked::Single(Some(peek)) | Peeked::Double(peek, None) => {
        self.front_loc = Location(self.str_index);
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
        let loc = Location(self.str_index);
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

  pub fn slice(&self, range: Range<Location>) -> &'text str {
    &self.original[range.start.0 as usize..range.end.0 as usize]
  }

  pub fn into_info(mut self) -> SourceTextInfo<'text> {
    assert_eq!(self.str_index, self.original.len() as u32);
    if self.prev_char_was_cr
      && *self.utf8_line_starts.last().unwrap() < self.str_index
    {
      self.utf8_line_starts.push(self.str_index);
    }
    SourceTextInfo {
      text: self.original,
      utf8_line_starts: self.utf8_line_starts,
    }
  }
}

/// A view onto the source text, with additional information about the source
/// text that was derived during parsing.
///
/// This struct provides methods to convert between opaque [Location] values,
/// UTF-8 line and column indices, and UTF-16 line and column indices. It also
/// provides methods to calculate the length of a span in UTF-8 bytes or UTF-16
/// code units.
pub struct SourceTextInfo<'text> {
  text: &'text str,
  utf8_line_starts: Vec<u32>,
}

impl Spanned for SourceTextInfo<'_> {
  fn span(&self) -> Span {
    Span {
      start: Location(0),
      end: Location(self.text.len() as u32),
    }
  }
}

impl<'text> SourceTextInfo<'text> {
  pub fn text(&self, span: Span) -> &'text str {
    &self.text[span.start.0 as usize..span.end.0 as usize]
  }
}

impl SourceTextInfo<'_> {
  /// Returns a UTF-8 line and column index pair given a [Location].
  ///
  /// It is undefined behavior to pass a location that is out of bounds for the
  /// source text.
  pub fn utf8_line_col(&self, loc: Location) -> LineColUtf8 {
    let result = self.utf8_line_starts.binary_search_by(|&x| x.cmp(&loc.0));
    match result {
      Ok(line) => LineColUtf8 {
        line: line as u32,
        col: 0,
      },
      Err(line) => {
        let line = line - 1;
        let col = loc.0 - self.utf8_line_starts[line];
        LineColUtf8 {
          line: line as u32,
          col,
        }
      }
    }
  }

  /// Returns a UTF-16 line and column index pair given a [Location].
  ///
  /// It is undefined behavior to pass a location that is out of bounds for the
  /// source text.
  pub fn utf16_line_col(&self, loc: Location) -> LineColUtf16 {
    let result = self.utf8_line_starts.binary_search_by(|&x| x.cmp(&loc.0));
    match result {
      Ok(line) => LineColUtf16 {
        line: line as u32,
        col: 0,
      },
      Err(line) => {
        let line = line - 1;
        let line_text =
          &self.text[self.utf8_line_starts[line] as usize..loc.0 as usize];
        let col = line_text
          .chars()
          .fold(0, |acc, c| acc + c.len_utf16() as u32);
        LineColUtf16 {
          line: line as u32,
          col,
        }
      }
    }
  }

  /// Returns the length of the given span in UTF-8 bytes.
  pub fn utf8_len(&self, span: Span) -> u32 {
    span.end.0 - span.start.0
  }

  /// Returns the length of the given span in UTF-16 code units.
  pub fn utf16_len(&self, span: Span) -> u32 {
    let text = &self.text[span.start.0 as usize..span.end.0 as usize];
    text.chars().fold(0, |acc, c| acc + c.len_utf16() as u32)
  }

  /// Returns the location of the given UTF-8 line and column index pair.
  ///
  /// If the line index is out of bounds, returns a location pointing to the end
  /// of the source text.
  ///
  /// If the column index is greater than the line length, it is clamped to the
  /// line length. If the column index points to within a multi-byte character,
  /// the location will point to the the start of that character.
  pub fn utf8_loc(&self, line_col: LineColUtf8) -> Location {
    let line = line_col.line as usize;
    let line_start = match self.utf8_line_starts.get(line) {
      Some(&x) => x as usize,
      None => return Location(self.text.len() as u32),
    };
    let line_end = self
      .utf8_line_starts
      .get(line + 1)
      .map(|&x| x as usize)
      .unwrap_or_else(|| self.text.len());
    let line_text = &self.text[line_start..line_end];

    let mut col = line_col.col as usize;
    let mut location = Location(line_start as u32);
    for ch in line_text.chars() {
      col = match col.checked_sub(ch.len_utf8()) {
        Some(x) => x,
        None => break,
      };
      location = location + ch;
      if col == 0 {
        break;
      }
    }
    location
  }

  /// Returns the location of the given UTF-16 line and column index pair.
  ///
  /// If the line index is out of bounds, returns a location pointing to the end
  /// of the source text.
  ///
  /// If the column index is greater than the line length, it is clamped to the
  /// line length. If the column index points to within a multi-byte character,
  /// the location will point to the the start of that character.
  pub fn utf16_loc(&self, line_col: LineColUtf16) -> Location {
    let line = line_col.line as usize;
    let line_start = match self.utf8_line_starts.get(line) {
      Some(&x) => x as usize,
      None => return Location(self.text.len() as u32),
    };
    let line_end = self
      .utf8_line_starts
      .get(line + 1)
      .map(|&x| x as usize)
      .unwrap_or_else(|| self.text.len());
    let line_text = &self.text[line_start..line_end];

    let mut col = line_col.col as usize;
    let mut location = Location(line_start as u32);
    for ch in line_text.chars() {
      col = match col.checked_sub(ch.len_utf16()) {
        Some(x) => x,
        None => break,
      };
      location = location + ch;
      if col == 0 {
        break;
      }
    }
    location
  }
}

/// A location is an opaque value that is used to represent a position in the
/// source text. It can be mapped to UTF-8 byte indices, UTF-8 line and column,
/// or UTF-16 line and column indices in the source text using the
/// [SourceTextInfo] struct.
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub struct Location(u32);

impl Location {
  #[doc(hidden)]
  pub fn new_for_test(byte: u32) -> Location {
    Location(byte)
  }

  #[doc(hidden)]
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

/// A span is a pair of [Location]s that represent a range in the source text.
///
/// The start location is inclusive, and the end location is exclusive. A span
/// with the same start and end location is considered empty.
#[derive(Clone, Copy)]
pub struct Span {
  pub start: Location,
  pub end: Location,
}

impl Span {
  /// Creates a new span from a range of [Location]s. The range must be valid, i.e.
  /// the start location must be less than or equal to the end location.
  ///
  /// ### Panics
  ///
  /// In debug builds, panics if the range is invalid.
  pub fn new(range: Range<Location>) -> Self {
    debug_assert!(range.start <= range.end);
    Span {
      start: range.start,
      end: range.end,
    }
  }

  /// Whether the span contains the given [Location].
  pub fn contains_loc(&self, loc: Location) -> bool {
    self.start.0 <= loc.0 && self.end.0 > loc.0
  }

  /// Whether the span fully contains the given span. This includes the case
  /// where the spans are equal.
  pub fn contains(&self, other: &Span) -> bool {
    self.start.0 <= other.start.0 && self.end.0 >= other.end.0
  }

  /// Whether the span is empty.
  pub fn is_empty(&self) -> bool {
    self.start == self.end
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

/// A line and column index pair, both 0-based, for the UTF-8 encoding of the
/// source text.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineColUtf8 {
  pub line: u32,
  pub col: u32,
}

impl Debug for LineColUtf8 {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}:{}", self.line, self.col)
  }
}

/// A line and column index pair, both 0-based, for the UTF-16 encoding of the
/// source text.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineColUtf16 {
  pub line: u32,
  pub col: u32,
}

impl Debug for LineColUtf16 {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}:{}", self.line, self.col)
  }
}

#[cfg(test)]
mod tests {
  const SOURCE: &str = "a\nbc\r\nf\r🍊😅🎃\r\nasd🍊a";

  #[test]
  fn source_text_line_col_from_loc() {
    let mut source_text = super::SourceTextIterator::new(SOURCE);
    while source_text.next().is_some() {}
    let info = source_text.into_info();

    macro_rules! assert_utf8_line_col {
      ($byte:literal == ($line:literal, $col:literal)) => {
        assert_eq!(
          info.utf8_line_col(super::Location($byte)),
          super::LineColUtf8 {
            line: $line,
            col: $col
          },
          "byte {}",
          $byte
        );
      };
    }

    assert_utf8_line_col!(0 == (0, 0));
    assert_utf8_line_col!(1 == (0, 1));
    assert_utf8_line_col!(2 == (1, 0));
    assert_utf8_line_col!(3 == (1, 1));
    assert_utf8_line_col!(4 == (1, 2));
    assert_utf8_line_col!(5 == (1, 3));
    assert_utf8_line_col!(6 == (2, 0));
    assert_utf8_line_col!(7 == (2, 1));
    assert_utf8_line_col!(8 == (3, 0));
    // 9, 10, 11 are in the middle of the multi-byte character 🍊
    assert_utf8_line_col!(12 == (3, 4));
    // 13, 14, 15 are in the middle of the multi-byte character 😅
    assert_utf8_line_col!(16 == (3, 8));
    // 17, 18, 19 are in the middle of the multi-byte character 🎃
    assert_utf8_line_col!(20 == (3, 12));
    assert_utf8_line_col!(21 == (3, 13));
    assert_utf8_line_col!(22 == (4, 0));
    assert_utf8_line_col!(23 == (4, 1));
    assert_utf8_line_col!(24 == (4, 2));
    assert_utf8_line_col!(25 == (4, 3));
    // 26, 27, 28 are in the middle of the multi-byte character 🍊
    assert_utf8_line_col!(29 == (4, 7));
    assert_utf8_line_col!(30 == (4, 8));

    macro_rules! assert_utf16_line_col {
      ($byte:literal == ($line:literal, $col:literal)) => {
        assert_eq!(
          info.utf16_line_col(super::Location($byte)),
          super::LineColUtf16 {
            line: $line,
            col: $col
          },
          "byte {}",
          $byte
        );
      };
    }

    assert_utf16_line_col!(0 == (0, 0));
    assert_utf16_line_col!(1 == (0, 1));
    assert_utf16_line_col!(2 == (1, 0));
    assert_utf16_line_col!(3 == (1, 1));
    assert_utf16_line_col!(4 == (1, 2));
    assert_utf16_line_col!(5 == (1, 3));
    assert_utf16_line_col!(6 == (2, 0));
    assert_utf16_line_col!(7 == (2, 1));
    assert_utf16_line_col!(8 == (3, 0));
    // 9, 10, 11 are in the middle of the multi-byte character 🍊
    assert_utf16_line_col!(12 == (3, 2));
    // 13, 14, 15 are in the middle of the multi-byte character 😅
    assert_utf16_line_col!(16 == (3, 4));
    // 17, 18, 19 are in the middle of the multi-byte character 🎃
    assert_utf16_line_col!(20 == (3, 6));
    assert_utf16_line_col!(21 == (3, 7));
    assert_utf16_line_col!(22 == (4, 0));
    assert_utf16_line_col!(23 == (4, 1));
    assert_utf16_line_col!(24 == (4, 2));
    assert_utf16_line_col!(25 == (4, 3));
    // 26, 27, 28 are in the middle of the multi-byte character 🍊
    assert_utf16_line_col!(29 == (4, 5));
    assert_utf16_line_col!(30 == (4, 6));
  }

  #[test]
  fn source_text_loc_from_line_col() {
    let mut source_text = super::SourceTextIterator::new(SOURCE);
    while source_text.next().is_some() {}
    let info = source_text.into_info();

    macro_rules! assert_utf8_loc {
      (($line:literal, $col:literal) == $byte:literal) => {
        assert_eq!(
          info.utf8_loc(super::LineColUtf8 {
            line: $line,
            col: $col
          }),
          super::Location($byte),
          "loc {}:{}",
          $line,
          $col
        );
      };
    }

    assert_utf8_loc!((0, 0) == 0);
    assert_utf8_loc!((0, 1) == 1);
    assert_utf8_loc!((1, 0) == 2);
    assert_utf8_loc!((1, 1) == 3);
    assert_utf8_loc!((1, 2) == 4);
    assert_utf8_loc!((1, 3) == 5);
    assert_utf8_loc!((2, 0) == 6);
    assert_utf8_loc!((2, 1) == 7);
    assert_utf8_loc!((3, 0) == 8);
    assert_utf8_loc!((3, 1) == 8);
    assert_utf8_loc!((3, 2) == 8);
    assert_utf8_loc!((3, 3) == 8);
    assert_utf8_loc!((3, 4) == 12);
    assert_utf8_loc!((3, 5) == 12);
    assert_utf8_loc!((3, 6) == 12);
    assert_utf8_loc!((3, 7) == 12);
    assert_utf8_loc!((3, 8) == 16);
    assert_utf8_loc!((3, 9) == 16);
    assert_utf8_loc!((3, 10) == 16);
    assert_utf8_loc!((3, 11) == 16);
    assert_utf8_loc!((3, 12) == 20);
    assert_utf8_loc!((3, 13) == 21);
    assert_utf8_loc!((4, 0) == 22);
    assert_utf8_loc!((4, 1) == 23);
    assert_utf8_loc!((4, 2) == 24);
    assert_utf8_loc!((4, 3) == 25);
    assert_utf8_loc!((4, 4) == 25);
    assert_utf8_loc!((4, 5) == 25);
    assert_utf8_loc!((4, 6) == 25);
    assert_utf8_loc!((4, 7) == 29);
    assert_utf8_loc!((4, 8) == 30);

    // Out of bounds line index
    assert_utf8_loc!((5, 0) == 30);

    // Out of bounds column index
    assert_utf8_loc!((0, 10) == 2);

    macro_rules! assert_utf16_loc {
      (($line:literal, $col:literal) == $byte:literal) => {
        assert_eq!(
          info.utf16_loc(super::LineColUtf16 {
            line: $line,
            col: $col
          }),
          super::Location($byte),
          "loc {}:{}",
          $line,
          $col
        );
      };
    }

    assert_utf16_loc!((0, 0) == 0);
    assert_utf16_loc!((0, 1) == 1);
    assert_utf16_loc!((1, 0) == 2);
    assert_utf16_loc!((1, 1) == 3);
    assert_utf16_loc!((1, 2) == 4);
    assert_utf16_loc!((1, 3) == 5);
    assert_utf16_loc!((2, 0) == 6);
    assert_utf16_loc!((2, 1) == 7);
    assert_utf16_loc!((3, 0) == 8);
    assert_utf16_loc!((3, 1) == 8);
    assert_utf16_loc!((3, 2) == 12);
    assert_utf16_loc!((3, 3) == 12);
    assert_utf16_loc!((3, 4) == 16);
    assert_utf16_loc!((3, 5) == 16);
    assert_utf16_loc!((3, 6) == 20);
    assert_utf16_loc!((3, 7) == 21);
    assert_utf16_loc!((4, 0) == 22);
    assert_utf16_loc!((4, 1) == 23);
    assert_utf16_loc!((4, 2) == 24);
    assert_utf16_loc!((4, 3) == 25);
    assert_utf16_loc!((4, 4) == 25);
    assert_utf16_loc!((4, 5) == 29);
    assert_utf16_loc!((4, 6) == 30);

    // Out of bounds line index
    assert_utf16_loc!((5, 0) == 30);

    // Out of bounds column index
    assert_utf16_loc!((0, 10) == 2);
  }

  #[test]
  fn source_text_line_col_reset() {
    let source = "a\rb";
    let mut source_text = super::SourceTextIterator::new(source);
    assert_eq!(source_text.next(), Some((super::Location(0), 'a')));
    assert_eq!(source_text.next(), Some((super::Location(1), '\r')));
    source_text.reset_to(super::Location(2)); // doesn't change anything, but \r tracking must be set correctly now
    assert_eq!(source_text.next(), Some((super::Location(2), 'b')));
    assert_eq!(source_text.next(), None);
    let info = source_text.into_info();
    assert_eq!(info.utf8_line_starts, vec![0, 2]);
  }

  #[test]
  fn source_text_span_len() {
    let source = "a\nbc\r\nf\r🍊😅🎃\r\nasd🍊a";
    let mut source_text = super::SourceTextIterator::new(source);
    while source_text.next().is_some() {}

    let info = source_text.into_info();
    assert_eq!(
      info.utf8_len(super::Span::new(super::Location(0)..super::Location(0))),
      0
    );
    assert_eq!(
      info.utf8_len(super::Span::new(super::Location(0)..super::Location(1))),
      1
    );
    assert_eq!(
      info.utf8_len(super::Span::new(super::Location(0)..super::Location(2))),
      2
    );
    assert_eq!(
      info.utf8_len(super::Span::new(super::Location(8)..super::Location(12))),
      4
    );

    assert_eq!(
      info.utf16_len(super::Span::new(super::Location(0)..super::Location(0))),
      0
    );
    assert_eq!(
      info.utf16_len(super::Span::new(super::Location(0)..super::Location(1))),
      1
    );
    assert_eq!(
      info.utf16_len(super::Span::new(super::Location(0)..super::Location(2))),
      2
    );
    assert_eq!(
      info.utf16_len(super::Span::new(super::Location(8)..super::Location(12))),
      2
    );
  }
}
