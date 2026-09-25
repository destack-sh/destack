use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tspp_lsp_server::{UriExt, jsonrpc};
use tspp_lsp_types as lsp;
use tspp_repository::Revision;
use tspp_source::{File, FileId, PatchSet, Span, TextChange, TextPosition, TextRange, Uri};
use tspp_workspace::{FileEdit, Workspace};

use crate::server::{ProjectId, TSPP_URI_SCHEME, internal_error, workspace_error};

/// Convert one LSP value into a TS++ source value.
pub(crate) trait IntoSource {
    /// The converted TS++ source value.
    type Source;

    /// Convert this value.
    fn into_source(self) -> Self::Source;
}

/// Convert one TS++ value into an LSP value.
pub(crate) trait IntoLsp {
    /// The converted LSP value.
    type Lsp;

    /// Convert this value.
    fn into_lsp(self) -> Self::Lsp;
}

impl IntoSource for lsp::Position {
    type Source = TextPosition;

    /// Convert this LSP position into a source position.
    fn into_source(self) -> TextPosition {
        TextPosition {
            line: self.line,
            character: self.character,
        }
    }
}

impl IntoSource for lsp::Range {
    type Source = TextRange;

    /// Convert this LSP range into a source range.
    fn into_source(self) -> TextRange {
        TextRange {
            start: self.start.into_source(),
            end: self.end.into_source(),
        }
    }
}

impl IntoSource for lsp::TextDocumentContentChangeEvent {
    type Source = TextChange;

    /// Convert this LSP document change into a source change.
    fn into_source(self) -> TextChange {
        TextChange {
            range: self.range.map(IntoSource::into_source),
            text: self.text,
        }
    }
}

/// Convert one path-like value into an LSP URI.
pub(crate) trait ToLspUri {
    /// Convert this value when LSP can represent it.
    fn to_lsp_uri(&self) -> Option<lsp::Uri>;
}

impl ToLspUri for Path {
    /// Convert this file system path into an LSP URI.
    fn to_lsp_uri(&self) -> Option<lsp::Uri> {
        lsp::Uri::from_file_path(self)
    }
}

impl ToLspUri for Uri {
    /// Convert this source URI or absolute path into an LSP URI.
    fn to_lsp_uri(&self) -> Option<lsp::Uri> {
        let value = self.as_ref();
        if Path::new(value).is_absolute() {
            Path::new(value).to_lsp_uri()
        } else {
            value.parse().ok()
        }
    }
}

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
    pub(crate) fn uri(&self, project: ProjectId) -> jsonrpc::Result<lsp::Uri> {
        if self.file.uri.scheme() == Some(TSPP_URI_SCHEME) {
            return project.qualify(&self.file.uri);
        }

        let Some(uri) = self.file.uri.to_lsp_uri() else {
            return Err(internal_error(format!(
                "source file {:?} has no representable LSP URI: {}",
                self.file.id, self.file.uri
            )));
        };

        Ok(uri)
    }

    /// Return this document's path for editor display.
    pub(crate) fn display_path(&self, root: &Path) -> String {
        let uri = &self.file.uri;

        // retain virtual URIs and physical paths outside the workspace
        let path = uri.to_path();
        let path = path.and_then(|path| path.strip_prefix(root).ok());
        let Some(path) = path else {
            return uri.to_string();
        };

        path.to_string_lossy().replace('\\', "/")
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
    pub(crate) fn location(
        &self,
        project: ProjectId,
        span: Span,
    ) -> jsonrpc::Result<lsp::Location> {
        let range = self.range(span)?;
        let uri = self.uri(project)?;

        Ok(lsp::Location { uri, range })
    }
}

/// Source documents loaded from one workspace revision.
pub(crate) struct DocumentSet {
    /// Stable identity of the owning project.
    pub(super) project: ProjectId,
    /// The workspace root used to load the documents.
    pub(super) root: PathBuf,
    /// Documents keyed by source file id.
    documents: HashMap<FileId, Document>,
}

impl DocumentSet {
    /// Create an empty document set.
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            project: ProjectId::from_root(root),
            root: root.to_path_buf(),
            documents: HashMap::new(),
        }
    }

    /// Load the requested documents from one revision.
    pub(crate) fn load(
        workspace: &Workspace,
        revision: Revision,
        file_ids: impl IntoIterator<Item = FileId>,
    ) -> jsonrpc::Result<Self> {
        let mut requested: Vec<FileId> = file_ids.into_iter().collect();
        requested.sort_unstable();
        requested.dedup();
        if requested.is_empty() {
            return Ok(Self::new(workspace.root()));
        }

        // load every document in one workspace request
        let loaded = workspace
            .read_files(revision, requested.clone())
            .map_err(workspace_error)?;
        for file in &loaded {
            if requested.binary_search(&file.id).is_err() {
                return Err(internal_error(format!(
                    "workspace returned unrequested source file {:?}",
                    file.id
                )));
            }
        }
        let mut documents = Self::new(workspace.root());
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

        document.location(self.project, span)
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
            let uri = document.uri(self.project)?;

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
