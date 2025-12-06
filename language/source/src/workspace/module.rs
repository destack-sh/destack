// nocheckin: make ModuleId/ProgramId/PackageId persistent hashes (based on path?)

/// Unique identifier for Modules.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

impl std::fmt::Debug for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl ModuleId {
    /// Wrap an id as a ModuleId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}
