#![allow(warnings)]

use core::mem;

use crate::memmem::{
    byte_frequencies::BYTE_FREQUENCIES, rabinkarp, rabinkarp::NeedleHash,
};

mod fallback;
#[cfg(all(not(miri), target_arch = "x86_64", memchr_runtime_simd))]
mod x86;

/// The maximum frequency rank permitted. If the rarest byte in the needle
/// has a frequency rank above this value, then Freqy is not used.
const MAX_FALLBACK_RANK: usize = 250;

/// The type of a prefilter function. All prefilters must satisfy this
/// signature.
///
/// A prefilter function describes both forward and reverse searches. In the
/// case of a forward search, the position returned corresponds to the starting
/// offset of a match (confirmed or possible). Its minimum value is `0`, and
/// its maximum value is `haystack.len() - 1`. In the case of a reverse search,
/// the position returned corresponds to the position immediately after a match
/// (confirmed or possible). Its minimum value is `1` and its maximum value is
/// `haystack.len()`.
///
/// In both cases, the position returned is either a confirmed match (in the
/// case where a prefilter can determine a full match) or is the starting
/// point of a _possible_ match. That is, returning a false positive is okay.
/// A prefilter, however, must never return any false negatives. That is, if a
/// match exists at a particular position `i`, then a prefilter _must_ return
/// that position. It cannot skip past it.
///
/// Using a function pointer like this does inhibit inlining, but it does
/// eliminate branching and the extra costs associated with copying a larger
/// enum. Note also, that using Box<dyn SomePrefilterTrait> can't really work
/// here, since we want to work in contexts that don't have dynamic memory
/// allocation. Moreover, in the default configuration of this crate on x86_64
/// CPUs released in the past ~decade, we will use an AVX2-optimized prefilter,
/// which generally won't be inlineable into the surrounding code anyway.
/// (Unless AVX2 is enabled at compile time, but this is typically rare, since
/// it produces a non-portable binary.)
pub(crate) type PrefilterFn = unsafe fn(
    prestate: &mut PrefilterState,
    freqy: &NeedleInfo,
    haystack: &[u8],
    needle: &[u8],
) -> Option<usize>;

/// TODO
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Prefilter {
    /// TODO
    None,
    /// TODO
    Auto,
}

impl Default for Prefilter {
    fn default() -> Prefilter {
        Prefilter::Auto
    }
}

impl Prefilter {
    fn is_none(&self) -> bool {
        match *self {
            Prefilter::None => true,
            _ => false,
        }
    }
}

/// PrefilterState tracks state associated with the effectiveness of a
/// prefilter. It is used to track how many bytes, on average, are skipped by
/// the prefilter. If this average dips below a certain threshold over time,
/// then the state renders the prefilter inert and stops using it.
///
/// A prefilter state should be created for each search. (Where creating an
/// iterator is treated as a single search.) A prefilter state should only be
/// created from a `Freqy`. e.g., An inert `Freqy` will produce an inert
/// `PrefilterState`.
#[derive(Clone, Debug)]
pub(crate) struct PrefilterState {
    /// The number of skips that has been executed. This is always 1 greater
    /// than the actual number of skips. The special sentinel value of 0
    /// indicates that the prefilter is inert. This is useful to avoid
    /// additional checks to determine whether the prefilter is still
    /// "effective." Once a prefilter becomes inert, it should no longer be
    /// used (according to our heuristics).
    skips: u32,
    /// The total number of bytes that have been skipped.
    skipped: u32,
}

impl PrefilterState {
    /// The minimum number of skip attempts to try before considering whether
    /// a prefilter is effective or not.
    const MIN_SKIPS: u32 = 50;

    /// The minimum amount of bytes that skipping must average.
    ///
    /// This value was chosen based on varying it and checking
    /// the microbenchmarks. In particular, this can impact the
    /// pathological/repeated-{huge,small} benchmarks quite a bit if it's set
    /// too low.
    const MIN_SKIP_BYTES: u32 = 8;

    /// Create a fresh prefilter state.
    fn new() -> PrefilterState {
        PrefilterState { skips: 1, skipped: 0 }
    }

    /// Create a fresh prefilter state that is always inert.
    fn inert() -> PrefilterState {
        PrefilterState { skips: 0, skipped: 0 }
    }

