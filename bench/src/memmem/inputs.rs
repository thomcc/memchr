use crate::data;

#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// A name describing the corpus, used to identify it in benchmarks.
    pub name: &'static str,
    /// The haystack to search.
    pub corpus: &'static str,
    /// Queries that are expected to never occur.
    pub never: &'static [Query],
    /// Queries that are expected to occur rarely.
    pub rare: &'static [Query],
    /// Queries that are expected to fairly common.
    pub common: &'static [Query],
}

/// A substring search query for a particular haystack.
#[derive(Clone, Copy, Debug)]
pub struct Query {
    /// A name for this query, used to identify it in benchmarks.
    pub name: &'static str,
    /// The needle to search for.
    pub needle: &'static str,
    /// The expected number of occurrences.
    pub count: usize,
}

pub const INPUTS: &'static [Input] = &[
    Input {
        name: "huge-en",
        corpus: data::SUBTITLE_EN_HUGE,
        never: &[
            Query { name: "john-watson", needle: "John Watson", count: 0 },
            Query { name: "all-common-bytes", needle: "sternness", count: 0 },
            Query { name: "some-rare-bytes", needle: "quartz", count: 0 },
            Query { name: "two-space", needle: "  ", count: 0 },
        ],
        rare: &[
            Query {
                name: "sherlock-holmes",
                needle: "Sherlock Holmes",
                count: 1,
            },
            Query { name: "sherlock", needle: "Sherlock", count: 1 },
        ],
        common: &[
            Query { name: "that", needle: "that", count: 865 },
            Query { name: "one-space", needle: " ", count: 96667 },
            Query { name: "you", needle: "you", count: 5016 },
            // It would be nice to benchmark this case, although it's not
            // terribly important. The problem is that std's substring
            // implementation (correctly) never returns match offsets that
            // split an encoded codepoint, where as memmem on bytes will.
            // So the counts differ.
            // Query { name: "empty", needle: "", count: 613655 },
        ],
    },
    Input {
        name: "huge-ru",
        corpus: data::SUBTITLE_RU_HUGE,
        never: &[Query {
            name: "john-watson",
            needle: "Джон Уотсон",
            count: 0,
        }],
        rare: &[
            Query {
                name: "sherlock-holmes",
                needle: "Шерлок Холмс",
                count: 1,
            },
            Query { name: "sherlock", needle: "Шерлок", count: 1 },
        ],
        common: &[
            Query { name: "that", needle: "что", count: 998 },
            Query { name: "not", needle: "не", count: 3092 },
            Query { name: "one-space", needle: " ", count: 46941 },
        ],
    },
    Input {
        name: "huge-zh",
        corpus: data::SUBTITLE_ZH_HUGE,
        never: &[Query {
            name: "john-watson", needle: "约翰·沃森", count: 0
        }],
        rare: &[
            Query {
                name: "sherlock-holmes",
                needle: "夏洛克·福尔摩斯",
                count: 1,
            },
            Query { name: "sherlock", needle: "夏洛克", count: 1 },
        ],
        common: &[
            Query { name: "that", needle: "那", count: 1056 },
            Query { name: "do-not", needle: "不", count: 2751 },
            Query { name: "one-space", needle: " ", count: 17232 },
        ],
    },
    Input {
        name: "teeny-en",
        corpus: data::SUBTITLE_EN_TEENY,
        never: &[
            Query { name: "john-watson", needle: "John Watson", count: 0 },
            Query { name: "all-common-bytes", needle: "sternness", count: 0 },
            Query { name: "some-rare-bytes", needle: "quartz", count: 0 },
            Query { name: "two-space", needle: "  ", count: 0 },
        ],
        rare: &[
            Query {
                name: "sherlock-holmes",
                needle: "Sherlock Holmes",
                count: 1,
            },
            Query { name: "sherlock", needle: "Sherlock", count: 1 },
        ],
        common: &[],
    },
    Input {
        name: "teeny-ru",
        corpus: data::SUBTITLE_RU_TEENY,
        never: &[Query {
            name: "john-watson",
            needle: "Джон Уотсон",
            count: 0,
        }],
        rare: &[
            Query {
                name: "sherlock-holmes",
                needle: "Шерлок Холмс",
                count: 1,
            },
            Query { name: "sherlock", needle: "Шерлок", count: 1 },
        ],
        common: &[],
    },
    Input {
        name: "teeny-zh",
        corpus: data::SUBTITLE_ZH_TEENY,
        never: &[Query {
            name: "john-watson", needle: "约翰·沃森", count: 0
        }],
        rare: &[
            Query {
                name: "sherlock-holmes",
                needle: "夏洛克·福尔摩斯",
                count: 1,
            },
            Query { name: "sherlock", needle: "夏洛克", count: 1 },
        ],
        common: &[],
    },
    Input {
        name: "pathological-md5-huge",
        corpus: data::PATHOLOGICAL_MD5_HUGE,
        never: &[Query {
            name: "no-hash",
            needle: "61a1a40effcf97de24505f154a306597",
            count: 0,
        }],
        rare: &[Query {
            name: "last-hash",
            needle: "831df319d8597f5bc793d690f08b159b",
            count: 1,
        }],
        common: &[Query { name: "two-bytes", needle: "fe", count: 520 }],
    },
    Input {
        name: "pathological-repeated-rare-huge",
        corpus: data::PATHOLOGICAL_REPEATED_RARE_HUGE,
        never: &[Query { name: "tricky", needle: "abczdef", count: 0 }],
        rare: &[],
        common: &[Query { name: "match", needle: "zzzzzzzzzz", count: 50010 }],
    },
    Input {
        name: "pathological-repeated-rare-small",
        corpus: data::PATHOLOGICAL_REPEATED_RARE_SMALL,
        never: &[Query { name: "tricky", needle: "abczdef", count: 0 }],
        rare: &[],
        common: &[Query { name: "match", needle: "zzzzzzzzzz", count: 100 }],
    },
    Input {
        name: "pathological-defeat-simple-vector",
        corpus: data::PATHOLOGICAL_DEFEAT_SIMPLE_VECTOR,
        never: &[],
        rare: &[Query {
            name: "alphabet",
            needle: "qbz",
            count: 1,
        }],
        common: &[],
    },
    Input {
        name: "pathological-defeat-simple-vector-freq",
        corpus: data::PATHOLOGICAL_DEFEAT_SIMPLE_VECTOR_FREQ,
        never: &[],
        rare: &[Query {
            name: "alphabet",
            needle: "qjaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaz",
            count: 1,
        }],
        common: &[],
    },
    Input {
        name: "pathological-defeat-simple-vector-repeated",
        corpus: data::PATHOLOGICAL_DEFEAT_SIMPLE_VECTOR_REPEATED,
        never: &[],
        rare: &[Query {
            name: "alphabet",
            needle: "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzaz",
            count: 1,
        }],
        common: &[],
    },
];
