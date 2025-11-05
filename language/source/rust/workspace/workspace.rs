use std::collections::HashMap;

use crate::{
    Diagnostic, File, FileMode, LanguageOptions, Package, PackageId, Session, SourceId, Uri,
};

/// A workspace of Packages with Files in a shared Session.
/// Packages are used for dependency management and comprise logical modules.
/// (Modules are tracked at the AST-level, even for file/directory scoped modules).
#[derive(Debug)]
pub struct Workspace {
    /// The root URI of the workspace.
    pub root_uri: Uri,
    /// The shared session for the workspace.
    pub session: Session,

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
        // orphan package (for working with out-of-package sources)
        let orphan_package_id = PackageId::new(0);
        let orphan_package_uri = Uri::from_string("<orphan>");
        let orphan_package = Package::new(
            orphan_package_id,
            "<orphan>".to_string(),
            orphan_package_uri.clone(),
            SourceId::new(0),
            LanguageOptions::default(),
        );

        // create workspace
        let mut workspace = Self {
            root_uri,
            session: Session::new(),

            next_source_id: 1,
            next_package_id: 1,

            orphan_package_id,
            main_package_id: None,
            packages_by_id: HashMap::new(),
            files_by_uri: HashMap::new(),
            files_by_id: HashMap::new(),
        };

        // insert orphan package
        workspace.insert_package(orphan_package);
        workspace
    }

    /// Get or create a source ID for a URI.
    fn get_or_create_source_id(&mut self, uri: &Uri) -> SourceId {
        self.files_by_uri
            .get(uri)
            .map(|doc| doc.id)
            .unwrap_or_else(|| {
                let id = SourceId::new(self.next_source_id);
                self.next_source_id = self.next_source_id.wrapping_add(1);
                id
            })
    }

    /// Get the package containing a source URI (default to orphan package).
    pub fn get_package_containing_uri(&self, uri: &Uri) -> Option<&'_ Package> {
        self.packages_by_id
            .values()
            .find(|pkg| pkg.is_parent_of(uri))
    }

    /// Get the package containing a source URI mut (default to orphan package).
    pub fn get_package_containing_uri_mut(&mut self, uri: &Uri) -> Option<&mut Package> {
        self.packages_by_id
            .values_mut()
            .find(|pkg| pkg.is_parent_of(uri))
    }

    /// Get the package containing a source ID.
    pub fn get_package_containing_source_id(&self, source_id: SourceId) -> Option<&'_ Package> {
        self.packages_by_id
            .values()
            .find(|pkg| pkg.sources.contains(&source_id))
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

    /// Get all diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.session.diagnostics
    }

    /// Get diagnostics for a source.
    pub fn get_diagnostics_for_source(&self, source: SourceId) -> Vec<Diagnostic> {
        self.session.get_diagnostics_for_source(source)
    }

    /// Get diagnostics for a predicate.
    pub fn get_diagnostics_for_predicate(
        &self,
        predicate: impl Fn(&Diagnostic) -> bool,
    ) -> Vec<Diagnostic> {
        self.session.get_diagnostics_for_predicate(predicate)
    }

    /// Reset all diagnostics.
    pub fn reset_diagnostics(&mut self) {
        self.session.reset_diagnostics();
    }

    /// Reset diagnostics for a source.
    pub fn reset_diagnostics_for_source(&mut self, source: SourceId) {
        self.session.reset_diagnostics_for_source(source);
    }

    /// Get a file by its ID.
    pub fn get_file_by_source_id(&self, id: SourceId) -> Option<&File> {
        self.files_by_id
            .get(&id)
            .and_then(|uri| self.files_by_uri.get(uri))
    }
}
