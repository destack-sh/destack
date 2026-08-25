use std::hash::Hasher;

/// One canonical cross-module type identity: structural bytes with extents erased.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TypeFingerprint(Vec<u8>);

impl Hasher for TypeFingerprint {
    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }

    fn finish(&self) -> u64 {
        unreachable!("fingerprints compare by bytes")
    }
}
