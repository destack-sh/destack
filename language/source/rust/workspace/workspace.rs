use std::collections::HashMap;

use crate::{File, FileMode, Package, PackageId, SourceId, Uri};

/// A workspace of Packages with Files in a shared Session.
/// Packages are used for dependency management and comprise logical modules.
/// (Modules are tracked at the AST-level, even for file/directory scoped modules).
#[derive(Debug)]
pub struct Workspace {
    /// The root URI of the workspace.
    pub root_uri: Uri,

    /// The next package ID to use for a new package.
    next_package_id: u32,
    /// The next source ID to use for a new file.
    next_source_id: u32,

    /// The package ID of the "orphan" package.
    orphan_package_id: PackageId,
    /// The ID of the main package.
    main_package_id: Option<PackageId>,
    /// All the packages.
    packages_by_id: HashMap<PackageId, Package>,
    /// All the files.
    files_by_uri: HashMap<Uri, File>,
    /// Index files by their ID.
    files_by_id: HashMap<SourceId, Uri>,
}

#[cfg(test)]
impl Default for Workspace {
    fn default() -> Self {
        Self::empty(Uri::from_string("test"))
    }
}

impl Workspace {
    /// Create a new empty workspace.
    pub fn empty(root_uri: Uri) -> Self {
        todo!()
    }

    /// Get the package containing a source ID mut (default to orphan package).
    pub fn get_package_containing_source_id_mut(
        &mut self,
        source_id: SourceId,
    ) -> Option<&mut Package> {
        self.packages_by_id
            .values_mut()
            .find(|pkg| pkg.sources.contains(&source_id))
    }

    /// Get the package ID for the main package.
    pub fn get_main_package_id(&self) -> PackageId {
        self.main_package_id.unwrap_or(self.orphan_package_id)
    }

    /// Get a package by its ID.
    pub fn get_package_by_id(&self, id: PackageId) -> &'_ Package {
        self.packages_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("package not found: {id:?} in {self:?}"))
    }

    /// Insert a package into the workspace.
    pub fn insert_package(&mut self, package: Package) {
        self.packages_by_id.insert(package.id, package);
    }

    /// Check if a file exists in the workspace.
    pub fn has_file(&self, uri: &Uri) -> bool {
        self.files_by_uri.contains_key(uri)
    }

    /// Check if the workspace has a file and it is open.
    pub fn has_open_file(&self, uri: &Uri) -> bool {
        self.files_by_uri
            .get(uri)
            .map(|doc| doc.is_open)
            .unwrap_or(false)
    }

    /// Get a file from the workspace.
    pub fn get_file(&self, uri: &Uri) -> Option<&File> {
        self.files_by_uri.get(uri)
    }

    /// Get all files from the workspace.
    pub fn files(&self) -> impl Iterator<Item = &File> {
        self.files_by_uri.values()
    }

    /// Collect URIs for all tracked files.
    pub fn files_uris(&self) -> Vec<Uri> {
        self.files_by_uri.keys().cloned().collect()
    }

    /// Get a file by its ID.
    pub fn get_file_by_source_id(&self, id: SourceId) -> Option<&File> {
        self.files_by_id
            .get(&id)
            .and_then(|uri| self.files_by_uri.get(uri))
    }
}