    /// Update this state with the number of bytes skipped on the last
    /// invocation of the prefilter.
    #[inline]
    pub(crate) fn update(&mut self, skipped: usize) {
        self.skips = self.skips.saturating_add(1);
        // We need to do this dance since it's technically possible for
        // `skipped` to overflow a `u32`. (And we use a `u32` to reduce the
        // size of a prefilter state.)
        if skipped > u32::MAX as usize {
            self.skipped = u32::MAX;
        } else {
            self.skipped = self.skipped.saturating_add(skipped as u32);
        }
    }

    /// Return true if and only if this state indicates that a prefilter is
    /// still effective.
    #[inline]
    pub(crate) fn is_effective(&mut self) -> bool {
        if self.is_inert() {
            return false;
        }
        if self.skips() < PrefilterState::MIN_SKIPS {
            return true;
        }
        if self.skipped >= PrefilterState::MIN_SKIP_BYTES * self.skips() {
            return true;
        }

        // We're inert.
        self.skips = 0;
        false
    }

    #[inline]
    fn is_inert(&self) -> bool {
        self.skips == 0
    }

    #[inline]
    fn skips(&self) -> u32 {
        self.skips.saturating_sub(1)
    }
}

/// TODO
#[derive(Clone)]
pub(crate) struct Freqy {
    pub(crate) ninfo: NeedleInfo,
    prefn: PrefilterFn,
    inert: bool,
}

impl Freqy {
    #[cfg(not(all(not(miri), target_arch = "x86_64", memchr_runtime_simd)))]
    #[inline(always)]
    pub(crate) fn forward(config: &Prefilter, needle: &[u8]) -> Freqy {
        if let Prefilter::None = *config {
            return Freqy::inert();
        }
        let ninfo = match NeedleInfo::forward(needle, false) {
            None => return Freqy::inert(),
            Some(ninfo) => ninfo,
        };
        if rank(needle[ninfo.rare1i as usize]) > MAX_FALLBACK_RANK {
            return Freqy::inert();
        }
        let prefn = fallback::find as PrefilterFn;
        Freqy { ninfo, prefn, inert: false }
    }

    #[cfg(all(not(miri), target_arch = "x86_64", memchr_runtime_simd))]
    #[inline(always)]
    pub(crate) fn forward(config: &Prefilter, needle: &[u8]) -> Freqy {
        let mut ninfo = NeedleInfo::default();
        let mut inert = false;
        let mut prefn: PrefilterFn = fallback::find;

        if config.is_none() || needle.len() <= 1 {
            ninfo.nhash = NeedleHash::new(needle);
            inert = true;
        } else {
            let is_sse = cfg!(memchr_runtime_sse2);
            let mut is_avx = false;
            #[cfg(feature = "std")]
            {
                if cfg!(memchr_runtime_avx) {
                    is_avx = is_x86_feature_detected!("avx2");
                }
            }
            ninfo = NeedleInfo::forward(needle, is_sse || is_avx);
            // TODO: This should use x86::sse::find since memchr_runtime_simd
            // implies that SSE2 is enabled. (But we should still check
            // cfg!(memchr_runtime_sse2).)
            if is_avx {
                prefn = x86::avx::find;
            } else if is_sse {
                prefn = x86::sse::find;
            } else {
                let rare1 = needle[ninfo.rare1i as usize];
                inert = rank(rare1) > MAX_FALLBACK_RANK;
            }
        }
        Freqy { ninfo, prefn, inert }
    }

    /// TODO
    #[inline(always)]
    pub(crate) fn reverse(config: &Prefilter, needle: &[u8]) -> Freqy {
        let mut inert = false;
        let ninfo = if let Prefilter::None = *config {
            inert = true;
            NeedleInfo { rare1i: 0, rare2i: 0, nhash: NeedleHash::new(needle) }
        } else {
            let is_avx = is_x86_feature_detected!("avx2");
            NeedleInfo::reverse(needle, is_avx)
        };
        if (ninfo.rare1i as usize) < needle.len() {
            let rare1 = needle[needle.len() - ninfo.rare1i as usize - 1];
            inert = rank(rare1) > MAX_FALLBACK_RANK;
        }
        let prefn = fallback::rfind;
        let reports_false_positives = true;
        Freqy { ninfo, prefn, inert: false }
    }

