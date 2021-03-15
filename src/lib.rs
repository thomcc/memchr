/*!
The `memchr` crate provides heavily optimized routines for searching bytes.

The `memchr` function is traditionally provided by libc, however, the
performance of `memchr` can vary significantly depending on the specific
implementation of libc that is used. They can range from manually tuned
Assembly implementations (like that found in GNU's libc) all the way to
non-vectorized C implementations (like that found in MUSL).

To smooth out the differences between implementations of libc, at least
on `x86_64` for Rust 1.27+, this crate provides its own implementation of
`memchr` that should perform competitively with the one found in GNU's libc.
The implementation is in pure Rust and has no dependency on a C compiler or an
Assembler.

Additionally, GNU libc also provides an extension, `memrchr`. This crate
provides its own implementation of `memrchr` as well, on top of `memchr2`,
`memchr3`, `memrchr2` and `memrchr3`. The difference between `memchr` and
`memchr2` is that that `memchr2` permits finding all occurrences of two bytes
instead of one. Similarly for `memchr3`.
*/

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/memchr/2.0.0")]

// Supporting 8-bit (or others) would be fine. If you need it, please submit a
// bug report at https://github.com/BurntSushi/rust-memchr
#[cfg(not(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
)))]
compile_error!("memchr currently not supported on non-{16,32,64}");

pub use crate::memchr::{
    memchr, memchr2, memchr2_iter, memchr3, memchr3_iter, memchr_iter,
    memrchr, memrchr2, memrchr2_iter, memrchr3, memrchr3_iter, memrchr_iter,
    Memchr, Memchr2, Memchr3,
};

mod cow;
mod memchr;
pub mod memmem;
#[cfg(test)]
mod tests;
