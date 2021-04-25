/*!
TODO
*/

pub use self::prefilter::Prefilter;
use self::prefilter::PrefilterState;

/// Defines a suite of quickcheck properties for forward and reverse
/// substring searching.
///
/// This is defined in this specific spot so that it can be used freely among
/// the different substring search implementations. I couldn't be bothered to
/// fight with the macro-visibility rules enough to figure out how to stuff it
/// somewhere more convenient.
#[cfg(all(test, feature = "std"))]
macro_rules! define_memmem_quickcheck_tests {
    ($fwd:expr, $rev:expr) => {
        use crate::memmem::testprops;

        quickcheck::quickcheck! {
            fn qc_fwd_prefix_is_substring(bs: Vec<u8>) -> bool {
                testprops::prefix_is_substring(false, &bs, $fwd)
            }

            fn qc_fwd_suffix_is_substring(bs: Vec<u8>) -> bool {
                testprops::suffix_is_substring(false, &bs, $fwd)
            }

            fn qc_fwd_matches_naive(
                haystack: Vec<u8>,
                needle: Vec<u8>
            ) -> bool {
                testprops::matches_naive(false, &haystack, &needle, $fwd)
            }

            fn qc_rev_prefix_is_substring(bs: Vec<u8>) -> bool {
                testprops::prefix_is_substring(true, &bs, $rev)
            }

            fn qc_rev_suffix_is_substring(bs: Vec<u8>) -> bool {
                testprops::suffix_is_substring(true, &bs, $rev)
            }

            fn qc_rev_matches_naive(
                haystack: Vec<u8>,
                needle: Vec<u8>
            ) -> bool {
                testprops::matches_naive(true, &haystack, &needle, $rev)
            }
        }
    };
}

/// Defines a suite of "simple" hand-written tests for a substring
/// implementation.
///
/// This is defined here for the same reason that
/// define_memmem_quickcheck_tests is defined here.
#[cfg(test)]
macro_rules! define_memmem_simple_tests {
    ($fwd:expr, $rev:expr) => {
        use crate::memmem::testsimples;

        #[test]
        fn simple_forward() {
            testsimples::run_search_tests_fwd($fwd);
        }

        #[test]
        fn simple_reverse() {
            testsimples::run_search_tests_rev($rev);
        }
    };
}

mod byte_frequencies;
mod prefilter;
mod rabinkarp;
mod twoway;
mod util;
// SIMD is only supported on x86_64 currently.
#[cfg(target_arch = "x86_64")]
mod vector;

/// Returns an iterator over all occurrences of a substring in a haystack.
///
/// # Complexity
///
/// This routine is guaranteed to have worst case linear time complexity
/// with respect to both the needle and the haystack. That is, this runs
/// in `O(needle.len() + haystack.len())` time.
///
/// This routine is also guaranteed to have worst case constant space
/// complexity.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use memchr::memmem::memmem_iter;
///
/// let haystack = b"foo bar foo baz foo";
/// let mut it = memmem_iter(haystack, b"foo");
/// assert_eq!(Some(0), it.next());
/// assert_eq!(Some(8), it.next());
/// assert_eq!(Some(16), it.next());
/// assert_eq!(None, it.next());
/// ```
#[inline]
pub fn memmem_iter<'h, 'n>(
    haystack: &'h [u8],
    needle: &'n [u8],
) -> Memmem<'h, 'n> {
    Memmem::new(haystack, Finder::new(needle))
}

/// Returns a reverse iterator over all occurrences of a substring in a
/// haystack.
///
/// # Complexity
///
/// This routine is guaranteed to have worst case linear time complexity
/// with respect to both the needle and the haystack. That is, this runs
/// in `O(needle.len() + haystack.len())` time.
///
/// This routine is also guaranteed to have worst case constant space
/// complexity.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use memchr::memmem::memrmem_iter;
///
/// let haystack = b"foo bar foo baz foo";
/// let mut it = memrmem_iter(haystack, b"foo");
/// assert_eq!(Some(16), it.next());
/// assert_eq!(Some(8), it.next());
/// assert_eq!(Some(0), it.next());
/// assert_eq!(None, it.next());
/// ```
#[inline]
pub fn memrmem_iter<'h, 'n>(
    haystack: &'h [u8],
    needle: &'n [u8],
) -> Memrmem<'h, 'n> {
    Memrmem::new(haystack, FinderRev::new(needle))
}

