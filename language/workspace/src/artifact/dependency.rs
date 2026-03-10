/// Dependency stamp captured for one artifact build.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ArtifactDependency(pub u64);

impl std::fmt::Debug for ArtifactDependency {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d{:016x}", self.0)
    }
}

impl std::fmt::Display for ArtifactDependency {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d{:016x}", self.0)
    }
}

impl ArtifactDependency {
    /// Create a new artifact dependency.
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}
