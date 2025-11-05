use std::collections::HashMap;

/// An unresolved dependency to a package with a "virtual" name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dependency {
    /// The name of the package.
    pub name: String,
    /// The version of the package.
    pub version: DependencyVersion,
}

/// The version of a dependency.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DependencyVersion {
    Exact(String),
}

/// An index of dependencies.
#[derive(Debug, Clone)]
pub struct DependencyIndex {
    /// The dependencies by name.
    pub dependencies_by_name: HashMap<&'static str, DependencyVersion>,
}