    fn inert() -> Freqy {
        Freqy {
            ninfo: NeedleInfo::default(),
            prefn: fallback::find,
            inert: true,
        }
    }

    /// Return a fresh prefilter state that can be used with this prefilter. A
    /// prefilter state is used to track the effectiveness of a prefilter for
    /// speeding up searches. Therefore, the prefilter state should generally
    /// be reused on subsequent searches (such as in an iterator). For searches
    /// on a different haystack, then a new prefilter state should be used.
    pub(crate) fn state(&self) -> PrefilterState {
        if self.inert {
            PrefilterState::inert()
        } else {
            PrefilterState::new()
        }
    }

    /// TODO
    pub(crate) fn find(
        &self,
        state: &mut PrefilterState,
        haystack: &[u8],
        needle: &[u8],
    ) -> Option<usize> {
        unsafe { (self.prefn)(state, &self.ninfo, haystack, needle) }
    }
}

impl core::fmt::Debug for Freqy {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Freqy")
            .field("ninfo", &self.ninfo)
            .field("prefn", &"<fn(...)>")
            .finish()
    }
}

/// A heuristic frequency based prefilter for searching a single needle.
///
/// This prefilter attempts to pick out the byte in a needle that is predicted
/// to occur least frequently, and search for that using fast vectorized
/// routines. If a rare enough byte could not be found, then this prefilter's
/// constructors will return an inert `NeedleInfo`. The purpose of an inert
/// set of offsets is to always produce an inert PrefilterState. In this way,
/// the prefilter will be disabled. (We do this instead of using an Option to
/// reduce space.)
///
/// This can be combined with `PrefilterState` to dynamically render this
/// prefilter inert if it proves to ineffective.
///
/// A set of offsets is only computed for needles of length 2 or greater.
/// Smaller needles should be special cased by the substring search algorithm
/// in use.
#[derive(Clone, Debug, Default)]
pub(crate) struct NeedleInfo {
    /// The leftmost offset of the rarest byte in the needle, according to
    /// pre-computed frequency analysis.
    pub(crate) rare1i: u8,
    /// The leftmost offset of the second rarest byte in the needle, according
    /// to pre-computed frequency analysis.
    ///
    /// The second rarest byte is used as a type of guard for quickly detecting
    /// a mismatch after memchr locates an instance of the rarest byte.
    /// This is a hedge against pathological cases where the pre-computed
    /// frequency analysis may be off. (But of course, does not prevent *all*
    /// pathological cases.)
    ///
    /// In general, rare1i != rare2i by construction, although there is no hard
    /// requirement that they be different. However, since the case of a single
    /// byte needle is handled specially by memchr itself, rare2i generally
    /// always should be different from rare1i since it would otherwise be
    /// ineffective as a guard.
    pub(crate) rare2i: u8,
    pub(crate) nhash: NeedleHash,
}

impl NeedleInfo {
    fn forward(needle: &[u8], skip_first_byte: bool) -> NeedleInfo {
        let nhash = NeedleHash::new(needle);
        if needle.len() <= 1 || needle.len() > u8::MAX as usize {
            // For needles bigger than u8::MAX, our offsets aren't big enough.
            // (We make our offsets small to reduce stack copying.) It also
            // isn't clear if needles that big would benefit from prefilters.
            // If you have a use case for it, please file an issue. In that
            // case, we should probably just adjust the routine below to pick
            // some rare bytes from the first 255 bytes of the needle.
            //
            // Also note that for needles of size 0 or 1, they are special
            // cased in Two-Way.
            return NeedleInfo { rare1i: 0, rare2i: 0, nhash };
        }

        // Find the rarest two bytes. We make them distinct by construction.
        let start = if skip_first_byte && needle.len() >= 3 { 1 } else { 0 };
        let (mut rare1, mut rare1i) = (needle[start], start as u8);
        let (mut rare2, mut rare2i) = (needle[start + 1], start as u8 + 1);
        if rank(rare2) < rank(rare1) {
            mem::swap(&mut rare1, &mut rare2);
            mem::swap(&mut rare1i, &mut rare2i);
        }
        for (i, &b) in needle.iter().enumerate().skip(start + 2) {
            if rank(b) < rank(rare1) {
                rare2 = rare1;
                rare2i = rare1i;
                rare1 = b;
                rare1i = i as u8;
            } else if b != rare1 && rank(b) < rank(rare2) {
                rare2 = b;
                rare2i = i as u8;
            }
        }
        // While not strictly required, we really don't want these to be
        // equivalent. If they were, it would reduce the effectiveness of the
        // prefilter.
        assert_ne!(rare1i, rare2i);
        NeedleInfo { rare1i, rare2i, nhash }
    }

