use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_lsp_server::UriExt;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_repository::{FormatterOptions, Revision};
use destack_source::{File, FileId, Span};
use destack_workspace::{Error, QueryRequest, QueryResponseBody, Workspace};

use super::source::file_from_image;
use super::view;

/// File state used by one LSP query request.
pub(super) struct QueryFile {
    /// The local file path.
    pub(super) path: PathBuf,
    /// The exact revision used for the query.
    pub(super) revision: Revision,
    /// The query module.
    pub(super) module: query::Module,
    /// The source file id.
    pub(super) file_id: FileId,
    /// The coherent file contents.
    pub(super) file: Arc<File>,
    /// Formatter options selected for the file.
    pub(super) formatter: FormatterOptions,
}

/// Request-local source file map.
pub(super) struct FileMap {
    /// Workspace used to load file images.
    workspace: Arc<dyn Workspace>,
    /// Root used for workspace view requests.
    root: PathBuf,
    /// Revision containing the file images.
    revision: Revision,
    /// Loaded files keyed by source file id.
    files: HashMap<FileId, Arc<File>>,
}

impl FileMap {
    /// Create an empty file map for one revision.
    pub(super) fn new(workspace: Arc<dyn Workspace>, root: PathBuf, revision: Revision) -> Self {
        Self {
            workspace,
            root,
            revision,
            files: HashMap::new(),
        }
    }

    /// Return one source file.
    pub(super) fn file(&mut self, file_id: FileId) -> Option<Arc<File>> {
        // reuse files loaded earlier in the same request
        if let Some(file) = self.files.get(&file_id) {
            return Some(file.clone());
        }

        // load exactly the requested file image
        let images = view::file_images(
            self.workspace.as_ref(),
            &self.root,
            self.revision,
            vec![file_id],
        )
        .ok()?;

        // retain returned file images for later mappings
        for image in images {
            let file = file_from_image(&image)?;
            self.files.insert(image.id, file);
        }

        self.files.get(&file_id).cloned()
    }
}

/// Build one query position from an LSP file callback.
pub(super) fn position(module: query::Module, file_id: FileId, offset: u32) -> query::Position {
    query::Position {
        module,
        file_id,
        offset,
    }
}

/// Build one query range from an LSP file callback.
pub(super) fn range(module: query::Module, file_id: FileId, start: u32, end: u32) -> query::Range {
    let range_start = start.min(end);
    let range_end = start.max(end);

    query::Range {
        module,
        span: Span::new(file_id, range_start, range_end),
    }
}

/// Return the exact file state for one LSP query URI.
pub(super) fn file(
    workspace: &dyn Workspace,
    uri: &lsp::Uri,
    target: Option<String>,
) -> Option<QueryFile> {
    let path = uri.to_file_path().map(|path| path.into_owned())?;
    let snapshot = view::file_snapshot(workspace, path.clone(), target)
        .ok()
        .flatten()?;
    let file = file_from_image(&snapshot.file)?;
    let module = snapshot.module?;

    Some(QueryFile {
        path,
        revision: snapshot.revision,
        module,
        file_id: snapshot.file_id,
        file,
        formatter: snapshot.formatter,
    })
}

/// Execute one semantic query.
pub(super) fn execute(
    workspace: &dyn Workspace,
    path: &Path,
    request: destack_query::QueryRequest,
    revision: Revision,
) -> Result<QueryResponseBody, Error> {
    let root = workspace.root(path)?;
    let request = QueryRequest {
        expected_revision: Some(revision),
        request,
    };
    let result = workspace.query(&root, request)?;

    Ok(QueryResponseBody {
        revision: result.revision,
        response: result.response,
    })
}
