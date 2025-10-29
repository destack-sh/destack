use dyst_source::{LanguageOptions, SourceId, Uri};

use crate::Dependency;

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
    /// The manifest of the package.
    pub manifest: SourceId,
    /// The language options of the package.
    pub language: LanguageOptions,
    /// The dependencies of the package.
    pub dependencies: Vec<Dependency>,
    /// The sources in the package.
    pub sources: Vec<SourceId>,
}

impl Package {
    /// Create an empty package.
    pub fn new(
        id: PackageId,
        name: String,
        root: Uri,
        manifest: SourceId,
        language: LanguageOptions,
    ) -> Self {
        Self {
            id,
            name,
            root_uri: root,
            manifest,
            language,
            dependencies: Vec::new(),
            sources: Vec::new(),
        }
    }

    /// Add a source to the package.
    pub fn add_source(&mut self, source: SourceId) {
        self.sources.push(source);
    }

    /// Remove a source from the package.
    pub fn remove_source(&mut self, source: SourceId) {
        self.sources.retain(|s| *s != source);
    }

    /// Check if a URI is contained in the package.
    pub fn is_parent_of(&self, uri: &Uri) -> bool {
        uri.starts_with(&self.root_uri) // is that it? 
    }
}
