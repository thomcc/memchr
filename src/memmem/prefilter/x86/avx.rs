use core::arch::x86_64::__m256i;

use crate::memmem::prefilter::{NeedleInfo, PrefilterFn, PrefilterState};

// Check that the functions below satisfy the Prefilter function type.
const _: PrefilterFn = find;

/// An AVX2 accelerated candidate finder for single-substring search.
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn find(
    prestate: &mut PrefilterState,
    ninfo: &NeedleInfo,
    haystack: &[u8],
    needle: &[u8],
) -> Option<usize> {
    super::super::genericsimd::find::<__m256i>(
        prestate,
        ninfo,
        haystack,
        needle,
        super::sse::find,
    )
}

#[cfg(test)]
mod tests {
    use crate::memmem::rabinkarp;

    use super::*;

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
