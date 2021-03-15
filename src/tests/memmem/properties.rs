use quickcheck::quickcheck;

use crate::memmem::{memmem, memrmem};

quickcheck! {
    fn qc_twoway_fwd_prefix_is_substring(bs: Vec<u8>) -> bool {
        prop_prefix_is_substring(false, &bs, |n, h| memmem(h, n))
    }

    fn qc_twoway_fwd_suffix_is_substring(bs: Vec<u8>) -> bool {
        prop_suffix_is_substring(false, &bs, |n, h| memmem(h, n))
    }

    fn qc_twoway_rev_prefix_is_substring(bs: Vec<u8>) -> bool {
        prop_prefix_is_substring(true, &bs, |n, h| memrmem(h, n))
    }

    fn qc_twoway_rev_suffix_is_substring(bs: Vec<u8>) -> bool {
        prop_suffix_is_substring(true, &bs, |n, h| memrmem(h, n))
    }

    fn qc_twoway_fwd_matches_naive(
        needle: Vec<u8>,
        haystack: Vec<u8>
    ) -> bool {
        prop_matches_naive(
            false,
            &needle,
            &haystack,
            |n, h| memmem(h, n),
        )
    }

    fn qc_twoway_rev_matches_naive(
        needle: Vec<u8>,
        haystack: Vec<u8>
    ) -> bool {
        prop_matches_naive(
            true,
            &needle,
            &haystack,
            |n, h| memrmem(h, n),
        )
    }
}

/// Check that every prefix of the given byte string is a substring.
fn prop_prefix_is_substring(
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
            assert_eq!(naive_rfind(prefix, bs), search(prefix, bs));
        } else {
            assert_eq!(naive_find(prefix, bs), search(prefix, bs));
        }
    }
    true
}

/// Check that every suffix of the given byte string is a substring.
fn prop_suffix_is_substring(
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
            assert_eq!(naive_rfind(suffix, bs), search(suffix, bs));
        } else {
            assert_eq!(naive_find(suffix, bs), search(suffix, bs));
        }
    }
    true
}

/// Check that naive substring search matches the result of the given search
/// algorithm.
fn prop_matches_naive(
    reverse: bool,
    needle: &[u8],
    haystack: &[u8],
    mut search: impl FnMut(&[u8], &[u8]) -> Option<usize>,
) -> bool {
    if reverse {
        naive_rfind(needle, haystack) == search(needle, haystack)
    } else {
        naive_find(needle, haystack) == search(needle, haystack)
    }
}

/// Naively search forwards for the given needle in the given haystack.
fn naive_find(needle: &[u8], haystack: &[u8]) -> Option<usize> {
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
fn naive_rfind(needle: &[u8], haystack: &[u8]) -> Option<usize> {
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
