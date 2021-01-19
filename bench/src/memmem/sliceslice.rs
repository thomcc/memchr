// use std::{
// fs::{self, File},
// io::{BufRead, BufReader},
// };

// use criterion::{black_box, criterion_group, criterion_main, Criterion};
use criterion::{black_box, Criterion};

use crate::{data::*, define};

pub fn all(c: &mut Criterion) {
    search_short_haystack(c);
    search_long_haystack(c);
}

fn search_short_haystack(c: &mut Criterion) {
    let words = std::str::from_utf8(WORDS).unwrap();
    let mut words = words.lines().collect::<Vec<_>>();
    words.sort_unstable_by_key(|word| word.len());
    let words: Vec<&str> = words.iter().map(|&s| s).collect();

    let needles = words.clone();
    define(c, "memmem/crate", "words/short", &[], move |b| {
        let searchers = needles
            .iter()
            .map(|needle| memchr::MemmemFinder::new(needle.as_bytes()))
            .collect::<Vec<_>>();
        b.iter(|| {
            for (i, searcher) in searchers.iter().enumerate() {
                for haystack in &needles[i..] {
                    black_box(searcher.find(haystack.as_bytes()).is_some());
                }
            }
        });
    });

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use sliceslice::x86::DynamicAvx2Searcher;

        let needles = words.clone();
        define(c, "memmem/sliceslice", "words/short", &[], move |b| {
            let searchers = needles
                .iter()
                .map(|&needle| unsafe {
                    DynamicAvx2Searcher::new(
                        needle.as_bytes().to_owned().into_boxed_slice(),
                    )
                })
                .collect::<Vec<_>>();

            b.iter(|| {
                for (i, searcher) in searchers.iter().enumerate() {
                    for haystack in &needles[i..] {
                        black_box(unsafe {
                            searcher.search_in(haystack.as_bytes())
                        });
                    }
                }
            });
        });
    }
}

fn search_long_haystack(c: &mut Criterion) {
    let words = std::str::from_utf8(WORDS).unwrap();
    let words: Vec<&str> = words.lines().collect();
    let i386 = String::from_utf8_lossy(I386);

    let haystack = i386.clone();
    let needles = words.clone();
    define(c, "memmem/crate", "words/long", &[], move |b| {
        let searchers = needles
            .iter()
            .map(|needle| memchr::MemmemFinder::new(needle.as_bytes()))
            .collect::<Vec<_>>();
        b.iter(|| {
            for searcher in searchers.iter() {
                black_box(searcher.find(haystack.as_bytes()).is_some());
            }
        });
    });

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use sliceslice::x86::DynamicAvx2Searcher;

        let haystack = i386.clone();
        let needles = words.clone();
        define(c, "memmem/sliceslice", "words/long", &[], move |b| {
            let searchers = needles
                .iter()
                .map(|needle| unsafe {
                    DynamicAvx2Searcher::new(
                        needle.as_bytes().to_owned().into_boxed_slice(),
                    )
                })
                .collect::<Vec<_>>();

            b.iter(|| {
                for searcher in &searchers {
                    black_box(unsafe {
                        searcher.search_in(haystack.as_bytes())
                    });
                }
            });
        });
    }
}
