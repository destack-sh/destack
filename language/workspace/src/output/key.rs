use destack_source::{ModuleId, PackageId};

use crate::TargetId;

/// Scope of a generated output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputScope {
    /// Output for a single module.
    Module(ModuleId),
    /// Output for an entire package.
    Package(PackageId),
}

impl OutputScope {
    /// Return the module id when this is a module output.
    pub fn module(&self) -> Option<ModuleId> {
        match self {
            Self::Module(id) => Some(*id),
            Self::Package(_) => None,
        }
    }

    /// Return the package id when this is a package output.
    pub fn package(&self) -> Option<PackageId> {
        match self {
            Self::Module(_) => None,
            Self::Package(id) => Some(*id),
        }
    }
}

/// Lookup key for outputs by scope and target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputKey {
    /// The output scope.
    pub scope: OutputScope,
    /// The target id.
    pub target: TargetId,
}

impl OutputKey {
    /// Create a new output key.
    pub fn new(scope: OutputScope, target: TargetId) -> Self {
        Self { scope, target }
    }

    /// Create a module output key.
    pub fn module(module: ModuleId, target: TargetId) -> Self {
        Self::new(OutputScope::Module(module), target)
    }

    /// Create a package output key.
    pub fn package(package: PackageId, target: TargetId) -> Self {
        Self::new(OutputScope::Package(package), target)
    }
}

/// Version of an output's generated content.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct OutputVersion(pub u64);

impl std::fmt::Debug for OutputVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "v{}", self.0)
    }
}

impl std::fmt::Display for OutputVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "v{}", self.0)
    }
}

impl OutputVersion {
    /// The initial output version.
    pub const INITIAL: Self = Self(0);

    /// Create a new output version.
    pub fn new(version: u64) -> Self {
        Self(version)
    }

    /// Increment the version, returning the new value.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// The id of an output.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(pub u32);

impl std::fmt::Debug for OutputId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

impl std::fmt::Display for OutputId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

impl OutputId {
    /// Create a new output id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}
