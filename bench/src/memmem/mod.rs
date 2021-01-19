use std::str;

use criterion::Criterion;
use memchr::{memmem_iter, memrmem_iter};

use crate::{data::*, define};

mod sliceslice;

pub fn all(c: &mut Criterion) {
    iter_fwd(c);
    iter_rev(c);
    sliceslice::all(c);
}

fn iter_fwd(c: &mut Criterion) {
    define_iter_fwd(
        c,
        "rare",
        "en-huge-ascii",
        SUBTITLE_EN_HUGE,
        "Sherlock Holmes",
        1,
    );
    define_iter_fwd(
        c,
        "verycommon1",
        "en-huge-ascii",
        SUBTITLE_EN_HUGE,
        " ",
        76792,
    );
    define_iter_fwd(
        c,
        "verycommon2",
        "en-huge-ascii",
        SUBTITLE_EN_HUGE,
        "  ",
        0,
    );

    define_iter_fwd(
        c,
        "rare",
        "en-small-ascii",
        SUBTITLE_EN_SMALL,
        "IM Pictures",
        1,
    );
    define_iter_fwd(
        c,
        "verycommon1",
        "en-small-ascii",
        SUBTITLE_EN_SMALL,
        " ",
        155,
    );
    define_iter_fwd(
        c,
        "verycommon2",
        "en-small-ascii",
        SUBTITLE_EN_SMALL,
        "  ",
        0,
    );

    define_iter_fwd(
        c,
        "verycommon1",
        "en-tiny-ascii",
        SUBTITLE_EN_TINY,
        " ",
        5,
    );
    define_iter_fwd(
        c,
        "verycommon2",
        "en-tiny-ascii",
        SUBTITLE_EN_TINY,
        "  ",
        0,
    );

    define_iter_fwd(
        c,
        "pathological",
        "repeated-huge",
        REPEATED_RARE_HUGE,
        "abczdef",
        0,
    );
    define_iter_fwd(
        c,
        "pathological",
        "repeated-small",
        REPEATED_RARE_SMALL,
        "abczdef",
        0,
    );
}

fn iter_rev(c: &mut Criterion) {
    define_iter_rev(
        c,
        "rare",
        "en-huge-ascii",
        SUBTITLE_EN_HUGE,
        "Sherlock Holmes",
        1,
    );
    define_iter_rev(
        c,
        "verycommon1",
        "en-huge-ascii",
        SUBTITLE_EN_HUGE,
        " ",
        76792,
    );
    define_iter_rev(
        c,
        "verycommon2",
        "en-huge-ascii",
        SUBTITLE_EN_HUGE,
        "  ",
        0,
    );

    define_iter_rev(
        c,
        "rare",
        "en-small-ascii",
        SUBTITLE_EN_SMALL,
        "IM Pictures",
        1,
    );
    define_iter_rev(
        c,
        "verycommon1",
        "en-small-ascii",
        SUBTITLE_EN_SMALL,
        " ",
        155,
    );
    define_iter_rev(
        c,
        "verycommon2",
        "en-small-ascii",
        SUBTITLE_EN_SMALL,
        "  ",
        0,
    );

    define_iter_rev(
        c,
        "verycommon1",
        "en-tiny-ascii",
        SUBTITLE_EN_TINY,
        " ",
        5,
    );
    define_iter_rev(
        c,
        "verycommon2",
        "en-tiny-ascii",
        SUBTITLE_EN_TINY,
        "  ",
        0,
    );

    define_iter_rev(
        c,
        "pathological",
        "repeated-huge",
        REPEATED_RARE_HUGE,
        "abczdef",
        0,
    );
    define_iter_rev(
        c,
        "pathological",
        "repeated-small",
        REPEATED_RARE_SMALL,
        "abczdef",
        0,
    );
}

fn define_iter_fwd(
    c: &mut Criterion,
    group_name: &str,
    bench_name: &str,
    corpus: &'static [u8],
    needle: &'static str,
    expected: usize,
) {
    let corpus = str::from_utf8(corpus).unwrap();

    let name = format!("memmem/rust/{}", group_name);
    define(c, &name, bench_name, corpus.as_bytes(), move |b| {
        let corpus = corpus.as_bytes();
        b.iter(|| {
            assert_eq!(
                expected,
                memmem_iter(corpus, needle.as_bytes()).count()
            );
        });
    });

    let name = format!("memmem/std/{}", group_name);
    define(c, &name, bench_name, corpus.as_bytes(), move |b| {
        b.iter(|| {
            assert_eq!(expected, corpus.matches(needle).count());
        });
    });
}

fn define_iter_rev(
    c: &mut Criterion,
    group_name: &str,
    bench_name: &str,
    corpus: &'static [u8],
    needle: &'static str,
    expected: usize,
) {
    let corpus = str::from_utf8(corpus).unwrap();

    let name = format!("memrmem/rust/{}", group_name);
    define(c, &name, bench_name, corpus.as_bytes(), move |b| {
        let corpus = corpus.as_bytes();
        b.iter(|| {
            assert_eq!(
                expected,
                memrmem_iter(corpus, needle.as_bytes()).count()
            );
        });
    });

    let name = format!("memrmem/std/{}", group_name);
    define(c, &name, bench_name, corpus.as_bytes(), move |b| {
        b.iter(|| {
            assert_eq!(expected, corpus.rmatches(needle).count());
        });
    });
}
