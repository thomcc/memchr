pub use self::{
    iter::{Memmem, Memrmem},
    prefilter::PrefilterState,
    twoway::TwoWay,
};

mod byte_frequencies;
mod iter;
mod prefilter;
mod twoway;

/// Returns an iterator over all occurrences of a substring in a haystack.
#[inline]
pub fn memmem_iter<'h, 'n>(
    haystack: &'h [u8],
    needle: &'n [u8],
) -> Memmem<'h, 'n> {
    Memmem::new(haystack, needle)
}

/// Returns a reverse iterator over all occurrences of a substring in a
/// haystack.
#[inline]
pub fn memrmem_iter<'h, 'n>(
    haystack: &'h [u8],
    needle: &'n [u8],
) -> Memrmem<'h, 'n> {
    Memrmem::new(haystack, needle)
}

/// Returns the index of the first occurrence of the given needle.
///
/// Note that if you're are searching for the same needle in many different
/// small haystacks, it may be faster to initialize a [`MemmemFinder`] once,
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
/// use memchr::memmem;
///
/// let haystack = b"foo bar baz";
/// assert_eq!(Some(0), memmem(haystack, b"foo"));
/// assert_eq!(Some(4), memmem(haystack, b"bar"));
/// assert_eq!(None, memmem(haystack, b"quux"));
/// ```
#[inline]
pub fn memmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    crate::memmem::TwoWay::forward(needle).find(haystack)
}

/// Returns the index of the last occurrence of the given needle.
///
/// Note that if you're are searching for the same needle in many different
/// small haystacks, it may be faster to initialize a [`MemrmemFinder`] once,
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
/// use memchr::memrmem;
///
/// let haystack = b"foo bar baz";
/// assert_eq!(Some(0), memrmem(haystack, b"foo"));
/// assert_eq!(Some(4), memrmem(haystack, b"bar"));
/// assert_eq!(Some(8), memrmem(haystack, b"ba"));
/// assert_eq!(None, memrmem(haystack, b"quux"));
/// ```
#[inline]
pub fn memrmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    crate::memmem::TwoWay::reverse(needle).rfind(haystack)
}

/// A single substring searcher fixed to a particular needle.
///
/// The purpose of this type is to permit callers to construct a substring
/// searcher that can be used to search haystacks without the overhead of
/// constructing the searcher in the first place. This is a somewhat niche
/// concern when it's necessary to re-use the same needle to search multiple
/// different haystacks with as little overhead as possible. In general, using
/// [`memmem`] is good enough, but `MemmemFinder` is useful when you can
/// meaningfully observe searcher construction time in a profile.
///
/// When the `std` feature is enabled, then this type has an `into_owned`
/// version which permits building a `MemmemFinder` that is not connected to
/// the lifetime of its needle.
#[derive(Clone, Debug)]
pub struct MemmemFinder<'a> {
    searcher: TwoWay<'a>,
}

impl<'a> MemmemFinder<'a> {
    /// Create a new finder for the given needle.
    #[inline]
    pub fn new<B: ?Sized + AsRef<[u8]>>(needle: &'a B) -> MemmemFinder<'a> {
        MemmemFinder { searcher: TwoWay::forward(needle.as_ref()) }
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
    pub fn into_owned(self) -> MemmemFinder<'static> {
        MemmemFinder { searcher: self.searcher.into_owned() }
    }

    /// Returns the needle that this finder searches for.
    ///
    /// Note that the lifetime of the needle returned is tied to the lifetime
    /// of the finder, and may be shorter than the `'a` lifetime. Namely, a
    /// finder's needle can be either borrowed or owned, so the lifetime of the
    /// needle returned must necessarily be the shorter of the two.
    #[inline]
    pub fn needle(&self) -> &[u8] {
        self.searcher.needle()
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
    /// use memchr::MemmemFinder;
    ///
    /// let haystack = b"foo bar baz";
    /// assert_eq!(Some(0), MemmemFinder::new("foo").find(haystack));
    /// assert_eq!(Some(4), MemmemFinder::new("bar").find(haystack));
    /// assert_eq!(None, MemmemFinder::new("quux").find(haystack));
    /// ```
    #[inline]
    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
        self.searcher.find(haystack)
    }
}

/// A single substring reverse searcher fixed to a particular needle.
///
/// The purpose of this type is to permit callers to construct a substring
/// searcher that can be used to search haystacks without the overhead of
/// constructing the searcher in the first place. This is a somewhat niche
/// concern when it's necessary to re-use the same needle to search multiple
/// different haystacks with as little overhead as possible. In general, using
/// [`memrmem`] is good enough, but `MemrmemFinder` is useful when you can
/// meaningfully observe searcher construction time in a profile.
///
/// When the `std` feature is enabled, then this type has an `into_owned`
/// version which permits building a `MemrmemFinder` that is not connected to
/// the lifetime of its needle.
#[derive(Clone, Debug)]
pub struct MemrmemFinder<'a> {
    searcher: TwoWay<'a>,
}

impl<'a> MemrmemFinder<'a> {
    /// Create a new reverse finder for the given needle.
    #[inline]
    pub fn new<B: ?Sized + AsRef<[u8]>>(needle: &'a B) -> MemrmemFinder<'a> {
        MemrmemFinder { searcher: TwoWay::reverse(needle.as_ref()) }
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
    pub fn into_owned(self) -> MemrmemFinder<'static> {
        MemrmemFinder { searcher: self.searcher.into_owned() }
    }

    /// Returns the needle that this finder searches for.
    ///
    /// Note that the lifetime of the needle returned is tied to the lifetime
    /// of this finder, and may be shorter than the `'a` lifetime. Namely,
    /// a finder's needle can be either borrowed or owned, so the lifetime of
    /// the needle returned must necessarily be the shorter of the two.
    #[inline]
    pub fn needle(&self) -> &[u8] {
        self.searcher.needle()
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
    /// use memchr::MemrmemFinder;
    ///
    /// let haystack = b"foo bar baz";
    /// assert_eq!(Some(0), MemrmemFinder::new("foo").rfind(haystack));
    /// assert_eq!(Some(4), MemrmemFinder::new("bar").rfind(haystack));
    /// assert_eq!(None, MemrmemFinder::new("quux").rfind(haystack));
    /// ```
    #[inline]
    pub fn rfind<B: AsRef<[u8]>>(&self, haystack: B) -> Option<usize> {
        self.searcher.rfind(haystack.as_ref())
    }
}
