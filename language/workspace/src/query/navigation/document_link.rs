use destack_source::{FileId, Span};

use crate::Session;

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
pub fn document_links(_session: &Session, _file: FileId) -> Vec<DocumentLink> {
    // 1. find all import statements, create links to resolved files
    // 2. find URLs in comments and strings
    // 3. find file path references (e.g., in configuration)
    todo!("#Incomplete: document_links")
}

/// Resolve a document link (compute its target if deferred).
///
/// Some links defer resolution until clicked.
pub fn resolve_document_link(_session: &Session, _link: &DocumentLink) -> DocumentLink {
    // For import links: resolve the module specifier to a file path
    // For relative paths: resolve to absolute
    todo!("#Incomplete: resolve_document_link")
}
