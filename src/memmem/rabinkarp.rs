#![allow(warnings)]

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NeedleHash {
    hash: Hash,
    hash_2pow: u32,
}

impl NeedleHash {
    pub(crate) fn new(needle: &[u8]) -> NeedleHash {
        let mut nh = NeedleHash { hash: Hash::new(), hash_2pow: 1 };
        if needle.is_empty() {
            return nh;
        }
        nh.hash.add(needle[0]);
        for &b in needle.iter().skip(1) {
            nh.hash.add(b);
            nh.hash_2pow = nh.hash_2pow.wrapping_shl(1);
        }
        nh
    }

    fn eq(&self, hash: Hash) -> bool {
        self.hash == hash
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Hash(u32);

impl Hash {
    pub(crate) fn new() -> Hash {
        Hash(0)
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Hash {
        let mut hash = Hash::new();
        for &b in bytes {
            hash.add(b);
        }
        hash
    }

    pub(crate) fn add(&mut self, byte: u8) {
        self.0 = self.0.wrapping_shl(1).wrapping_add(byte as u32);
        // self.0 = self.0.wrapping_add(byte as u32);
    }

    pub(crate) fn del(&mut self, nhash: &NeedleHash, byte: u8) {
        let factor = nhash.hash_2pow;
        self.0 = self.0.wrapping_sub((byte as u32).wrapping_mul(factor));
        // self.0 = self.0.wrapping_sub(byte as u32);
    }
}

pub(crate) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    find_with(&NeedleHash::new(needle), haystack, needle)
}

pub(crate) fn find_with(
    nhash: &NeedleHash,
    mut haystack: &[u8],
    needle: &[u8],
) -> Option<usize> {
    if haystack.len() < needle.len() {
        return None;
    }
    let start = haystack.as_ptr() as usize;
    let mut hash = Hash::from_bytes(&haystack[..needle.len()]);
    loop {
        if nhash.eq(hash) && is_prefix(haystack, needle) {
            return Some(haystack.as_ptr() as usize - start);
        }
        if needle.len() >= haystack.len() {
            return None;
        }
        hash.del(&nhash, haystack[0]);
        hash.add(haystack[needle.len()]);
        haystack = &haystack[1..];
    }
}

// We forcefully don't inline the is_prefix call and hint at the compiler that
// it is unlikely to be called. This causes the inner rabinkarp loop above
// to be a bit tighter and leads to some performance improvement. See the
// memmem/krate/prebuilt/sliceslice-words/words benchmark.
#[cold]
#[inline(never)]
fn is_prefix(haystack: &[u8], needle: &[u8]) -> bool {
    crate::memmem::util::is_prefix(haystack, needle)
}
