use std::collections::{HashMap, HashSet};
use std::{fs, io};

use destack_file::glob::glob;
use dyst_ast::NodeTree;
use dyst_diagnostic::Diagnostic;
use dyst_session::Session;
use dyst_source::{SourceFormat, SourceId, Uri};

use crate::{
    File, FileContent, FileIntent, PACKAGE_FILE_NAME, Package, PackageId, SourceFile,
    infer_source_format_from_uri,
};

pub const TRACKED_FORMATS: [SourceFormat; 4] = [
    SourceFormat::Dyst,
    SourceFormat::DystText,
    SourceFormat::DystBinary,
    SourceFormat::DystExecutable,
];

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

    /// Load workspace from disk.
    pub fn load(root_uri: Uri) -> io::Result<Self> {
        let mut workspace = Self::empty(root_uri);
        workspace.reload_from_disk()?;
        Ok(workspace)
    }

    /// Find the containing workspace package for a source URI (if any).
    /// Scans the file system upwards looking for a containing package root.
    pub fn load_containing(uri: &Uri) -> io::Result<Option<Self>> {
        // find containing root
        let root_uri = {
            // loop until we find the `package.dst` file in the directory
            let mut current_path = uri.to_file_path();
            let mut root_uri = None;
            while let Some(current_dir) = current_path {
                if current_dir.is_dir() {
                    let package_file = current_dir.join(PACKAGE_FILE_NAME);
                    if package_file.exists() {
                        let root_dir = current_dir.canonicalize()?;
                        root_uri = Some(Uri::from_file_path(root_dir));
                        break;
                    }
                }
                current_path = current_dir.parent();
            }
            root_uri
        };

        // load workspace
        if let Some(root_uri) = root_uri {
            let workspace = Self::load(root_uri)?;
            Ok(Some(workspace))
        } else {
            Ok(None)
        }
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

    /// Get a file's AST by its ID.
    pub fn get_ast_by_source_id(&self, id: SourceId) -> Option<&NodeTree> {
        self.get_file_by_source_id(id)
            .and_then(|file| match &file.content {
                FileContent::Source(SourceFile { ast, .. }) => Some(ast),
                _ => None,
            })
    }

    /// Upsert any file into the workspace.
    pub fn upsert_file(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: Vec<u8>,
    ) -> SourceId {
        match format {
            SourceFormat::Dyst | SourceFormat::DystText => {
                let content = String::from_utf8_lossy(&content).to_string();
                self.upsert_text_file(uri, format, is_open, content)
            }
            SourceFormat::DystBinary | SourceFormat::DystExecutable => {
                self.upsert_binary_file(uri, format, is_open, content)
            }
        }
    }

    /// Upsert and parse a file into the workspace.
    pub fn upsert_text_file(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: String,
    ) -> SourceId {
        // get id
        let source_id = self.get_or_create_source_id(uri);
        let package_id = self
            .get_package_containing_uri(uri)
            .map(|pkg| pkg.id)
            .unwrap_or(self.orphan_package_id);
        let package = self
            .packages_by_id
            .get_mut(&package_id)
            .unwrap_or_else(|| panic!("package not found: {package_id:?}"));
        package.add_source(source_id);

        // reset diagnostics
        self.reset_diagnostics_for_source(source_id);

        // create file
        let name = uri.last_segment().unwrap_or("<file>").to_string();
        let filoe = File::parse_text(
            source_id,
            package_id,
            name,
            uri.clone(),
            format,
            is_open,
            content,
            &mut self.session,
        );
        self.files_by_uri.insert(uri.clone(), filoe);
        self.files_by_id.insert(source_id, uri.clone());

        source_id
    }

    /// Upsert and parse a binary file into the workspace.
    pub fn upsert_binary_file(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: Vec<u8>,
    ) -> SourceId {
        // get id
        let source_id = self.get_or_create_source_id(uri);
        let package_id = self
            .get_package_containing_uri(uri)
            .map(|pkg| pkg.id)
            .unwrap_or(self.orphan_package_id);
        let package = self
            .packages_by_id
            .get_mut(&package_id)
            .unwrap_or_else(|| panic!("package not found: {package_id:?}"));
        package.add_source(source_id);

        // create file
        let name = uri.last_segment().unwrap_or("<file>").to_string();
        let file = File::wrap_binary(
            source_id,
            package_id,
            name,
            uri.clone(),
            format,
            is_open,
            content,
        );
        self.files_by_uri.insert(uri.clone(), file);
        self.files_by_id.insert(source_id, uri.clone());

        source_id
    }

    /// Remove a file from the workspace.
    pub fn remove_file(&mut self, uri: &Uri) {
        if let Some(file) = self.files_by_uri.remove(uri) {
            self.reset_diagnostics_for_source(file.id);
            self.files_by_id.remove(&file.id);
            let package = self.packages_by_id.get_mut(&file.package_id);
            if let Some(package) = package {
                package.remove_source(file.id);
            }
        }
    }

    /// Refresh a single file from disk when it is not open.
    pub fn reload_file_from_disk(&mut self, uri: &Uri) -> io::Result<()> {
        // infer the format
        let format = infer_source_format_from_uri(uri).unwrap_or(SourceFormat::Dyst);

        // read the file from disk
        let path = uri
            .to_file_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "uri is not a file path"))?
            .to_path_buf();
        let content = fs::read(path)?;

        // upsert the file
        self.upsert_file(uri, format, false, content);

        Ok(())
    }

    /// Rebuild the workspace state from disk for all sources.
    pub fn reload_from_disk(&mut self) -> io::Result<WorkspaceReindex> {
        // build pattern
        let root_uri = &self.root_uri;
        let root_path = root_uri
            .to_file_path()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "workspace root is not a file path",
                )
            })?
            .to_string_lossy()
            .into_owned();

        // collect from workspace tree
        let mut seen_uris = HashSet::new();
        let mut index = WorkspaceReindex::default();
        for format in TRACKED_FORMATS {
            let glob_pattern = format!("{}/{}", root_path, format.glob());
            let paths = glob(&glob_pattern);
            for path in &paths {
                let uri = Uri::from_file_path(path);
                seen_uris.insert(uri.clone());

                // read the file from disk
                let content = match fs::read(path) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                // upsert the file
                self.upsert_file(&uri, format, false, content);
                index.updated.push(uri.clone());
            }
        }

        // drop files that disappeared from disk
        let stale_uris: Vec<Uri> = self
            .files_by_uri
            .iter()
            .filter(|(uri, file)| !file.is_open && !seen_uris.contains(uri.as_ref()))
            .map(|(uri, _)| uri.clone())
            .collect();
        for uri in stale_uris {
            self.remove_file(&uri);
            index.removed.push(uri);
        }

        // re-index packages
        self.reindex_packages();

        Ok(index)
    }

    /// Reindex packages and their associated files.
    /// Does not reload from disk, just checks and assigns files to packages.
    pub fn reindex_packages(&mut self) {
        // re-index packages
        let orphan_package = self
            .packages_by_id
            .remove(&self.orphan_package_id)
            .expect("orphan package not found");
        self.packages_by_id.clear();
        self.packages_by_id
            .insert(self.orphan_package_id, orphan_package);

        // create packages for missing manifests
        for file in self.files_by_uri.values_mut() {
            if file.intent == FileIntent::Package
                && !self.packages_by_id.contains_key(&file.package_id)
            {
                let uri = file.uri.clone();
                let package_id = PackageId::new(self.next_package_id);
                file.package_id = package_id;
                self.next_package_id += 1;
                let package = Package::new(
                    package_id,
                    // NOTE @Broken: read package name from package manifest file?
                    file.name.clone(),
                    uri,
                    file.id,
                );
                self.packages_by_id.insert(package.id, package);
            }
        }

        // assign files to packages
        let mut package_id_by_url = HashMap::new();
        for package in self.packages_by_id.values() {
            package_id_by_url.insert(package.root_uri.clone(), package.id);
        }
        for file in self.files_by_uri.values_mut() {
            let package = self
                .packages_by_id
                .values_mut()
                .find(|pkg| pkg.is_parent_of(&file.uri));
            if let Some(package) = package {
                file.package_id = package.id;
                package.add_source(file.id);
            } else {
                file.package_id = self.orphan_package_id;
            }
        }
    }
}

/// The result of a workspace reindex.
#[derive(Debug, Default)]
pub struct WorkspaceReindex {
    /// URIs whose content was refreshed from disk.
    pub updated: Vec<Uri>,
    /// URIs removed from the workspace because the backing file no longer exists.
    pub removed: Vec<Uri>,
}
