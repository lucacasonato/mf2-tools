use std::str::Chars;

pub struct ResettablePeekableCharIndices<'a> {
  pub(super) original: &'a str,
  pub(super) front_offset: usize,
  pub(super) iter: Chars<'a>,
  pub(super) peeked: Option<Option<(usize, char)>>,
}

impl<'a> Iterator for ResettablePeekableCharIndices<'a> {
  type Item = (usize, char);

  #[inline]
  fn next(&mut self) -> Option<(usize, char)> {
    if let Some(peeked) = self.peeked.take() {
      if let Some((_, ch)) = peeked {
        self.front_offset += ch.len_utf8();
      }
      return peeked;
    }
    match self.iter.next() {
      None => None,
      Some(ch) => {
        let index = self.front_offset;
        self.front_offset += ch.len_utf8();
        Some((index, ch))
      }
    }
  }

  #[inline]
  fn count(self) -> usize {
    self.iter.count()
  }

  #[inline]
  fn size_hint(&self) -> (usize, Option<usize>) {
    self.iter.size_hint()
  }
}

impl ResettablePeekableCharIndices<'_> {
  pub fn new(s: &str) -> ResettablePeekableCharIndices {
    ResettablePeekableCharIndices {
      original: s,
      front_offset: 0,
      iter: s.chars(),
      peeked: None,
    }
  }

  /// Resets the iterator to the given index.
  ///
  /// # Panics
  ///
  /// Panics if the index is greater than the length of the original string, or
  /// if the index is not at a character boundary.
  pub fn reset_to(&mut self, index: usize) {
    assert!(index <= self.original.len());
    self.front_offset = index;
    self.peeked = None;
    self.iter = self.original[self.front_offset..].chars();
  }

  pub fn peek(&mut self) -> Option<(usize, char)> {
    match &self.peeked {
      Some(peeked) => peeked.clone(),
      None => {
        let peeked = self.iter.next().map(|ch| (self.front_offset, ch));
        self.peeked = Some(peeked.clone());
        peeked
      }
    }
  }

  pub fn current_byte_index(&self) -> usize {
    self.front_offset
  }
}
