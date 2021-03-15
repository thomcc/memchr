use core::{arch::x86_64::*, mem::size_of};

use crate::memmem::{
    prefilter::{fallback, NeedleInfo, PrefilterFn, PrefilterState},
    rabinkarp,
};

// Check that the functions below satisfy the Prefilter function type.
const _: PrefilterFn = find;
const _: PrefilterFn = rfind;

const VECTOR_SIZE: usize = size_of::<__m256i>();

/// An AVX2 accelerated candidate finder for single-substring search.
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn find(
    prestate: &mut PrefilterState,
    ninfo: &NeedleInfo,
    haystack: &[u8],
    needle: &[u8],
) -> Option<usize> {
    Find::new(prestate, ninfo, haystack, needle).run()
}

/// An AVX2 accelerated candidate finder for reverse single-substring search.
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn rfind(
    prestate: &mut PrefilterState,
    ninfo: &NeedleInfo,
    haystack: &[u8],
    needle: &[u8],
) -> Option<usize> {
    todo!()
}

/// The implementation of the forward AVX2 accelerated candidate finder.
///
/// The implementation used to be in a single function, but parts of it beg
/// to be split up into different routines. It was also a more convenient
/// way to experiment with more complicated variants of this prefilter.
///
/// This organization should not have an impact. All relevant routines should
/// get inlined into one big routine.
#[derive(Debug)]
struct Find<'a> {
    prestate: &'a mut PrefilterState,
    ninfo: &'a NeedleInfo,
    haystack: &'a [u8],
    needle: &'a [u8],
    rare1i: usize,
    rare2i: usize,
    min_haystack_len: usize,
    firstchunk: __m256i,
    rare1chunk: __m256i,
    rare2chunk: __m256i,
    start_ptr: *const u8,
    end_ptr: *const u8,
    max_ptr: *const u8,
    ptr: *const u8,
}

impl<'a> Find<'a> {
    /// Initialize the AVX2 candidate finder.
    #[inline(always)]
    unsafe fn new(
        prestate: &'a mut PrefilterState,
        ninfo: &'a NeedleInfo,
        haystack: &'a [u8],
        needle: &'a [u8],
    ) -> Find<'a> {
        assert!(needle.len() >= 2, "needle must be at least 2 bytes");
        let (rare1i, rare2i) = ninfo.as_rare_ordered_usize();

        let min_haystack_len = rare2i + VECTOR_SIZE;
        let start_ptr = haystack.as_ptr();
        let end_ptr = start_ptr.add(haystack.len());
        Find {
            prestate,
            ninfo,
            haystack,
            needle,
            rare1i,
            rare2i,
            min_haystack_len,
            firstchunk: _mm256_set1_epi8(needle[0] as i8),
            rare1chunk: _mm256_set1_epi8(needle[rare1i] as i8),
            rare2chunk: _mm256_set1_epi8(needle[rare2i] as i8),
            start_ptr,
            end_ptr,
            max_ptr: end_ptr.sub(min_haystack_len),
            ptr: start_ptr,
        }
    }

    /// Run the main loop. If a candidate was found, then the position at which
    /// the needle could start is returned. If no candidate was found then no
    /// matches are possible and None is returned.
    #[inline(always)]
    unsafe fn run(&mut self) -> Option<usize> {
        if self.haystack.len() < self.min_haystack_len {
            return super::sse::find(
                self.prestate,
                self.ninfo,
                self.haystack,
                self.needle,
            );
        }
        // N.B. I did experiment with unrolling the loop to deal with 64 bytes
        // at a time and 128 bytes at a time. The double unroll was marginally
        // faster while the quadruple unroll was unambiguously slower. In the
        // end, I decided the complexity from unrolling wasn't worth it. I
        // used the memmem/krate/prebuilt/huge-en/* benchmarks to compare.
        while self.ptr <= self.max_ptr {
            if let Some(chunki) = self.find_in_chunk2() {
                return Some(self.matched(chunki));
            }
            self.ptr = self.ptr.add(VECTOR_SIZE);
        }
        if self.ptr < self.end_ptr {
            // This routine immediately quits if a candidate match is found.
            // That means that if we're here, no candidate matches have been
            // found at or before 'ptr'. Thus, we don't need to mask anything
            // out even though we might technically search part of the haystack
            // that we've already searched (because we know it can't match).
            self.ptr = self.max_ptr;
            if let Some(chunki) = self.find_in_chunk2() {
                return Some(self.matched(chunki));
            }
        }
        self.prestate.update(self.haystack.len());
        None
    }

    // Below are two different techniques for checking whether a candidate
    // match exists in a given chunk or not. find_in_chunk2 checks two bytes
    // where as find_in_chunk3 checks three bytes. The idea behind checking
    // three bytes is that while we do a bit more work per iteration, we
    // decrease the chances of a false positive match being reported and thus
    // make the search faster overall. This actually works out for the
    // memmem/krate/prebuilt/huge-en/never-all-common-bytes benchmark, where
    // using find_in_chunk3 is about 25% faster than find_in_chunk2. However,
    // it turns out that find_in_chunk2 is faster for all other benchmarks, so
    // perhaps the extra check isn't worth it in practice.
    //
    // For now, we go with find_in_chunk2, but we leave find_in_chunk3 around
    // to make it easy to switch to and benchmark when possible.

    /// Search for an occurrence of two rare bytes from the needle in the
    /// current chunk pointed to by self.ptr. It must be valid to do an
    /// unaligned read of 32 bytes starting at both (self.ptr + self.rare1i)
    /// and (self.ptr + self.rare2i).
    #[allow(dead_code)]
    #[inline(always)]
    unsafe fn find_in_chunk2(&mut self) -> Option<usize> {
        let chunk0 =
            _mm256_loadu_si256(self.ptr.add(self.rare1i) as *const __m256i);
        let chunk1 =
            _mm256_loadu_si256(self.ptr.add(self.rare2i) as *const __m256i);

        let eq0 = _mm256_cmpeq_epi8(chunk0, self.rare1chunk);
        let eq1 = _mm256_cmpeq_epi8(chunk1, self.rare2chunk);

        let match_offsets = _mm256_movemask_epi8(_mm256_and_si256(eq0, eq1));
        if match_offsets == 0 {
            return None;
        }
        Some(match_offsets.trailing_zeros() as usize)
    }

    /// Search for an occurrence of two rare bytes and the first byte (even
    /// if one of the rare bytes is equivalent to the first byte) from the
    /// needle in the current chunk pointed to by self.ptr. It must be valid
    /// to do an unaligned read of 32 bytes starting at self.ptr, (self.ptr +
    /// self.rare1i) and (self.ptr + self.rare2i).
    #[allow(dead_code)]
    #[inline(always)]
    unsafe fn find_in_chunk3(&mut self) -> Option<usize> {
        let chunk0 = _mm256_loadu_si256(self.ptr as *const __m256i);
        let chunk1 =
            _mm256_loadu_si256(self.ptr.add(self.rare1i) as *const __m256i);
        let chunk2 =
            _mm256_loadu_si256(self.ptr.add(self.rare2i) as *const __m256i);

        let eq0 = _mm256_cmpeq_epi8(chunk0, self.firstchunk);
        let eq1 = _mm256_cmpeq_epi8(chunk1, self.rare1chunk);
        let eq2 = _mm256_cmpeq_epi8(chunk2, self.rare2chunk);

        let match_offsets = _mm256_movemask_epi8(_mm256_and_si256(
            eq0,
            _mm256_and_si256(eq1, eq2),
        ));
        if match_offsets == 0 {
            return None;
        }
        Some(match_offsets.trailing_zeros() as usize)
    }

    /// Accepts a chunk-relative offset and returns a haystack relative offset
    /// after updating the prefilter state.
    ///
    /// Why do we use this unlineable function when a search completes? Well,
    /// I don't know. Really. Obviously this function was not here initially.
    /// When doing profiling, the codegen for the inner loop here looked bad
    /// and I didn't know why. There were a couple extra 'add' instructions and
    /// an extra 'lea' instruction that I couldn't explain. I hypothesized that
    /// the optimizer was having trouble untangling the hot code in the loop
    /// from the code that deals with a candidate match. By putting the latter
    /// into an unlineable function, it kind of forces the issue and it had
    /// the intended effect: codegen improved measurably. It's good for a ~10%
    /// improvement across the board on the memmem/krate/prebuilt/huge-en/*
    /// benchmarks.
    #[cold]
    #[inline(never)]
    fn matched(&mut self, chunki: usize) -> usize {
        let found = diff(self.ptr, self.start_ptr) + chunki;
        self.prestate.update(found);
        found
    }
}

