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
pub enum DependencyVersion {}

/// An index of dependencies.
pub struct DependencyIndex {
    /// The dependencies.
    pub dependencies: HashMap<String, DependencyVersion>,
}
