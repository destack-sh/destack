use std::collections::{HashMap, HashSet};
use std::{fs, io};

use destack_file::glob::glob;
use dyst_diagnostic::Diagnostic;
use dyst_session::Session;
use dyst_source::{SourceFormat, SourceId, Uri};

use crate::{Document, PACKAGE_FILE_NAME, Package, PackageId, infer_source_format_from_uri};

pub const TRACKED_FORMATS: [SourceFormat; 4] = [
    SourceFormat::Dyst,
    SourceFormat::DystText,
    SourceFormat::DystBinary,
    SourceFormat::DystExecutable,
];

/// A workspace of Packages with Documents in a shared Session.
#[derive(Debug)]
pub struct Workspace {
    /// The root URI of the workspace.
    pub root: Uri,

    /// The next package ID to use for a new package.
    next_package_id: u32,
    /// The next source ID to use for a new document.
    next_source_id: u32,
    /// The package ID of the "orphan" package.
    orphan_package_id: PackageId,
    /// All the packages.
    packages: HashMap<Uri, Package>,
    /// All the documents.
    documents: HashMap<Uri, Document>,
    /// The shared session for the workspace.
    pub session: Session,
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
        Self {
            root: root_uri,
            next_source_id: 1,
            next_package_id: 1,
            orphan_package_id: PackageId::new(0),
            packages: HashMap::new(),
            session: Session::new(),
            documents: HashMap::new(),
        }
    }

    /// Load workspace from disk.
    pub fn load(root_uri: Uri) -> io::Result<Self> {
        let mut workspace = Self::empty(root_uri);
        workspace.reload_all_documents_from_disk()?;
        Ok(workspace)
    }

    /// Find the containing workspace package for a source URI (if any).
    /// Scans the file system upwards looking for a containing package root.
    pub fn load_containing(uri: &Uri) -> io::Result<Option<Self>> {
        // find containing root
        let root_uri = {
            // loop until we find the `package.dst` file in the directory
            let mut current = uri.to_file_path();
            let mut root_uri = None;
            while let Some(current_path) = current {
                if current_path.is_dir() {
                    let package_file = current_path.join(PACKAGE_FILE_NAME);
                    if package_file.exists() {
                        root_uri = Some(Uri::from_file_path(current_path));
                        break;
                    }
                } else {
                    current = current_path.parent();
                }
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
        self.documents
            .get(uri)
            .map(|doc| doc.id)
            .unwrap_or_else(|| {
                let id = SourceId::new(self.next_source_id);
                self.next_source_id = self.next_source_id.wrapping_add(1);
                id
            })
    }

    /// Get or create a package ID for a URI.
    fn get_or_create_package_id(&mut self, uri: &Uri) -> PackageId {
        self.packages.get(uri).map(|pkg| pkg.id).unwrap_or_else(|| {
            let id = PackageId::new(self.next_package_id);
            self.next_package_id = self.next_package_id.wrapping_add(1);
            id
        })
    }

    /// Get the containing package ID for a URI.
    fn get_containing_package_id(&self, uri: &Uri) -> Option<PackageId> {
        self.packages.values().find_map(|pkg| {
            if pkg.is_parent_of(uri) {
                Some(pkg.id)
            } else {
                None
            }
        })
    }

    /// Get the containing package for a URI (default to orphan package).
    fn get_containing_package_id_or_orphan(&self, uri: &Uri) -> PackageId {
        self.get_containing_package_id(uri)
            .unwrap_or(self.orphan_package_id)
    }

    /// Check if a document exists in the workspace.
    pub fn has_document(&self, uri: &Uri) -> bool {
        self.documents.contains_key(uri)
    }

    /// Check if the workspace has a document and it is open.
    pub fn has_open_document(&self, uri: &Uri) -> bool {
        self.documents
            .get(uri)
            .map(|doc| doc.is_open)
            .unwrap_or(false)
    }

    /// Get a document from the workspace.
    pub fn get_document(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// Collect URIs for all tracked documents.
    pub fn document_uris(&self) -> Vec<Uri> {
        self.documents.keys().cloned().collect()
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

    /// Upsert any document into the workspace.
    pub fn upsert_document(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: Vec<u8>,
    ) -> SourceId {
        match format {
            SourceFormat::Dyst | SourceFormat::DystText => {
                let content = String::from_utf8_lossy(&content).to_string();
                self.upsert_text_document(uri, format, is_open, content)
            }
            SourceFormat::DystBinary | SourceFormat::DystExecutable => {
                self.upsert_binary_document(uri, format, is_open, content)
            }
        }
    }

    /// Upsert and parse a document into the workspace.
    pub fn upsert_text_document(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: String,
    ) -> SourceId {
        // get id
        let source_id = self.get_or_create_source_id(uri);
        let package_id = self.get_containing_package_id_or_orphan(uri);

        // reset diagnostics
        self.reset_diagnostics_for_source(source_id);

        // create document
        let name = uri.last_segment().unwrap_or("<file>").to_string();
        let document = Document::parse_text(
            source_id,
            package_id,
            name,
            uri.clone(),
            format,
            is_open,
            content,
            &mut self.session,
        );
        self.documents.insert(uri.clone(), document);

        source_id
    }

    /// Upsert and parse a binary document into the workspace.
    pub fn upsert_binary_document(
        &mut self,
        uri: &Uri,
        format: SourceFormat,
        is_open: bool,
        content: Vec<u8>,
    ) -> SourceId {
        // get id
        let source_id = self.get_or_create_source_id(uri);
        let package_id = self.get_containing_package_id_or_orphan(uri);

        // create document
        let name = uri.last_segment().unwrap_or("<file>").to_string();
        let document = Document::wrap_binary(
            source_id,
            package_id,
            name,
            uri.clone(),
            format,
            is_open,
            content,
        );
        self.documents.insert(uri.clone(), document);

        source_id
    }

    /// Remove a document from the workspace.
    pub fn remove_document(&mut self, uri: &Uri) {
        if let Some(document) = self.documents.remove(uri) {
            self.reset_diagnostics_for_source(document.id);
        }
    }

    /// Refresh a single document from disk when it is not open.
    pub fn reload_document_from_disk(&mut self, uri: &Uri) -> io::Result<()> {
        // infer the format
        let format = infer_source_format_from_uri(uri).unwrap_or(SourceFormat::Dyst);

        // read the document from disk
        let path = uri
            .to_file_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "uri is not a file path"))?
            .to_path_buf();
        let content = fs::read(path)?;

        // upsert the document
        self.upsert_document(uri, format, false, content);

        Ok(())
    }

    /// Rebuild the workspace state from disk for all sources.
    pub fn reload_all_documents_from_disk(&mut self) -> io::Result<WorkspaceReindex> {
        // build pattern
        let root_uri = &self.root;
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

                // read the document from disk
                let content = match fs::read(path) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                // upsert the document
                self.upsert_document(&uri, format, false, content);
                index.updated.push(uri.clone());
            }
        }

        // drop documents that disappeared from disk
        let stale_uris: Vec<Uri> = self
            .documents
            .iter()
            .filter(|(uri, document)| !document.is_open && !seen_uris.contains(uri.as_ref()))
            .map(|(uri, _)| uri.clone())
            .collect();
        for uri in stale_uris {
            self.remove_document(&uri);
            index.removed.push(uri);
        }

        // re-index packages from new documents
        todo!("re-index packages from new documents (and assign them)");

        Ok(index)
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