/// Returns the index of the first occurrence of the given needle.
///
/// Note that if you're are searching for the same needle in many different
/// small haystacks, it may be faster to initialize a [`Finder`] once,
/// and reuse it for each search.
///
/// # Complexity
///
/// This routine is guaranteed to have worst case linear time complexity
/// with respect to both the needle and the haystack. That is, this runs
/// in `O(needle.len() + haystack.len())` time.
///
/// This routine is also guaranteed to have worst case constant space
/// complexity.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use memchr::memmem::memmem;
///
/// let haystack = b"foo bar baz";
/// assert_eq!(Some(0), memmem(haystack, b"foo"));
/// assert_eq!(Some(4), memmem(haystack, b"bar"));
/// assert_eq!(None, memmem(haystack, b"quux"));
/// ```
#[inline]
pub fn memmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if haystack.len() < 64 {
        rabinkarp::find(haystack, needle)
    } else {
        Finder::new(needle).find(haystack)
    }
}

/// Returns the index of the last occurrence of the given needle.
///
/// Note that if you're are searching for the same needle in many different
/// small haystacks, it may be faster to initialize a [`FinderRev`] once,
/// and reuse it for each search.
///
/// # Complexity
///
/// This routine is guaranteed to have worst case linear time complexity
/// with respect to both the needle and the haystack. That is, this runs
/// in `O(needle.len() + haystack.len())` time.
///
/// This routine is also guaranteed to have worst case constant space
/// complexity.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use memchr::memmem::memrmem;
///
/// let haystack = b"foo bar baz";
/// assert_eq!(Some(0), memrmem(haystack, b"foo"));
/// assert_eq!(Some(4), memrmem(haystack, b"bar"));
/// assert_eq!(Some(8), memrmem(haystack, b"ba"));
/// assert_eq!(None, memrmem(haystack, b"quux"));
/// ```
#[inline]
pub fn memrmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if haystack.len() < 64 {
        rabinkarp::rfind(haystack, needle)
    } else {
        FinderRev::new(needle).rfind(haystack)
    }
}

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
    finder: Finder<'n>,
    pos: usize,
}