    fn reverse(needle: &[u8], skip_first_byte: bool) -> NeedleInfo {
        let nhash = NeedleHash::new(needle);
        if needle.len() <= 1 || needle.len() > u8::MAX as usize {
            // See comments above for the forward direction.
            return NeedleInfo { rare1i: 0, rare2i: 0, nhash };
        }

        let hash = 0;
        // Find the rarest two bytes. We make them distinct by construction. In
        // reverse, the offsets correspond to the number of bytes from the end
        // of the needle. So `0` is the last byte in the needle.
        let start = if skip_first_byte && needle.len() >= 3 { 1 } else { 0 };
        let (mut rare1i, mut rare2i) = (start as u8, start as u8 + 1);
        let mut rare1 = needle[needle.len() - rare1i as usize - 1];
        let mut rare2 = needle[needle.len() - rare2i as usize - 1];
        if rank(rare2) < rank(rare1) {
            mem::swap(&mut rare1, &mut rare2);
            mem::swap(&mut rare1i, &mut rare2i);
        }
        for (i, &b) in needle.iter().rev().enumerate().skip(start + 2) {
            if rank(b) < rank(rare1) {
                rare2 = rare1;
                rare2i = rare1i;
                rare1 = b;
                rare1i = i as u8;
            } else if b != rare1 && rank(b) < rank(rare2) {
                rare2 = b;
                rare2i = i as u8;
            }
        }
        assert_ne!(rare1i, rare2i);
        NeedleInfo { rare1i, rare2i, nhash }
    }

    /// Return the rare bytes in the given needle in the forward direction. The
    /// needle given must be the same one given to the NeedleInfo constructor.
    pub(crate) fn fwd_rare(&self, needle: &[u8]) -> (u8, u8) {
        (needle[self.rare1i as usize], needle[self.rare2i as usize])
    }

    /// Return the rare bytes in the given needle in the reverse direction. The
    /// needle given must be the same one given to the NeedleInfo constructor.
    pub(crate) fn rev_rare(&self, needle: &[u8]) -> (u8, u8) {
        let i1 = needle.len() - self.rare1i as usize - 1;
        let i2 = needle.len() - self.rare2i as usize - 1;
        (needle[i1], needle[i2])
    }

    /// Return the rare offsets for this neelde info such that the first offset
    /// is always <= to the second offset. This is useful when a prefilter
    /// doesn't care whether rare1 is rarer than rare2, but just wants to
    /// ensure that they are ordered with respect to one another.
    pub(crate) fn as_rare_ordered_usize(&self) -> (usize, usize) {
        if self.rare1i <= self.rare2i {
            (self.rare1i as usize, self.rare2i as usize)
        } else {
            (self.rare2i as usize, self.rare1i as usize)
        }
    }

    /// Return the rare offsets for this neelde info as usize values in
    /// the order in which they were constructed. rare1, for example, is
    /// constructed as the "rarer" byte, and thus, prefilters may want to treat
    /// it differently from rare2.
    pub(crate) fn as_rare_usize(&self) -> (usize, usize) {
        (self.rare1i as usize, self.rare2i as usize)
    }
}

/// Return the heuristical frequency rank of the given byte. A lower rank
/// means the byte is believed to occur less frequently.
fn rank(b: u8) -> usize {
    BYTE_FREQUENCIES[b as usize] as usize
}

#[cfg(all(test, not(miri)))]
pub(crate) mod tests {
    use std::convert::{TryFrom, TryInto};

    use super::*;

    /// A test that represents the input and expected output to a prefilter
    /// function. The test should be able to run with any prefilter function
    /// and get the expected output.
    pub(crate) struct PrefilterTest {
        pub(crate) ninfo: NeedleInfo,
        pub(crate) haystack: Vec<u8>,
        pub(crate) needle: Vec<u8>,
        pub(crate) output: Option<usize>,
    }

