use crate::memmem::prefilter::{NeedleInfo, PrefilterFn, PrefilterState};

// Check that the functions below satisfy the Prefilter function type.
const _: PrefilterFn = find;
const _: PrefilterFn = rfind;

/// Look for a possible occurrence of needle. The position returned
/// corresponds to the beginning of the occurrence, if one exists.
///
/// Callers may assume that this never returns false negatives (i.e., it
/// never misses an actual occurrence), but must check that the returned
/// position corresponds to a match. That is, it can return false
/// positives.
///
/// This should only be used when Freqy is constructed for forward
/// searching.
pub(crate) fn find(
    prestate: &mut PrefilterState,
    ninfo: &NeedleInfo,
    haystack: &[u8],
    needle: &[u8],
) -> Option<usize> {
    let mut i = 0;
    let (rare1i, rare2i) = ninfo.as_rare_usize();
    let (rare1, rare2) = ninfo.fwd_rare(needle);
    while prestate.is_effective() {
        // Use a fast vectorized implementation to skip to the next
        // occurrence of the rarest byte (heuristically chosen) in the
        // needle.
        let found = crate::memchr(rare1, &haystack[i..])?;
        prestate.update(found);
        i += found;

        // If we can't align our first match with the haystack, then a
        // match is impossible.
        if i < rare1i {
            i += 1;
            continue;
        }

        // Align our rare2 byte with the haystack. A mismatch means that
        // a match is impossible.
        let aligned_rare2i = i - rare1i + rare2i;
        if haystack.get(aligned_rare2i) != Some(&rare2) {
            i += 1;
            continue;
        }

        // We've done what we can. There might be a match here.
        return Some(i - rare1i);
    }
    // The only way we get here is if we believe our skipping heuristic
    // has become ineffective. We're allowed to return false positives,
    // so return the position at which we advanced to, aligned to the
    // haystack.
    Some(i.saturating_sub(rare1i))
}

/// Look for a possible occurrence of needle, in reverse, starting from the
/// end of the given haystack. The position returned corresponds to the
/// position immediately after the end of the occurrence, if one exists.
///
/// Callers may assume that this never returns false negatives (i.e., it
/// never misses an actual occurrence), but must check that the returned
/// position corresponds to a match. That is, it can return false
/// positives.
///
/// This should only be used when Freqy is constructed for reverse
/// searching.
pub(crate) fn rfind(
    prestate: &mut PrefilterState,
    ninfo: &NeedleInfo,
    haystack: &[u8],
    needle: &[u8],
) -> Option<usize> {
    let mut i = haystack.len();
    let (rare1i, rare2i) = ninfo.as_rare_usize();
    let (rare1, rare2) = ninfo.rev_rare(needle);
    while prestate.is_effective() {
        // Use a fast vectorized implementation to skip to the next
        // occurrence of the rarest byte (heuristically chosen) in the
        // needle.
        let found = crate::memrchr(rare1, &haystack[..i])?;
        prestate.update(i - found);
        i = found;

        // If we can't align our first match with the haystack, then a
        // match is impossible.
        if i + rare1i + 1 > haystack.len() {
            continue;
        }

        // Align our rare2 byte with the haystack. A mismatch means that
        // a match is impossible.
        let aligned = match (i + rare1i).checked_sub(rare2i) {
            None => continue,
            Some(aligned) => aligned,
        };
        if haystack.get(aligned) != Some(&rare2) {
            continue;
        }

        // We've done what we can. There might be a match here.
        return Some(i + rare1i + 1);
    }
    // The only way we get here is if we believe our skipping heuristic
    // has become ineffective. We're allowed to return false positives,
    // so return the position at which we advanced to, aligned to the
    // haystack.
    Some(core::cmp::min(haystack.len(), i + rare1i + 1))
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    fn freqy_find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        let ninfo = NeedleInfo::forward(needle, false);
        let mut prestate = PrefilterState::new();
        find(&mut prestate, &ninfo, haystack, needle)
    }

    fn freqy_rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        let ninfo = NeedleInfo::reverse(needle, false);
        let mut prestate = PrefilterState::new();
        rfind(&mut prestate, &ninfo, haystack, needle)
    }

    #[test]
    fn freqy_forward() {
        assert_eq!(Some(0), freqy_find(b"BARFOO", b"BAR"));
        assert_eq!(Some(3), freqy_find(b"FOOBAR", b"BAR"));
        assert_eq!(Some(0), freqy_find(b"zyzz", b"zyzy"));
        assert_eq!(Some(2), freqy_find(b"zzzy", b"zyzy"));
        assert_eq!(None, freqy_find(b"zazb", b"zyzy"));
        assert_eq!(Some(0), freqy_find(b"yzyy", b"yzyz"));
        assert_eq!(Some(2), freqy_find(b"yyyz", b"yzyz"));
        assert_eq!(None, freqy_find(b"yayb", b"yzyz"));
    }

    #[test]
    fn freqy_reverse() {
        assert_eq!(Some(3), freqy_rfind(b"BARFOO", b"BAR"));
        assert_eq!(Some(6), freqy_rfind(b"FOOBAR", b"BAR"));
        assert_eq!(Some(2), freqy_rfind(b"zyzz", b"zyzy"));
        assert_eq!(Some(4), freqy_rfind(b"zzzy", b"zyzy"));
        assert_eq!(None, freqy_rfind(b"zazb", b"zyzy"));
        assert_eq!(Some(2), freqy_rfind(b"yzyy", b"yzyz"));
        assert_eq!(Some(4), freqy_rfind(b"yyyz", b"yzyz"));
        assert_eq!(None, freqy_rfind(b"yayb", b"yzyz"));
    }

    #[test]
    #[cfg(not(miri))]
    fn prefilter_permutations() {
        use crate::memmem::prefilter::tests::PrefilterTest;

        // SAFETY: super::find is safe to call for all inputs and on all
        // platforms.
        unsafe { PrefilterTest::run_all_tests(super::find) };
    }
}