impl<'h, 'n> Memmem<'h, 'n> {
    #[inline(always)]
    pub(crate) fn new(
        haystack: &'h [u8],
        finder: Finder<'n>,
    ) -> Memmem<'h, 'n> {
        let prestate = finder.searcher.prefilter_state();
        Memmem { haystack, prestate, finder, pos: 0 }
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
            .finder
            .find_with(&mut self.prestate, &self.haystack[self.pos..]);
        match result {
            None => None,
            Some(i) => {
                let pos = self.pos + i;
                self.pos = pos + core::cmp::max(1, self.finder.needle().len());
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
    finder: FinderRev<'n>,
    /// When searching with an empty needle, this gets set to `None` after
    /// we've yielded the last element at `0`.
    pos: Option<usize>,
}

impl<'h, 'n> Memrmem<'h, 'n> {
    #[inline(always)]
    pub(crate) fn new(
        haystack: &'h [u8],
        finder: FinderRev<'n>,
    ) -> Memrmem<'h, 'n> {
        let pos = Some(haystack.len());
        Memrmem { haystack, finder, pos }
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
        let result = self.finder.rfind(&self.haystack[..pos]);
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

/// A single substring searcher fixed to a particular needle.
///
/// The purpose of this type is to permit callers to construct a substring
/// searcher that can be used to search haystacks without the overhead of
/// constructing the searcher in the first place. This is a somewhat niche
/// concern when it's necessary to re-use the same needle to search multiple
/// different haystacks with as little overhead as possible. In general, using
/// [`memmem`] is good enough, but `Finder` is useful when you can
/// meaningfully observe searcher construction time in a profile.
///
/// When the `std` feature is enabled, then this type has an `into_owned`
/// version which permits building a `Finder` that is not connected to
/// the lifetime of its needle.
#[derive(Clone, Debug)]
pub struct Finder<'n> {
    searcher: twoway::Forward<'n>,
}

impl<'n> Finder<'n> {
    /// Create a new finder for the given needle.
    #[inline]
    pub fn new<B: ?Sized + AsRef<[u8]>>(needle: &'n B) -> Finder<'n> {
        FinderBuilder::new().build_forward(needle)
    }

    /// Returns the index of the first occurrence of this needle in the given
    /// haystack.
    ///
    /// # Complexity
    ///
    /// This routine is guaranteed to have worst case linear time complexity
    /// with respect to both the needle and the haystack. That is, this runs
    /// in `O(needle.len() + haystack.len())` time.
    ///
    /// This routine is also guaranteed to have worst case constant space
    /// complexity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use memchr::memmem::Finder;
    ///
    /// let haystack = b"foo bar baz";
    /// assert_eq!(Some(0), Finder::new("foo").find(haystack));
    /// assert_eq!(Some(4), Finder::new("bar").find(haystack));
    /// assert_eq!(None, Finder::new("quux").find(haystack));
    /// ```
    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
        self.searcher.find(haystack)
    }

    /// Returns an iterator over all occurrences of a substring in a haystack.
    ///
    /// # Complexity
    ///
    /// This routine is guaranteed to have worst case linear time complexity
    /// with respect to both the needle and the haystack. That is, this runs
    /// in `O(needle.len() + haystack.len())` time.
    ///
    /// This routine is also guaranteed to have worst case constant space
    /// complexity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use memchr::memmem::Finder;
    ///
    /// let haystack = b"foo bar foo baz foo";
    /// let finder = Finder::new(b"foo");
    /// let mut it = finder.find_iter(haystack);
    /// assert_eq!(Some(0), it.next());
    /// assert_eq!(Some(8), it.next());
    /// assert_eq!(Some(16), it.next());
    /// assert_eq!(None, it.next());
    /// ```
    #[inline]
    pub fn find_iter<'a, 'h>(&'a self, haystack: &'h [u8]) -> Memmem<'h, 'a> {
        Memmem::new(haystack, self.as_ref())
    }

    /// Convert this finder into its owned variant, such that it no longer
    /// borrows the needle.
    ///
    /// If this is already an owned finder, then this is a no-op. Otherwise,
    /// this copies the needle.
    ///
    /// This is only available when the `std` feature is enabled.
    #[cfg(feature = "std")]
    #[inline]
    pub fn into_owned(self) -> Finder<'static> {
        Finder { searcher: self.searcher.into_owned() }
    }

    /// Convert this finder into its borrowed variant.
    ///
    /// This is primarily useful if your finder is owned and you'd like to
    /// store its borrowed variant in some intermediate data structure.
    ///
    /// Note that the lifetime parameter of the returned finder is tied to the
    /// lifetime of `self`, and may be shorter than the `'n` lifetime of the
    /// needle itself. Namely, a finder's needle can be either borrowed or
    /// owned, so the lifetime of the needle returned must necessarily be the
    /// shorter of the two.
    #[inline]
    pub fn as_ref(&self) -> Finder<'_> {
        Finder { searcher: self.searcher.as_ref() }
    }

    /// Returns the needle that this finder searches for.
    ///
    /// Note that the lifetime of the needle returned is tied to the lifetime
    /// of the finder, and may be shorter than the `'n` lifetime. Namely, a
    /// finder's needle can be either borrowed or owned, so the lifetime of the
    /// needle returned must necessarily be the shorter of the two.
    #[inline]
    pub fn needle(&self) -> &[u8] {
        self.searcher.needle()
    }

    #[inline(always)]
    fn find_with(
        &self,
        prestate: &mut PrefilterState,
        haystack: &[u8],
    ) -> Option<usize> {
        self.searcher.find_with(prestate, haystack)
    }
}

/// A single substring reverse searcher fixed to a particular needle.
///
/// The purpose of this type is to permit callers to construct a substring
/// searcher that can be used to search haystacks without the overhead of
/// constructing the searcher in the first place. This is a somewhat niche
/// concern when it's necessary to re-use the same needle to search multiple
/// different haystacks with as little overhead as possible. In general, using
/// [`memrmem`] is good enough, but `FinderRev` is useful when you can
/// meaningfully observe searcher construction time in a profile.
///
/// When the `std` feature is enabled, then this type has an `into_owned`
/// version which permits building a `FinderRev` that is not connected to
/// the lifetime of its needle.
#[derive(Clone, Debug)]
pub struct FinderRev<'n> {
    searcher: twoway::Reverse<'n>,
}

impl<'n> FinderRev<'n> {
    /// Create a new reverse finder for the given needle.
    #[inline]
    pub fn new<B: ?Sized + AsRef<[u8]>>(needle: &'n B) -> FinderRev<'n> {
        FinderBuilder::new().build_reverse(needle)
    }

    /// Returns the index of the last occurrence of this needle in the given
    /// haystack.
    ///
    /// The haystack may be any type that can be cheaply converted into a
    /// `&[u8]`. This includes, but is not limited to, `&str` and `&[u8]`.
    ///
    /// # Complexity
    ///
    /// This routine is guaranteed to have worst case linear time complexity
    /// with respect to both the needle and the haystack. That is, this runs
    /// in `O(needle.len() + haystack.len())` time.
    ///
    /// This routine is also guaranteed to have worst case constant space
    /// complexity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use memchr::memmem::FinderRev;
    ///
    /// let haystack = b"foo bar baz";
    /// assert_eq!(Some(0), FinderRev::new("foo").rfind(haystack));
    /// assert_eq!(Some(4), FinderRev::new("bar").rfind(haystack));
    /// assert_eq!(None, FinderRev::new("quux").rfind(haystack));
    /// ```
    #[inline]
    pub fn rfind<B: AsRef<[u8]>>(&self, haystack: B) -> Option<usize> {
        self.searcher.rfind(haystack.as_ref())
    }

    /// Returns a reverse iterator over all occurrences of a substring in a
    /// haystack.
    ///
    /// # Complexity
    ///
    /// This routine is guaranteed to have worst case linear time complexity
    /// with respect to both the needle and the haystack. That is, this runs
    /// in `O(needle.len() + haystack.len())` time.
    ///
    /// This routine is also guaranteed to have worst case constant space
    /// complexity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use memchr::memmem::FinderRev;
    ///
    /// let haystack = b"foo bar foo baz foo";
    /// let finder = FinderRev::new(b"foo");
    /// let mut it = finder.rfind_iter(haystack);
    /// assert_eq!(Some(16), it.next());
    /// assert_eq!(Some(8), it.next());
    /// assert_eq!(Some(0), it.next());
    /// assert_eq!(None, it.next());
    /// ```
    #[inline]
    pub fn rfind_iter<'a, 'h>(
        &'a self,
        haystack: &'h [u8],
    ) -> Memrmem<'h, 'a> {
        Memrmem::new(haystack, self.as_ref())
    }

    /// Convert this finder into its owned variant, such that it no longer
    /// borrows the needle.
    ///
    /// If this is already an owned finder, then this is a no-op. Otherwise,
    /// this copies the needle.
    ///
    /// This is only available when the `std` feature is enabled.
    #[cfg(feature = "std")]
    #[inline]
    pub fn into_owned(self) -> FinderRev<'static> {
        FinderRev { searcher: self.searcher.into_owned() }
    }