    impl PrefilterTest {
        pub(crate) unsafe fn run_all_tests(prefn: PrefilterFn) {
            PrefilterTest::run_all_tests_filter(prefn, |_| true)
        }

        pub(crate) unsafe fn run_all_tests_filter(
            prefn: PrefilterFn,
            mut predicate: impl FnMut(&PrefilterTest) -> bool,
        ) {
            for seed in PREFILTER_TEST_SEEDS {
                for test in seed.generate() {
                    if predicate(&test) {
                        test.run(prefn);
                    }
                }
            }
        }

        fn new(
            seed: &PrefilterTestSeed,
            rare1i: usize,
            rare2i: usize,
            haystack_len: usize,
            needle_len: usize,
            output: Option<usize>,
        ) -> Option<PrefilterTest> {
            let rare1i: u8 = rare1i.try_into().unwrap();
            let rare2i: u8 = rare2i.try_into().unwrap();
            let mut haystack = vec![b'@'; haystack_len];
            let mut needle = vec![b'#'; needle_len];
            needle[0] = seed.first;
            needle[rare1i as usize] = seed.rare1;
            needle[rare2i as usize] = seed.rare2;
            if let Some(i) = output {
                haystack[i..i + needle.len()].copy_from_slice(&needle);
            }
            let mut ninfo = NeedleInfo {
                rare1i,
                rare2i,
                nhash: rabinkarp::NeedleHash::new(&needle),
            };
            // If the operations above lead to rare offsets pointing to the
            // non-first occurrence of a byte, then adjust it. This might lead
            // to redundant tests, but it's simpler than trying to change the
            // generation process I think.
            if let Some(i) = crate::memchr(seed.rare1, &needle) {
                ninfo.rare1i = u8::try_from(i).unwrap();
            }
            if let Some(i) = crate::memchr(seed.rare2, &needle) {
                ninfo.rare2i = u8::try_from(i).unwrap();
            }
            Some(PrefilterTest { ninfo, haystack, needle, output })
        }

        unsafe fn run(&self, prefn: PrefilterFn) {
            let mut prestate = PrefilterState::new();
            assert_eq!(
                self.output,
                prefn(
                    &mut prestate,
                    &self.ninfo,
                    &self.haystack,
                    &self.needle
                ),
                "ninfo: {:?}, haystack(len={}): {:?}, needle(len={}): {:?}",
                self.ninfo,
                self.haystack.len(),
                std::str::from_utf8(&self.haystack).unwrap(),
                self.needle.len(),
                std::str::from_utf8(&self.needle).unwrap(),
            );
        }
    }

    const PREFILTER_TEST_SEEDS: &[PrefilterTestSeed] = &[
        PrefilterTestSeed { first: b'x', rare1: b'y', rare2: b'z' },
        PrefilterTestSeed { first: b'x', rare1: b'x', rare2: b'z' },
        PrefilterTestSeed { first: b'x', rare1: b'y', rare2: b'x' },
        PrefilterTestSeed { first: b'x', rare1: b'x', rare2: b'x' },
        PrefilterTestSeed { first: b'x', rare1: b'y', rare2: b'y' },
    ];

    struct PrefilterTestSeed {
        first: u8,
        rare1: u8,
        rare2: u8,
    }

    impl PrefilterTestSeed {
        fn generate(&self) -> Vec<PrefilterTest> {
            let mut tests = vec![];
            let mut push = |test: Option<PrefilterTest>| {
                if let Some(test) = test {
                    tests.push(test);
                }
            };
            let len_start = 2;
            let mut count = 0;
            for needle_len in len_start..=40 {
                let rare_start = len_start - 1;
                for rare1i in rare_start..needle_len {
                    for rare2i in rare1i..needle_len {
                        for haystack_len in needle_len..=66 {
                            push(PrefilterTest::new(
                                self,
                                rare1i,
                                rare2i,
                                haystack_len,
                                needle_len,
                                None,
                            ));
                            for output in 0..=(haystack_len - needle_len) {
                                push(PrefilterTest::new(
                                    self,
                                    rare1i,
                                    rare2i,
                                    haystack_len,
                                    needle_len,
                                    Some(output),
                                ));
                            }
                        }
                    }
                }
            }
            tests
        }
    }
}
