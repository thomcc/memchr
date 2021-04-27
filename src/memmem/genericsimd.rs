use core::mem::size_of;

use crate::memmem::{
    prefilter::{PrefilterFnTy, PrefilterState},
    util::memcmp,
    vector::Vector,
    NeedleInfo,
};

/// The implementation of the forward vector accelerated substring search.
///
/// This is extremely similar to the prefilter vector module by the same name.
/// The key difference is that this is not a prefilter. Instead, it handles
/// confirming its own matches. The trade off is that this only works with
/// smaller needles. The speed up here is that an inlined memcmp on a tiny
/// needle is very quick, even on pathological inputs. This is much better than
/// combining a prefilter with Two-Way, where using Two-Way to confirm the
/// match has higher latency.
///
/// So why not use this for all needles? We could, and it would probably work
/// really well on most inputs. But its worst case is multiplicative and we
/// want to guarantee worst case additive time. Some of the benchmarks try to
/// justify this (see the pathological ones).
///
/// The prefilter variant of this has more comments. Also note that we only
/// implement this for forward searches for now. If you have a compelling use
/// case for accelerated reverse search, please file an issue.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Forward {
    needle: u128,
    mask: u128,
    rare1i: u8,
    rare2i: u8,
}

impl Forward {
    /// Create a new "generic simd" forward searcher. If one could not be
    /// created from the given inputs, then None is returned.
    pub(crate) fn new(ninfo: &NeedleInfo, needle: &[u8]) -> Option<Forward> {
        let (rare1i, rare2i) = ninfo.rarebytes.as_rare_ordered_u8();
        // If the needle is too short or too long, give up. Also, give up
        // if the rare bytes detected are at the same position. (It likely
        // suggests a degenerate case, although it should technically not be
        // possible.)
        if needle.len() < 2 || needle.len() > 16 || rare1i == rare2i {
            return None;
        }
        let mask = if needle.len() == 16 {
            !0
        } else {
            (1 << (8 * needle.len())) - 1
        };
        let mut needle_int = 0u128;
        // SAFETY: We've ensured that needle.len() <= 16 and any bit pattern is
        // valid for a u128. Finally, copy_to_nonoverlapping handles unaligned
        // loads, so alignment is not a concern.
        unsafe {
            needle.as_ptr().copy_to_nonoverlapping(
                &mut needle_int as *mut u128 as *mut u8,
                needle.len(),
            );
        }
        Some(Forward { needle: needle_int, mask, rare1i, rare2i })
    }

    /// Returns the minimum length of haystack that is needed for this searcher
    /// to work for a particular vector. Passing a haystack with a length
    /// smaller than this will cause `fwd_find` to panic.
    #[inline(always)]
    pub(crate) fn min_haystack_len<V: Vector>(&self) -> usize {
        self.rare2i as usize + size_of::<V>()
    }
}

