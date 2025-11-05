use crate::{Dependency, LanguageOptions, SourceId, Uri};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackageId(pub u32);

impl PackageId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A Package is a collection of sources in a Workspace.
#[derive(Debug, Clone)]
pub struct Package {
    /// The ID of the package.
    pub id: PackageId,
    /// The name of the package.
    pub name: String,
    /// The root URI of the package.
    pub root_uri: Uri,
    /// The language options of the package.
    pub language: LanguageOptions,
    /// The dependencies of the package.
    pub dependencies: Vec<Dependency>,
    /// The sources in the package.
    pub sources: Vec<SourceId>,
}

impl Package {
    
}
