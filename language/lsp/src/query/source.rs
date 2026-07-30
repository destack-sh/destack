use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use destack_lsp_server::{UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_repository::Revision;
use destack_source::{
    File, FileId, PatchSet, Span, TextChange, TextPosition, TextRange, Uri, WATCHABLE_FILE_TYPES,
};
use destack_workspace::{FileEdit, Workspace};

use crate::server::{internal_error, workspace_error};

/// One source document at a workspace revision.
pub(crate) struct Document {
    /// The source file.
    file: Arc<File>,
}

impl Document {
    /// Create a source document.
    pub(crate) fn new(file: Arc<File>) -> Self {
        Self { file }
    }

    /// Return the source file.
    pub(crate) fn file(&self) -> &File {
        &self.file
    }

    /// Return the source file id.
    pub(crate) fn id(&self) -> FileId {
        self.file.id
    }

    /// Build the document URI.
    pub(crate) fn uri(&self) -> jsonrpc::Result<lsp::Uri> {
        let Some(uri) = DocumentUri::source(&self.file.uri) else {
            return Err(internal_error(format!(
                "source file {:?} has no representable LSP URI: {}",
                self.file.id, self.file.uri
            )));
        };

        Ok(uri)
    }

    /// Decode an LSP position into a byte offset.
    pub(crate) fn offset(&self, position: &lsp::Position) -> jsonrpc::Result<u32> {
        let line_start_offsets = self.file.line_start_offsets().ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!(
                "source file {:?} has no line index",
                self.file.id
            ))
        })?;

        // resolve the requested source line
        let line_index = position.line as usize;
        let line_start = *line_start_offsets.get(line_index).ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!(
                "line {} is outside source file {:?}",
                position.line, self.file.id
            ))
        })?;
        let next_line_start = line_start_offsets.get(line_index + 1).copied();
        let line_end = next_line_start
            .map(|offset| offset - 1)
            .unwrap_or(self.file.len);
        let line = self
            .file
            .text()
            .get(line_start as usize..line_end as usize)
            .ok_or_else(|| {
                internal_error(format!(
                    "line {} has an invalid byte range in source file {:?}",
                    position.line, self.file.id
                ))
            })?;

        // resolve the requested UTF-16 column
        let mut utf16_column = 0u32;
        let mut byte_column = 0u32;
        for character in line.chars() {
            if utf16_column == position.character {
                break;
            }

            let character_width = character.len_utf16() as u32;
            if utf16_column + character_width > position.character {
                return Err(jsonrpc::Error::invalid_params(format!(
                    "character {} splits a UTF-16 surrogate pair on line {}",
                    position.character, position.line
                )));
            }

            utf16_column += character_width;
            byte_column += character.len_utf8() as u32;
        }

        if utf16_column != position.character {
            return Err(jsonrpc::Error::invalid_params(format!(
                "character {} is outside line {}",
                position.character, position.line
            )));
        }

        Ok(line_start + byte_column)
    }

    /// Encode a byte offset as an LSP position.
    pub(crate) fn position(&self, byte_index: u32) -> jsonrpc::Result<lsp::Position> {
        let Some((line, byte_column)) = self.file.get_position(byte_index) else {
            return Err(internal_error(format!(
                "byte offset {byte_index} is outside source file {:?} with length {}",
                self.file.id, self.file.len
            )));
        };
        let line_start = byte_index - byte_column;
        let Some(prefix) = self
            .file
            .text()
            .get(line_start as usize..byte_index as usize)
        else {
            return Err(internal_error(format!(
                "byte offset {byte_index} is not a character boundary in source file {:?}",
                self.file.id
            )));
        };
        let character = prefix.encode_utf16().count() as u32;

        Ok(lsp::Position { line, character })
    }

    /// Encode one byte span as an LSP range.
    pub(crate) fn range(&self, span: Span) -> jsonrpc::Result<lsp::Range> {
        if span.file != self.file.id {
            return Err(internal_error(format!(
                "span {:?} does not belong to source file {:?}",
                span, self.file.id
            )));
        }
        if span.start > span.end {
            return Err(internal_error(format!("span is reversed: {span:?}")));
        }

        let start = self.position(span.start)?;
        let end = self.position(span.end)?;

        Ok(lsp::Range { start, end })
    }

    /// Encode one target range and its contained selection range.
    pub(crate) fn ranges(
        &self,
        range: Span,
        selection_range: Span,
    ) -> jsonrpc::Result<(lsp::Range, lsp::Range)> {
        let is_contained = range.file == selection_range.file
            && range.start <= selection_range.start
            && selection_range.end <= range.end;
        if !is_contained {
            return Err(internal_error(format!(
                "selection range {selection_range:?} is not contained by target range {range:?}"
            )));
        }

        let range = self.range(range)?;
        let selection_range = self.range(selection_range)?;

        Ok((range, selection_range))
    }

    /// Encode one span as an LSP location.
    pub(crate) fn location(&self, span: Span) -> jsonrpc::Result<lsp::Location> {
        let range = self.range(span)?;
        let uri = self.uri()?;

        Ok(lsp::Location { uri, range })
    }
}