/// Searches the given haystack for the given needle. The needle given should
/// be the same as the needle that this searcher was initialized with.
///
/// # Panics
///
/// When the given haystack has a length smaller than `min_haystack_len`.
///
/// # Safety
///
/// Since this is meant to be used with vector functions, callers need to
/// specialize this inside of a function with a `target_feature` attribute.
/// Therefore, callers must ensure that whatever target feature is being used
/// supports the vector functions that this function is specialized for. (For
/// the specific vector functions used, see the Vector trait implementations.)
#[inline(always)]
pub(crate) unsafe fn fwd_find<V: Vector>(
    fwd: &Forward,
    haystack: &[u8],
    needle: &[u8],
) -> Option<usize> {
    // It would be nice if we didn't have this check here, since the meta
    // searcher should handle it for us. But without this, I don't this we
    // guarantee that end_ptr.sub(needle.len()) won't result in UB. We could
    // put it as part of the safety contract, but it makes it more complicated
    // than necessary.
    if haystack.len() < needle.len() {
        return None;
    }
    let min_haystack_len = fwd.min_haystack_len::<V>();
    assert!(haystack.len() >= min_haystack_len, "haystack too small");
    debug_assert!(needle.len() <= haystack.len());
    debug_assert!(needle.len() >= 2, "needle must be at least 2 bytes");
    debug_assert!(needle.len() <= 16, "needle must be at most 16 bytes");

    let (rare1i, rare2i) = (fwd.rare1i as usize, fwd.rare2i as usize);
    let rare1chunk = V::splat(needle[rare1i]);
    let rare2chunk = V::splat(needle[rare2i]);

    let start_ptr = haystack.as_ptr();
    let end_ptr = start_ptr.add(haystack.len());
    let max_ptr = end_ptr.sub(min_haystack_len);
    let mut ptr = start_ptr;

    // N.B. I did experiment with unrolling the loop to deal with size(V)
    // bytes at a time and 2*size(V) bytes at a time. The double unroll was
    // marginally faster while the quadruple unroll was unambiguously slower.
    // In the end, I decided the complexity from unrolling wasn't worth it. I
    // used the memmem/krate/prebuilt/huge-en/ benchmarks to compare.
    while ptr <= max_ptr {
        let m = fwd_find_in_chunk2(
            fwd, needle, ptr, end_ptr, rare1chunk, rare2chunk, !0,
        );
        if let Some(chunki) = m {
            return Some(matched(start_ptr, ptr, chunki));
        }
        ptr = ptr.add(size_of::<V>());
    }
    if ptr < end_ptr {
        let remaining = diff(end_ptr, ptr);
        debug_assert!(
            remaining < min_haystack_len,
            "remaining bytes should be smaller than the minimum haystack \
             length of {}, but there are {} bytes remaining",
            min_haystack_len,
            remaining,
        );
        if remaining < needle.len() {
            return None;
        }
        debug_assert!(
            max_ptr < ptr,
            "after main loop, ptr should have exceeded max_ptr",
        );
        let overlap = diff(ptr, max_ptr);
        debug_assert!(
            overlap > 0,
            "overlap ({}) must always be non-zero",
            overlap,
        );
        debug_assert!(
            overlap < size_of::<V>(),
            "overlap ({}) cannot possibly be >= than a vector ({})",
            overlap,
            size_of::<V>(),
        );
        // The mask has all of its bits set except for the first N least
        // significant bits, where N=overlap. This way, any matches that
        // occur in find_in_chunk within the overlap are automatically
        // ignored.
        let mask = !((1 << overlap) - 1);
        ptr = max_ptr;
        let m = fwd_find_in_chunk2(
            fwd, needle, ptr, end_ptr, rare1chunk, rare2chunk, mask,
        );
        if let Some(chunki) = m {
            return Some(matched(start_ptr, ptr, chunki));
        }
    }
    None
}

/// Search for an occurrence of two rare bytes from the needle in the current
/// chunk pointed to by ptr. It must be valid to do an unaligned read of
/// size(V) bytes starting at both (ptr + rare1i) and (ptr + rare2i). It
/// must also be valid to do an unaligned read of 16 bytes starting at
/// max_start_ptr.
///
/// rare1chunk and rare2chunk correspond to vectors with the rare1 and rare2
/// bytes repeated in each 8-bit lane, respectively.
#[inline(always)]
unsafe fn fwd_find_in_chunk2<V: Vector>(
    fwd: &Forward,
    needle: &[u8],
    ptr: *const u8,
    end_ptr: *const u8,
    rare1chunk: V,
    rare2chunk: V,
    mask: u32,
) -> Option<usize> {
    let chunk0 = V::load_unaligned(ptr.add(fwd.rare1i as usize));
    let chunk1 = V::load_unaligned(ptr.add(fwd.rare2i as usize));

    let eq0 = chunk0.cmpeq(rare1chunk);
    let eq1 = chunk1.cmpeq(rare2chunk);

    let mut match_offsets = eq0.and(eq1).movemask() & mask;
    while match_offsets != 0 {
        let offset = match_offsets.trailing_zeros() as usize;
        let ptr = ptr.add(offset);
        if end_ptr.sub(needle.len()) < ptr {
            return None;
        }
        let chunk = core::slice::from_raw_parts(ptr, needle.len());
        if memcmp(needle, chunk) {
            return Some(offset);
        }
        match_offsets &= match_offsets - 1;
    }
    None
}

/// Accepts a chunk-relative offset and returns a haystack relative offset
/// after updating the prefilter state.
///
/// See the same function with the same name in the prefilter variant of this
/// algorithm to learned why it's tagged with inline(never). Even here, where
/// the function is simpler, inlining it leads to poorer codegen. (Although
/// it does improve some benchmarks, like prebuiltiter/huge-en/common-you.)
#[cold]
#[inline(never)]
fn matched(start_ptr: *const u8, ptr: *const u8, chunki: usize) -> usize {
    diff(ptr, start_ptr) + chunki
}

/// Subtract `b` from `a` and return the difference. `a` must be greater than
/// or equal to `b`.
fn diff(a: *const u8, b: *const u8) -> usize {
    debug_assert!(a >= b);
    (a as usize) - (b as usize)
}