    /// Convert this finder into its borrowed variant.
    ///
    /// This is primarily useful if your finder is owned and you'd like to
    /// store its borrowed variant in some intermediate data structure.
    ///
    /// Note that the lifetime parameter of the returned finder is tied to the
    /// lifetime of `self`, and may be shorter than the `'n` lifetime of the
    /// needle itself. Namely, a finder's needle can be either borrowed or
    /// owned, so the lifetime of the needle returned must necessarily be the
    /// shorter of the two.
    #[inline]
    pub fn as_ref(&self) -> FinderRev<'_> {
        FinderRev { searcher: self.searcher.as_ref() }
    }

    /// Returns the needle that this finder searches for.
    ///
    /// Note that the lifetime of the needle returned is tied to the lifetime
    /// of the finder, and may be shorter than the `'n` lifetime. Namely, a
    /// finder's needle can be either borrowed or owned, so the lifetime of the
    /// needle returned must necessarily be the shorter of the two.
    #[inline]
    pub fn needle(&self) -> &[u8] {
        self.searcher.needle()
    }
}

/// A builder for constructing non-default forward or reverse memmem finders.
#[derive(Clone, Debug, Default)]
pub struct FinderBuilder {
    config: twoway::Config,
}

impl FinderBuilder {
    /// Create a new finder builder with default settings.
    pub fn new() -> FinderBuilder {
        FinderBuilder::default()
    }

