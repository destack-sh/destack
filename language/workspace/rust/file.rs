use crate::{TRACKED_FORMATS, Workspace};
use destack_file::glob::glob;
use dyst_source::{SourceFormat, Uri};
use std::collections::HashSet;
use std::{fs, io};

/// The result of a workspace reindex.
#[derive(Debug, Default)]
pub struct WorkspaceReindex {
    /// URIs whose content was refreshed from disk.
    pub updated: Vec<Uri>,
    /// URIs removed from the workspace because the backing file no longer exists.
    pub removed: Vec<Uri>,
}

impl Workspace {
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
    pub fn reindex_all_from_disk(&mut self) -> io::Result<WorkspaceReindex> {
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

        Ok(index)
    }
}

/// Infer a source format from a URI.
pub fn infer_source_format_from_uri(uri: &Uri) -> Option<SourceFormat> {
    infer_source_format_from_str(uri.as_ref())
}

/// Infer a source format from a string.
fn infer_source_format_from_str(value: &str) -> Option<SourceFormat> {
    let trimmed = value.split(['?', '#']).next().unwrap_or(value);
    let extension = trimmed.rsplit('.').next()?;
    if extension.contains('/') || extension.contains('\\') {
        return None;
    }
    SourceFormat::from_extension(&extension.to_ascii_lowercase())
}
