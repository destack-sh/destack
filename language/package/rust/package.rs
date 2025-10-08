use dyst_source::Uri;

pub const PACKAGE_FILE_NAME: &str = "package.dst";

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackageId(pub u32);

impl PackageId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A Package is a collection of sources.
#[derive(Debug, Clone)]
pub struct Package {
    /// The ID of the package.
    pub id: PackageId,
    /// The name of the package.
    pub name: String,
    /// The root URI of the package.
    pub root: Uri,
    /// The dependencies of the package.
    pub dependencies: Vec<PackageId>,
}

impl Package {
    /// Create an empty package.
    pub fn empty(id: PackageId, name: String, root: Uri) -> Self {
        Self {
            id,
            name,
            root,
            dependencies: Vec::new(),
        }
    }

    /// Check if a URI is contained in the package.
    pub fn is_parent_of(&self, uri: &Uri) -> bool {
        uri.starts_with(&self.root) // is that it? 
    }
}