    /// Build a forward finder using the given needle from the current
    /// settings.
    pub fn build_forward<'n, B: ?Sized + AsRef<[u8]>>(
        &self,
        needle: &'n B,
    ) -> Finder<'n> {
        Finder { searcher: twoway::Forward::new(self.config, needle.as_ref()) }
    }

    /// Build a reverse finder using the given needle from the current
    /// settings.
    pub fn build_reverse<'n, B: ?Sized + AsRef<[u8]>>(
        &self,
        needle: &'n B,
    ) -> FinderRev<'n> {
        FinderRev { searcher: twoway::Reverse::new(needle.as_ref()) }
    }

    /// Configure the prefilter setting for the finder.
    pub fn prefilter(&mut self, prefilter: Prefilter) -> &mut FinderBuilder {
        self.config.prefilter = prefilter;
        self
    }
}

/// This module defines some generic quickcheck properties useful for testing
/// any substring search algorithm. It also runs those properties for the
/// top-level public API memmem routines. (The properties are also used to
/// test various substring search implementations more granularly elsewhere as
/// well.)
#[cfg(all(test, feature = "std", not(miri)))]
mod testprops {
    // N.B. This defines the quickcheck tests using the properties defined
    // below. Because of macro-visibility weirdness, the actual macro is
    // defined at the top of this file.
    define_memmem_quickcheck_tests!(super::memmem, super::memrmem);

    /// Check that every prefix of the given byte string is a substring.
    pub(crate) fn prefix_is_substring(
        reverse: bool,
        bs: &[u8],
        mut search: impl FnMut(&[u8], &[u8]) -> Option<usize>,
    ) -> bool {
        if bs.is_empty() {
            return true;
        }
        for i in 0..(bs.len() - 1) {
            let prefix = &bs[..i];
            if reverse {
                assert_eq!(naive_rfind(bs, prefix), search(bs, prefix));
            } else {
                assert_eq!(naive_find(bs, prefix), search(bs, prefix));
            }
        }
        true
    }

    /// Check that every suffix of the given byte string is a substring.
    pub(crate) fn suffix_is_substring(
        reverse: bool,
        bs: &[u8],
        mut search: impl FnMut(&[u8], &[u8]) -> Option<usize>,
    ) -> bool {
        if bs.is_empty() {
            return true;
        }
        for i in 0..(bs.len() - 1) {
            let suffix = &bs[i..];
            if reverse {
                assert_eq!(naive_rfind(bs, suffix), search(bs, suffix));
            } else {
                assert_eq!(naive_find(bs, suffix), search(bs, suffix));
            }
        }
        true
    }

    /// Check that naive substring search matches the result of the given search
    /// algorithm.
    pub(crate) fn matches_naive(
        reverse: bool,
        haystack: &[u8],
        needle: &[u8],
        mut search: impl FnMut(&[u8], &[u8]) -> Option<usize>,
    ) -> bool {
        if reverse {
            naive_rfind(haystack, needle) == search(haystack, needle)
        } else {
            naive_find(haystack, needle) == search(haystack, needle)
        }
    }

    /// Naively search forwards for the given needle in the given haystack.
    fn naive_find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        } else if haystack.len() < needle.len() {
            return None;
        }
        for i in 0..(haystack.len() - needle.len() + 1) {
            if needle == &haystack[i..i + needle.len()] {
                return Some(i);
            }
        }
        None
    }

    /// Naively search in reverse for the given needle in the given haystack.
    fn naive_rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(haystack.len());
        } else if haystack.len() < needle.len() {
            return None;
        }
        for i in (0..(haystack.len() - needle.len() + 1)).rev() {
            if needle == &haystack[i..i + needle.len()] {
                return Some(i);
            }
        }
        None
    }
}

