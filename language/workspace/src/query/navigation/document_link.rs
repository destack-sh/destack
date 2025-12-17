use destack_dir::Expression;
use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::get_module_by_file_id;

/// A clickable link in a document.
#[derive(Debug, Clone)]
pub struct DocumentLink {
    /// The range of the link in the document.
    pub range: Span,
    /// The target of the link.
    pub target: DocumentLinkTarget,
    /// Tooltip text (shown on hover).
    pub tooltip: Option<String>,
}

/// The target of a document link.
#[derive(Debug, Clone)]
pub enum DocumentLinkTarget {
    /// Link to a file (resolved import).
    File {
        /// The file path.
        path: String,
    },
    /// Link to a URL.
    Url {
        /// The URL.
        url: String,
    },
    /// Link to a position in a file.
    Position {
        /// The file path.
        path: String,
        /// Line number (0-indexed).
        line: u32,
        /// Column number (0-indexed).
        column: u32,
    },
}

impl DocumentLink {
    /// Create a file link.
    pub fn file(range: Span, path: impl Into<String>) -> Self {
        Self {
            range,
            target: DocumentLinkTarget::File { path: path.into() },
            tooltip: None,
        }
    }

    /// Create a URL link.
    pub fn url(range: Span, url: impl Into<String>) -> Self {
        Self {
            range,
            target: DocumentLinkTarget::Url { url: url.into() },
            tooltip: None,
        }
    }

    /// Add a tooltip.
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Get document links for a file.
///
/// Document links are clickable regions that navigate to files or URLs.
/// Common uses: import paths, URLs in comments, file references.
pub fn document_links(session: &Session, file: FileId) -> Vec<DocumentLink> {
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };

    let mut links = Vec::new();

    let module_guard = module.read();
    let dir_tree = module_guard.dir.tree.read();

    // find all import and re-export statements
    for (expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
        match expr {
            Expression::Import {
                target,
                target_module,
                ..
            }
            | Expression::ReExport {
                target,
                target_module,
                ..
            } => {
                // get the target module's file path
                let target_module_ref = session.modules.get(*target_module);
                let target_guard = target_module_ref.read();

                let Some(ref path) = target_guard.path else {
                    continue;
                };

                // get the span of this import expression
                let ast_node_id = dir_tree.get_source(expr_id.id);
                let span = module_guard.ast.tree.source_map.get(ast_node_id);

                // the link should be on the import path string, not the whole statement
                // for now, use the whole span but ideally we'd narrow to just the string
                let link_span = Span::new(file, span.start, span.end); // nocheckin #Suspicious

                // get the import path string for tooltip
                let import_path = session.strings.get(*target).to_string();

                links.push(
                    DocumentLink::file(link_span, path.to_string_lossy().to_string())
                        .with_tooltip(format!("Go to {import_path}")),
                );
            }
            _ => {}
        }
    }

    links
}

/// Resolve a document link (compute its target if deferred).
///
/// Some links defer resolution until clicked.
pub fn resolve_document_link(_session: &Session, link: &DocumentLink) -> DocumentLink {
    // currently all links are resolved immediately
    link.clone()
}
