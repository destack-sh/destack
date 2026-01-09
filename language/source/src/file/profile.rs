/// Unique identifier for profiles.
///
/// A profile represents a semantic configuration (comptime world) that determines
/// which symbols exist and how types resolve. Multiple targets can share the same
/// profile, allowing them to share canonical DIR.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProfileId(pub u32);

impl std::fmt::Debug for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl ProfileId {
    /// Wrap an id as a ProfileId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw id value.
    pub fn raw(&self) -> u32 {
        self.0
    }
}

/// Version of a profile's compiled state (increments on recomputation).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ProfileVersion(pub u64);

impl std::fmt::Debug for ProfileVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl std::fmt::Display for ProfileVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl ProfileVersion {
    /// Initial version.
    pub const INITIAL: Self = Self(0);

    /// Create a new ProfileVersion.
    pub fn new(version: u64) -> Self {
        Self(version)
    }

    /// Increment the version, returning the new value.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
