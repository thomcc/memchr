/// Search for the first occurrence of needle in haystack using Rabin-Karp.
pub(crate) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    find_with(&NeedleHash::new(needle), haystack, needle)
}

/// Search for the first occurrence of needle in haystack using Rabin-Karp with
/// a pre-computed needle hash.
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
    // N.B. I've experimented with unrolling this loop, but couldn't realize
    // any obvious gains.
    loop {
        if nhash.eq(hash) && is_prefix(haystack, needle) {
            return Some(haystack.as_ptr() as usize - start);
        }
        if needle.len() >= haystack.len() {
            return None;
        }
        hash.roll(&nhash, haystack[0], haystack[needle.len()]);
        haystack = &haystack[1..];
    }
}

/// A hash derived from a needle.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NeedleHash {
    /// The actual hash.
    hash: Hash,
    /// The factor needed to multiply a byte by in order to subtract it from
    /// the hash. It is defined to be 2^(n-1) (using wrapping exponentiation),
    /// where n is the length of the needle. This is how we "remove" a byte
    /// from the hash once the hash window rolls past it.
    hash_2pow: u32,
}

impl NeedleHash {
    /// Create a new Rabin-Karp hash for the given needle.
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

    /// Return true if the hashes are equivalent.
    fn eq(&self, hash: Hash) -> bool {
        self.hash == hash
    }
}

/// A Rabin-Karp hash. This might represent the hash of a needle, or the hash
/// of a rolling window in the haystack.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Hash(u32);

impl Hash {
    /// Create a new hash that represents the empty string.
    pub(crate) fn new() -> Hash {
        Hash(0)
    }

    /// Create a new hash by hashing the bytes given.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Hash {
        let mut hash = Hash::new();
        for &b in bytes {
            hash.add(b);
        }
        hash
    }

    /// Add 'new' and remove 'old' from this hash. The given needle hash should
    /// correspond to the hash computed for the needle being searched for.
    ///
    /// This is meant to be used when the rolling window of the haystack is
    /// advanced.
    fn roll(&mut self, nhash: &NeedleHash, old: u8, new: u8) {
        self.del(nhash, old);
        self.add(new);
    }

    /// Add a byte to this hash.
    fn add(&mut self, byte: u8) {
        self.0 = self.0.wrapping_shl(1).wrapping_add(byte as u32);
    }

    /// Remove a byte from this hash. The given needle hash should correspond
    /// to the hash computed for the needle being searched for.
    fn del(&mut self, nhash: &NeedleHash, byte: u8) {
        let factor = nhash.hash_2pow;
        self.0 = self.0.wrapping_sub((byte as u32).wrapping_mul(factor));
    }
}

/// Returns true if the given needle is a prefix of the given haystack.
///
/// We forcefully don't inline the is_prefix call and hint at the compiler that
/// it is unlikely to be called. This causes the inner rabinkarp loop above
/// to be a bit tighter and leads to some performance improvement. See the
/// memmem/krate/prebuilt/sliceslice-words/words benchmark.
#[cold]
#[inline(never)]
fn is_prefix(haystack: &[u8], needle: &[u8]) -> bool {
    crate::memmem::util::is_prefix(haystack, needle)
}

/// Returns true if the given needle is a suffix of the given haystack.
///
/// See is_prefix for why this is forcefully not inlined.
#[cold]
#[inline(never)]
fn is_suffix(haystack: &[u8], needle: &[u8]) -> bool {
    crate::memmem::util::is_suffix(haystack, needle)
}
