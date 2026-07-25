use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use destack_lsp_types as lsp;
use destack_repository::Revision;
use destack_source::{File, FileId, TextChange, TextPosition, TextRange, WATCHABLE_FILE_TYPES};
use destack_workspace::{Error, FileImagesRequest, Workspace};

/// Source files loaded from one exact workspace revision.
pub(super) struct SourceFiles {
    /// Files keyed by source file id.
    files: HashMap<FileId, Arc<File>>,
}

impl SourceFiles {
    /// Create an empty source file set.
    pub(super) fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Load the requested source files from one exact revision.
    pub(super) fn load(
        workspace: &dyn Workspace,
        path: &Path,
        revision: Revision,
        file_ids: impl IntoIterator<Item = FileId>,
    ) -> Result<Self, Error> {
        let mut requested: Vec<FileId> = file_ids.into_iter().collect();
        requested.sort_unstable();
        requested.dedup();
        if requested.is_empty() {
            return Ok(Self::new());
        }

        // load every file in one workspace request
        let root = workspace.root(path)?;
        let request = FileImagesRequest {
            revision,
            file_ids: requested.clone(),
        };
        let images = workspace.file_images(&root, request)?;
        for image in &images {
            if requested.binary_search(&image.id).is_err() {
                return Err(Error::Internal {
                    detail: format!("workspace returned unrequested file image {:?}", image.id),
                });
            }
        }
        let mut files = Self::new();
        for image in images {
            let file = Arc::new(image.into_file()?);
            files.insert(file)?;
        }

        // require an exact response for the requested ids
        if let Some(file_id) = requested
            .iter()
            .find(|file_id| !files.files.contains_key(file_id))
        {
            return Err(Error::Internal {
                detail: format!("workspace omitted requested file image {file_id:?}"),
            });
        }

        Ok(files)
    }

    /// Return one loaded source file.
    pub(super) fn file(&self, file_id: FileId) -> Result<Arc<File>, Error> {
        self.files
            .get(&file_id)
            .cloned()
            .ok_or_else(|| Error::Internal {
                detail: format!("source files do not contain requested file {file_id:?}"),
            })
    }

    /// Add one exact source file.
    pub(super) fn insert(&mut self, file: Arc<File>) -> Result<(), Error> {
        let file_id = file.id;
        if self.files.insert(file_id, file).is_some() {
            return Err(Error::Internal {
                detail: format!("source files contain duplicate file {file_id:?}"),
            });
        }

        Ok(())
    }
}

/// Globs for config files tracked by the LSP.
pub(super) const CONFIG_GLOBS: [&str; 1] = ["**/destack.json"];

/// Build file watcher patterns for the client.
pub(super) fn file_watchers() -> Vec<lsp::FileSystemWatcher> {
    let mut watchers = Vec::new();
    for pattern in tracked_file_globs() {
        watchers.push(lsp::FileSystemWatcher {
            glob_pattern: pattern.to_string().into(),
            kind: None,
        });
    }

    watchers
}

/// Build the set of file globs tracked by the LSP.
pub(super) fn tracked_file_globs() -> Vec<&'static str> {
    let mut patterns = Vec::new();
    for file_type in WATCHABLE_FILE_TYPES {
        for pattern in file_type.globs() {
            if !patterns.contains(pattern) {
                patterns.push(pattern);
            }
        }
    }

    // append config globs
    for pattern in CONFIG_GLOBS {
        if !patterns.contains(&pattern) {
            patterns.push(pattern);
        }
    }

    patterns
}

/// Convert LSP text changes into source text changes.
pub(super) fn changes(changes: Vec<lsp::TextDocumentContentChangeEvent>) -> Vec<TextChange> {
    changes
        .into_iter()
        .map(|change| TextChange {
            range: change.range.map(|range| TextRange {
                start: TextPosition {
                    line: range.start.line,
                    character: range.start.character,
                },
                end: TextPosition {
                    line: range.end.line,
                    character: range.end.character,
                },
            }),
            text: change.text,
        })
        .collect()
}

/// Normalize line endings to LF.
pub(super) fn normalize_line_endings(content: String) -> String {
    if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content
    }
}