/// Source documents loaded from one workspace revision.
pub(crate) struct DocumentSet {
    /// Documents keyed by source file id.
    documents: HashMap<FileId, Document>,
}

impl DocumentSet {
    /// Create an empty document set.
    pub(crate) fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Load the requested documents from one revision.
    pub(crate) fn load(
        workspace: &dyn Workspace,
        path: &Path,
        revision: Revision,
        file_ids: impl IntoIterator<Item = FileId>,
    ) -> jsonrpc::Result<Self> {
        let mut requested: Vec<FileId> = file_ids.into_iter().collect();
        requested.sort_unstable();
        requested.dedup();
        if requested.is_empty() {
            return Ok(Self::new());
        }

        // load every document in one workspace request
        let root = workspace.root(path).map_err(workspace_error)?;
        let loaded = workspace
            .read_files(&root, revision, requested.clone())
            .map_err(workspace_error)?;
        for file in &loaded {
            if requested.binary_search(&file.id).is_err() {
                return Err(internal_error(format!(
                    "workspace returned unrequested source file {:?}",
                    file.id
                )));
            }
        }
        let mut documents = Self::new();
        for file in loaded {
            documents.insert(file)?;
        }

        // require an exact response for the requested ids
        if let Some(file_id) = requested
            .iter()
            .find(|file_id| !documents.documents.contains_key(file_id))
        {
            return Err(internal_error(format!(
                "workspace omitted requested source file {file_id:?}"
            )));
        }

        Ok(documents)
    }

    /// Return one loaded document.
    pub(crate) fn document(&self, file_id: FileId) -> jsonrpc::Result<&Document> {
        self.documents.get(&file_id).ok_or_else(|| {
            internal_error(format!(
                "document set does not contain requested file {file_id:?}"
            ))
        })
    }

    /// Add one source file.
    pub(crate) fn insert(&mut self, file: Arc<File>) -> jsonrpc::Result<()> {
        let document = Document::new(file);
        let file_id = document.id();
        if self.documents.insert(file_id, document).is_some() {
            return Err(internal_error(format!(
                "document set contains duplicate file {file_id:?}"
            )));
        }

        Ok(())
    }

    /// Encode one cross-document span as an LSP location.
    pub(crate) fn location(&self, span: Span) -> jsonrpc::Result<lsp::Location> {
        let document = self.document(span.file)?;

        document.location(span)
    }
}

impl Document {
    /// Encode one workspace file edit as an LSP text edit.
    pub(crate) fn text_edit(&self, edit: FileEdit) -> jsonrpc::Result<lsp::TextEdit> {
        Ok(lsp::TextEdit {
            range: self.range(edit.patch.span)?,
            new_text: edit.patch.new_text,
        })
    }
}

