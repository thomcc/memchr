#![allow(dead_code)]

pub const SHERLOCK_HUGE: &'static [u8] =
    include_bytes!("../data/sherlock-holmes-huge.txt");
pub const SHERLOCK_SMALL: &'static [u8] =
    include_bytes!("../data/sherlock-holmes-small.txt");
pub const SHERLOCK_TINY: &'static [u8] =
    include_bytes!("../data/sherlock-holmes-tiny.txt");

pub const SUBTITLE_EN_HUGE: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-en-huge-ascii.txt");
pub const SUBTITLE_EN_SMALL: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-en-small-ascii.txt");
pub const SUBTITLE_EN_TINY: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-en-tiny-ascii.txt");

pub const SUBTITLE_RU_HUGE: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-ru-huge-utf8.txt");
pub const SUBTITLE_RU_SMALL: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-ru-small-utf8.txt");
pub const SUBTITLE_RU_TINY: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-ru-tiny-utf8.txt");

pub const SUBTITLE_ZH_HUGE: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-zh-huge-utf8.txt");
pub const SUBTITLE_ZH_SMALL: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-zh-small-utf8.txt");
pub const SUBTITLE_ZH_TINY: &'static [u8] =
    include_bytes!("../data/opensubtitles2018-zh-tiny-utf8.txt");

pub const REPEATED_RARE_HUGE: &'static [u8] =
    include_bytes!("../data/repeated-rare-huge");
pub const REPEATED_RARE_SMALL: &'static [u8] =
    include_bytes!("../data/repeated-rare-small");

pub const I386: &'static [u8] = include_bytes!("../data/i386.txt");

pub const WORDS: &'static [u8] = include_bytes!("../data/words.txt");
