use super::fallback;

// We only use AVX when we are certain it's available. This generally requires
// detecting it at runtime with std::is_x86_feature_detected, unless we know AVX
// support is guaranteed at compile time. This generally requires that options
// like `-Ctarget-feature=+avx` or `-Ctarget-cpu=native` be explicitly added to
// RUSTFLAGS.
#[cfg(any(feature = "std", target_feature = "avx"))]
mod avx;
// We handle the edge case of x86_64 machines without guaranteed SSE2 in
// our build.rs
mod sse2;

macro_rules! have_x86_feature {
    ($feat:tt) => {
        cfg!(target_feature = $feat) || {
            #[cfg(feature = "std")]
            {
                std::is_x86_feature_detected!($feat)
            }
            #[cfg(not(feature = "std"))]
            {
                false
            }
        }
    };
}

/// This macro employs a gcc-like "ifunc" trick where by upon first calling
/// `memchr` (for example), CPU feature detection will be performed at runtime
/// to determine the best implementation to use. After CPU feature detection
/// is done, we replace `memchr`'s function pointer with the selection. Upon
/// subsequent invocations, the CPU-specific routine is invoked directly, which
/// skips the CPU feature detection and subsequent branch that's required.
///
/// While this typically doesn't matter for rare occurrences or when used on
/// larger haystacks, `memchr` can be called in tight loops where the overhead
/// of this branch can actually add up *and is measurable*. This trick was
/// necessary to bring this implementation up to glibc's speeds for the 'tiny'
/// benchmarks, for example.
///
/// At some point, I expect the Rust ecosystem will get a nice macro for doing
/// exactly this, at which point, we can replace our hand-jammed version of it.
///
/// N.B. The ifunc strategy does prevent function inlining of course, but
/// on modern CPUs, you'll probably end up with the AVX2 implementation,
/// which probably can't be inlined anyway---unless you've compiled your
/// entire program with AVX2 enabled. However, even then, the various memchr
/// implementations aren't exactly small, so inlining might not help anyway!
#[cfg(any(feature = "std", target_feature = "avx"))]
macro_rules! ifunc {
    ($name:ident, $haystack:ident, $($needle:ident),+) => {{
        if cfg!(memchr_runtime_avx) && have_x86_feature!("avx") {
            unsafe { avx::$name($($needle),+, $haystack) }
        } else if cfg!(memchr_runtime_sse2) {
            unsafe { sse2::$name($($needle),+, $haystack) }
        } else {
            fallback::$name($($needle),+, $haystack)
        }
    }}
}

/// When std isn't available to provide runtime CPU feature detection, or if
/// runtime CPU feature detection has been explicitly disabled, then just
/// call our optimized SSE2 routine directly. SSE2 is avalbale on all x86_64
/// targets, so no CPU feature detection is necessary.
///
/// # Safety
///
/// There are no safety requirements for this definition of the macro. It is
/// safe for all inputs since it is restricted to either the fallback routine
/// or the SSE routine, which is always safe to call on x86_64.
#[cfg(not(any(feature = "std", target_feature = "avx")))]
macro_rules! ifunc {
    ($name:ident, $haystack:ident, $($needle:ident),+) => {{
        if cfg!(memchr_runtime_sse2) {
            unsafe { sse2::$name($($needle),+, $haystack) }
        } else {
            fallback::$name($($needle),+, $haystack)
        }
    }}
}

#[inline(always)]
pub fn memchr(n1: u8, haystack: &[u8]) -> Option<usize> {
    ifunc!(memchr, haystack, n1)
}

#[inline(always)]
pub fn memchr2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    ifunc!(memchr2, haystack, n1, n2)
}

#[inline(always)]
pub fn memchr3(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
    ifunc!(memchr3, haystack, n1, n2, n3)
}

#[inline(always)]
pub fn memrchr(n1: u8, haystack: &[u8]) -> Option<usize> {
    ifunc!(memrchr, haystack, n1)
}

#[inline(always)]
pub fn memrchr2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    ifunc!(memrchr2, haystack, n1, n2)
}

#[inline(always)]
pub fn memrchr3(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
    ifunc!(memrchr3, haystack, n1, n2, n3)
}