impl DocumentSet {
    /// Encode a patch set as an LSP workspace edit.
    #[allow(clippy::mutable_key_type)]
    pub(crate) fn workspace_edit(&self, patches: &PatchSet) -> jsonrpc::Result<lsp::WorkspaceEdit> {
        // collect edits grouped by uri
        let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();

        // build edits per document
        for file_edit in &patches.files {
            let document = self.document(file_edit.file)?;
            let uri = document.uri()?;

            let mut edits = file_edit.patches.iter().collect::<Vec<_>>();
            edits.sort_by_key(|edit| (edit.span.start, edit.span.end));

            // reject overlapping edits
            let mut previous_end = 0;
            for edit in &edits {
                if edit.span.start < previous_end {
                    return Err(internal_error(format!(
                        "source patches overlap in file {:?}",
                        document.id()
                    )));
                }
                previous_end = edit.span.end;
            }

            let edits = edits
                .into_iter()
                .map(|edit| {
                    Ok(lsp::TextEdit {
                        range: document.range(edit.span)?,
                        new_text: edit.new_text.clone(),
                    })
                })
                .collect::<jsonrpc::Result<Vec<_>>>()?;

            if changes.insert(uri, edits).is_some() {
                return Err(internal_error(format!(
                    "patch set contains duplicate file {:?}",
                    document.id()
                )));
            }
        }

        Ok(lsp::WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }
}

/// Globs for configuration files tracked by the LSP.
const CONFIGURATION_GLOBS: [&str; 1] = ["**/destack.json"];

/// LSP source synchronization rules.
pub(crate) struct SourceSync;

impl SourceSync {
    /// Build file watcher patterns for the client.
    pub(crate) fn file_watchers() -> Vec<lsp::FileSystemWatcher> {
        Self::tracked_file_globs()
            .into_iter()
            .map(|pattern| lsp::FileSystemWatcher {
                glob_pattern: pattern.to_string().into(),
                kind: None,
            })
            .collect()
    }

    /// Build file operation filters for the client.
    pub(crate) fn file_operation_filters() -> Vec<lsp::FileOperationFilter> {
        Self::tracked_file_globs()
            .into_iter()
            .map(|glob| lsp::FileOperationFilter {
                scheme: Some("file".to_string()),
                pattern: lsp::FileOperationPattern {
                    glob: glob.to_string(),
                    matches: Some(lsp::FileOperationPatternKind::File),
                    options: None,
                },
            })
            .collect()
    }

    /// Build source text changes from LSP text changes.
    pub(crate) fn text_changes(
        changes: Vec<lsp::TextDocumentContentChangeEvent>,
    ) -> Vec<TextChange> {
        changes
            .into_iter()
            .map(|change| TextChange {
                range: change.range.map(Self::text_range),
                text: change.text,
            })
            .collect()
    }

    /// Build a source text range from one LSP text range.
    pub(crate) fn text_range(range: lsp::Range) -> TextRange {
        TextRange {
            start: Self::text_position(range.start),
            end: Self::text_position(range.end),
        }
    }

    /// Build a source text position from one LSP text position.
    fn text_position(position: lsp::Position) -> TextPosition {
        TextPosition {
            line: position.line,
            character: position.character,
        }
    }

    /// Build the file globs tracked by the LSP.
    fn tracked_file_globs() -> Vec<&'static str> {
        let mut patterns = Vec::new();
        for file_type in WATCHABLE_FILE_TYPES {
            for pattern in file_type.globs() {
                if !patterns.contains(pattern) {
                    patterns.push(pattern);
                }
            }
        }

        // append configuration globs
        for pattern in CONFIGURATION_GLOBS {
            if !patterns.contains(&pattern) {
                patterns.push(pattern);
            }
        }

        patterns
    }
}

/// LSP document URI construction.
pub(crate) struct DocumentUri;

impl DocumentUri {
    /// Build an LSP URI for a file system path.
    pub(crate) fn path(path: impl AsRef<Path>) -> Option<lsp::Uri> {
        lsp::Uri::from_file_path(path)
    }

    /// Build an LSP URI from one source URI or absolute path.
    pub(crate) fn source(uri: &Uri) -> Option<lsp::Uri> {
        let value = uri.as_ref();
        if Path::new(value).is_absolute() {
            Self::path(value)
        } else {
            value.parse().ok()
        }
    }
}
