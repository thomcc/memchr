use core::cmp;

use super::{PrefilterState, TwoWay};

/// An iterator over non-overlapping substring matches.
///
/// Matches are reported by the byte offset at which they begin.
///
/// `'h` is the lifetime of the haystack while `'n` is the lifetime of the
/// needle.
#[derive(Debug)]
pub struct Memmem<'h, 'n> {
    haystack: &'h [u8],
    prestate: PrefilterState,
    searcher: TwoWay<'n>,
    pos: usize,
}

impl<'h, 'n> Memmem<'h, 'n> {
    pub(crate) fn new(haystack: &'h [u8], needle: &'n [u8]) -> Memmem<'h, 'n> {
        let searcher = TwoWay::forward(needle);
        let prestate = searcher.prefilter_state();
        Memmem { haystack, prestate, searcher, pos: 0 }
    }
}

impl<'h, 'n> Iterator for Memmem<'h, 'n> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        if self.pos > self.haystack.len() {
            return None;
        }
        let result = self
            .searcher
            .find_with(&mut self.prestate, &self.haystack[self.pos..]);
        match result {
            None => None,
            Some(i) => {
                let pos = self.pos + i;
                self.pos = pos + cmp::max(1, self.searcher.needle().len());
                Some(pos)
            }
        }
    }
}

/// An iterator over non-overlapping substring matches in reverse.
///
/// Matches are reported by the byte offset at which they begin.
///
/// `'h` is the lifetime of the haystack while `'n` is the lifetime of the
/// needle.
#[derive(Debug)]
pub struct Memrmem<'h, 'n> {
    haystack: &'h [u8],
    prestate: PrefilterState,
    searcher: TwoWay<'n>,
    /// When searching with an empty needle, this gets set to `None` after
    /// we've yielded the last element at `0`.
    pos: Option<usize>,
}

impl<'h, 'n> Memrmem<'h, 'n> {
    pub(crate) fn new(
        haystack: &'h [u8],
        needle: &'n [u8],
    ) -> Memrmem<'h, 'n> {
        let searcher = TwoWay::reverse(needle);
        let prestate = searcher.prefilter_state();
        let pos = Some(haystack.len());
        Memrmem { haystack, prestate, searcher, pos }
    }
}

impl<'h, 'n> Iterator for Memrmem<'h, 'n> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        let pos = match self.pos {
            None => return None,
            Some(pos) => pos,
        };
        let result = self
            .searcher
            .rfind_with(&mut self.prestate, &self.haystack[..pos]);
        match result {
            None => None,
            Some(i) => {
                if pos == i {
                    self.pos = pos.checked_sub(1);
                } else {
                    self.pos = Some(i);
                }
                Some(i)
            }
        }
    }
}
