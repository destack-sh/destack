use destack_source::PackageId;

/// Unique identifier for a build target within a package.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TargetId {
    /// The package id.
    pub package_id: PackageId,
    /// The name of the target.
    /// NOTE #Performance: TargetId.name should be interned (but where? no obvious)
    pub name: String,
}

impl std::fmt::Debug for TargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.package_id, self.name)
    }
}

impl std::fmt::Display for TargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl TargetId {
    /// Create a new TargetId.
    pub fn new(package_id: PackageId, name: impl Into<String>) -> Self {
        Self {
            package_id,
            name: name.into(),
        }
    }
}
