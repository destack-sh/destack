use destack_dir::Expression;
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::with_query_context_for_file;

/// A clickable link in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentLink {
    /// The range of the link in the document.
    pub range: Span,
    /// The target of the link.
    pub target: DocumentLinkTarget,
    /// Tooltip text (shown on hover).
    pub tooltip: Option<String>,
}

/// The target of a document link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Request document links for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentLinksRequest {
    /// The document URI.
    pub uri: Uri,
}

/// Request to resolve a document link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveDocumentLinkRequest {
    /// The document link to resolve.
    pub link: DocumentLink,
}

/// Response payload for document links queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentLinksResponse {
    /// Document links.
    pub links: Vec<DocumentLink>,
}

/// Response payload for document link resolve queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveDocumentLinkResponse {
    /// The resolved document link.
    pub link: DocumentLink,
}

/// Get document links for a file.
///
/// Document links are clickable regions that navigate to files or URLs.
/// Common uses: import paths, URLs in comments, file references.
pub fn document_links(session: &Session, file: FileId) -> Vec<DocumentLink> {
    with_query_context_for_file(session, file, |ctx| {
        // find all import and re-export statements
        let dir_tree = ctx.tree();
        let mut links = Vec::new();
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
                    // skip module bindings for document links
                    let destack_dir::ModuleTarget::Module(target_module_id) = target_module else {
                        continue;
                    };

                    // get the target module's file path
                    let target_module_ref = session.modules.get(*target_module_id);
                    let target_guard = target_module_ref.read();
                    let Some(ref path) = target_guard.path else {
                        continue;
                    };

                    // get the span of this import expression
                    let ast_node_id = dir_tree.get_source(expr_id.id);
                    let span = ctx.ast.tree.source_map.get_main_or_enclosing(ast_node_id);

                    // make the link
                    let import_path = session.strings.get(*target).to_string();
                    links.push(
                        DocumentLink::file(span, path.to_string_lossy().to_string())
                            .with_tooltip(format!("Go to {import_path}")),
                    );
                }
                _ => {}
            }
        }

        links
    })
    .unwrap_or_default()
}

/// Resolve a document link (compute its target if deferred).
///
/// Some links defer resolution until clicked.
pub fn resolve_document_link(_session: &Session, link: &DocumentLink) -> DocumentLink {
    // currently all links are resolved immediately
    link.clone()
}
