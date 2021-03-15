use crate::memmem::{memmem, memrmem};

/// Each test is a (needle, haystack, expected_fwd, expected_rev) tuple.
type SearchTest = (&'static str, &'static str, Option<usize>, Option<usize>);

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

#[test]
fn forward() {
    run_search_tests_fwd("memmem", |n, h| memmem(h, n));
}

#[test]
fn reverse() {
    run_search_tests_rev("memrmem", |n, h| memrmem(h, n));
}

/// Run the substring search tests. `name` should be the type of searcher used,
/// for diagnostics. `search` should be a closure that accepts a needle and a
/// haystack and returns the starting position of the first occurrence of
/// needle in the haystack, or `None` if one doesn't exist.
fn run_search_tests_fwd(
    name: &str,
    mut search: impl FnMut(&[u8], &[u8]) -> Option<usize>,
) {
    for &(needle, haystack, expected_fwd, _) in SEARCH_TESTS {
        let (n, h) = (needle.as_bytes(), haystack.as_bytes());
        assert_eq!(
            expected_fwd,
            search(n, h),
            "{}: needle: {:?}, haystack: {:?}, expected: {:?}",
            name,
            n,
            h,
            expected_fwd
        );
    }
}

/// Run the substring search tests. `name` should be the type of searcher used,
/// for diagnostics. `search` should be a closure that accepts a needle and a
/// haystack and returns the starting position of the last occurrence of
/// needle in the haystack, or `None` if one doesn't exist.
fn run_search_tests_rev(
    name: &str,
    mut search: impl FnMut(&[u8], &[u8]) -> Option<usize>,
) {
    for &(needle, haystack, _, expected_rev) in SEARCH_TESTS {
        let (n, h) = (needle.as_bytes(), haystack.as_bytes());
        assert_eq!(
            expected_rev,
            search(n, h),
            "{}: needle: {:?}, haystack: {:?}, expected: {:?}",
            name,
            n,
            h,
            expected_rev
        );
    }
}
