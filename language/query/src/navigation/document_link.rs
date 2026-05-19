use destack_dir as dir;
use destack_dir::Expression;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::core::{ModuleQueryContext, QueryModule};
use crate::source::{main_or_enclosing_span_for_dir_node, string_literal_span_in_enclosing};

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
    /// The queried module.
    pub module: QueryModule,
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
pub fn document_links(ctx: &ModuleQueryContext<'_>) -> Vec<DocumentLink> {
    let dir_tree = ctx.dir().view();
    let mut links = Vec::new();

    // collect import and export path links
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let target = match expression {
            Expression::Import { target, .. } => Some(*target),
            Expression::Export { target, .. } => *target,
            _ => None,
        };
        let Some(target) = target else {
            continue;
        };

        let node_id = expression_id.into_global_any(ctx.module_id());
        let relation = match expression {
            Expression::Import { .. } => dir::DependencyRelation::Import,
            Expression::Export { .. } => dir::DependencyRelation::ReExport,
            _ => unreachable!(),
        };
        let Some(target_module_id) = ctx
            .dir()
            .dependencies()
            .target_for_source(node_id, relation)
        else {
            continue;
        };
        let Some(target_module) = ctx
            .repository()
            .module(ctx.revision(), target_module_id)
            .ok()
            .flatten()
        else {
            continue;
        };
        let Some(path) = target_module.path.as_ref() else {
            continue;
        };

        let Some(file) = ctx
            .repository()
            .file(ctx.revision(), ctx.file_id())
            .ok()
            .flatten()
        else {
            continue;
        };
        let enclosing =
            main_or_enclosing_span_for_dir_node(ctx.dir(), dir_tree, expression_id.into());
        let import_path = ctx.dir().strings().get(target).to_string();
        let span =
            string_literal_span_in_enclosing(&file, ctx.dir().tokens(), enclosing, &import_path)
                .unwrap_or(enclosing);

        links.push(
            DocumentLink::file(span, path.to_string_lossy().to_string())
                .with_tooltip(format!("Go to {import_path}")),
        );
    }

    links
}

/// Resolve a document link (compute its target if deferred).
///
/// Some links defer resolution until clicked.
pub fn resolve_document_link(link: &DocumentLink) -> DocumentLink {
    // currently all links are resolved immediately
    link.clone()
}
