/// Digest of a published semantic artifact.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ArtifactDigest(pub u64);

impl std::fmt::Debug for ArtifactDigest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a{:016x}", self.0)
    }
}

impl std::fmt::Display for ArtifactDigest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a{:016x}", self.0)
    }
}

impl ArtifactDigest {
    /// Create a new artifact digest.
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}