/// This module defines a bunch of hand-written "simple" substring tests. It
/// also provides routines for easily running them on any substring search
/// implementation.
#[cfg(test)]
mod testsimples {
    define_memmem_simple_tests!(super::memmem, super::memrmem);

    /// Each test is a (needle, haystack, expected_fwd, expected_rev) tuple.
    type SearchTest =
        (&'static str, &'static str, Option<usize>, Option<usize>);

    const SEARCH_TESTS: &'static [SearchTest] = &[
        ("", "", Some(0), Some(0)),
        ("", "a", Some(0), Some(1)),
        ("", "ab", Some(0), Some(2)),
        ("", "abc", Some(0), Some(3)),
        ("a", "", None, None),
        ("a", "a", Some(0), Some(0)),
        ("a", "aa", Some(0), Some(1)),
        ("a", "ba", Some(1), Some(1)),
        ("a", "bba", Some(2), Some(2)),
        ("a", "bbba", Some(3), Some(3)),
        ("a", "bbbab", Some(3), Some(3)),
        ("a", "bbbabb", Some(3), Some(3)),
        ("a", "bbbabbb", Some(3), Some(3)),
        ("a", "bbbbbb", None, None),
        ("ab", "", None, None),
        ("ab", "a", None, None),
        ("ab", "b", None, None),
        ("ab", "ab", Some(0), Some(0)),
        ("ab", "aab", Some(1), Some(1)),
        ("ab", "aaab", Some(2), Some(2)),
        ("ab", "abaab", Some(0), Some(3)),
        ("ab", "baaab", Some(3), Some(3)),
        ("ab", "acb", None, None),
        ("ab", "abba", Some(0), Some(0)),
        ("abc", "ab", None, None),
        ("abc", "abc", Some(0), Some(0)),
        ("abc", "abcz", Some(0), Some(0)),
        ("abc", "abczz", Some(0), Some(0)),
        ("abc", "zabc", Some(1), Some(1)),
        ("abc", "zzabc", Some(2), Some(2)),
        ("abc", "azbc", None, None),
        ("abc", "abzc", None, None),
        ("abczdef", "abczdefzzzzzzzzzzzzzzzzzzzz", Some(0), Some(0)),
        ("abczdef", "zzzzzzzzzzzzzzzzzzzzabczdef", Some(20), Some(20)),
        ("xyz", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaxyz", Some(32), Some(32)),
        // Failures caught by quickcheck.
        ("\u{0}\u{15}", "\u{0}\u{15}\u{15}\u{0}", Some(0), Some(0)),
        ("\u{0}\u{1e}", "\u{1e}\u{0}", None, None),
    ];

    /// Run the substring search tests. `search` should be a closure that
    /// accepts a haystack and a needle and returns the starting position
    /// of the first occurrence of needle in the haystack, or `None` if one
    /// doesn't exist.
    pub(crate) fn run_search_tests_fwd(
        mut search: impl FnMut(&[u8], &[u8]) -> Option<usize>,
    ) {
        for &(needle, haystack, expected_fwd, _) in SEARCH_TESTS {
            let (n, h) = (needle.as_bytes(), haystack.as_bytes());
            assert_eq!(
                expected_fwd,
                search(h, n),
                "needle: {:?}, haystack: {:?}, expected: {:?}",
                n,
                h,
                expected_fwd
            );
        }
    }

    /// Run the substring search tests. `search` should be a closure that
    /// accepts a haystack and a needle and returns the starting position of
    /// the last occurrence of needle in the haystack, or `None` if one doesn't
    /// exist.
    pub(crate) fn run_search_tests_rev(
        mut search: impl FnMut(&[u8], &[u8]) -> Option<usize>,
    ) {
        for &(needle, haystack, _, expected_rev) in SEARCH_TESTS {
            let (n, h) = (needle.as_bytes(), haystack.as_bytes());
            assert_eq!(
                expected_rev,
                search(h, n),
                "needle: {:?}, haystack: {:?}, expected: {:?}",
                n,
                h,
                expected_rev
            );
        }
    }
}