/// Subtract `b` from `a` and return the difference. `a` must be greater than
/// or equal to `b`.
fn diff(a: *const u8, b: *const u8) -> usize {
    debug_assert!(a >= b);
    (a as usize) - (b as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch() {
        // let needle = b"\x00\x00";
        let needle = format!("{}z", "a".repeat(32)).into_bytes();
        let haystack = "\x00".repeat(160).into_bytes();
        assert_eq!(None, crate::memmem::memmem(&haystack, &needle));
    }

    #[test]
    #[cfg(not(miri))]
    fn prefilter_permutations() {
        use crate::memmem::prefilter::tests::PrefilterTest;
        if !is_x86_feature_detected!("avx2") {
            return;
        }
        // SAFETY: The safety of super::find only requires that the current
        // CPU support AVX2, which we checked above.
        unsafe { PrefilterTest::run_all_tests(super::find) };
    }

    // These are specific regression tests that were discovered via the
    // permutation test above. We split them out like this so that we track
    // them more explicitly. Some of the tests were failures because of bad
    // test data itself, and were useful for debugging that.

    fn perm_find_fwd(
        rare1i: u8,
        rare2i: u8,
        haystack: &str,
        needle: &str,
    ) -> Option<Option<usize>> {
        if !is_x86_feature_detected!("avx2") {
            return None;
        }
        let mut prestate = PrefilterState::new();
        let nhash = rabinkarp::NeedleHash::new(needle.as_bytes());
        let ninfo = NeedleInfo { rare1i, rare2i, nhash };
        // SAFETY: The safety of super::find only requires that the current
        // CPU support AVX2, which we checked above.
        unsafe {
            Some(super::find(
                &mut prestate,
                &ninfo,
                haystack.as_bytes(),
                needle.as_bytes(),
            ))
        }
    }

    // This was faulty test data, since the rare indices pointed to second
    // occurrence of the rare byte instead of the first. The prefilters assume
    // that the indices always point to the first occurrence. This is critical
    // for correctness.
    #[test]
    fn perm1() {
        let got = match perm_find_fwd(1, 1, "@xx", "xx") {
            None => return,
            Some(got) => got,
        };
        // This was what was expected by the test data:
        // assert_eq!(Some(1), got);
        // But this is what was actually received:
        // assert_eq!(Some(0), got);
        // ... but then I added Rabin-Karp handling for small haystacks,
        // so we actually wind up with Some(1) now.
        // ... but then I switched to a simple memchr,
        // so we're back to Some(0).
        assert_eq!(Some(0), got);
    }
}
