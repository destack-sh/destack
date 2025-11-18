use std::hash::Hasher;

/// Simple identity hasher to avoid double caching.
#[derive(Debug, Default)]
pub(crate) struct IdentityHasher(u64);

impl Hasher for IdentityHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("invalid use of IdentityHasher")
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
