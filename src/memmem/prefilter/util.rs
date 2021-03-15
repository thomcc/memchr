#[inline(always)]
pub fn is_prefix(haystack: &[u8], needle: &[u8]) -> bool {
    needle.len() <= haystack.len() && memcmp(&haystack[..needle.len()], needle)
}

#[inline(always)]
pub fn memcmp(x: &[u8], y: &[u8]) -> bool {
    // Why not just use actual memcmp for this? Well, memcmp requires calling
    // out to libc, and this routine is called in fairly hot code paths. Other
    // than just calling out to libc, it also seems to result in worse codegen.
    // By rolling our own memcmp in pure Rust, it seems to appear more friendly
    // to the optimizer.

    if x.len() != y.len() {
        return false;
    }
    if x.len() < 8 {
        for (&b1, &b2) in x.iter().zip(y) {
            if b1 != b2 {
                return false;
            }
        }
        return true;
    }
    // When we have 8 or more bytes to compare, then proceed in chunks of
    // 8 at a time using unaligned loads.
    let mut px = x.as_ptr();
    let mut py = y.as_ptr();
    let pxend = x[x.len() - 8..].as_ptr();
    let pyend = y[y.len() - 8..].as_ptr();
    // SAFETY: Via the conditional above, we know that both `p1` and `p2`
    // have the same length, so `p1 < p1end` implies that `p2 < p2end`.
    // Thus, derefencing both `p1` and `p2` in the loop below is safe.
    //
    // Moreover, we set `p1end` and `p2end` to be 8 bytes before the actual
    // end of of `p1` and `p2`. Thus, the final dereference outside of the
    // loop is guaranteed to be valid.
    //
    // Finally, we needn't worry about 64-bit alignment here, since we
    // do unaligned loads.
    unsafe {
        while px < pxend {
            let vx = (px as *const u64).read_unaligned();
            let vy = (py as *const u64).read_unaligned();
            if vx != vy {
                return false;
            }
            px = px.add(8);
            py = py.add(8);
        }
        let vx = (pxend as *const u64).read_unaligned();
        let vy = (pyend as *const u64).read_unaligned();
        vx == vy
    }
}
