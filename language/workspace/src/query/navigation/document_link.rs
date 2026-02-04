use destack_dir::Expression;
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use {destack_ast as ast, destack_dir as dir};

use crate::Session;
use crate::query::common::{
    get_module_by_file_id, main_or_enclosing_span_for_dir_node, with_query_context_for_file,
};

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
    // prefer dir based resolution when possible
    if let Some(links) = document_links_with_dir(session, file) {
        return links;
    }

    // fall back to ast only links
    document_links_with_ast(session, file)
}

/// Build document links using DIR data when available.
fn document_links_with_dir(session: &Session, file: FileId) -> Option<Vec<DocumentLink>> {
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
                    let dir::ModuleTarget::Module(target_module_id) = target_module else {
                        continue;
                    };

                    // get the target module's file path
                    let target_module_ref = session.modules.get(*target_module_id);
                    let target_guard = target_module_ref.read();
                    let Some(ref path) = target_guard.path else {
                        continue;
                    };

                    // get the span of this import expression
                    let span = main_or_enclosing_span_for_dir_node(&ctx, &dir_tree, expr_id.into());

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
}

/// Build document links using AST data when DIR is unavailable.
fn document_links_with_ast(session: &Session, file: FileId) -> Vec<DocumentLink> {
    // resolve the module ast
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };
    let module = module.read();
    let Some(ast) = module.ast_maybe() else {
        return Vec::new();
    };

    // resolve the source file path
    let source_file = session.files.get(file);
    let Some(path) = source_file.path.as_ref() else {
        return Vec::new();
    };
    let Some(base_dir) = path.parent() else {
        return Vec::new();
    };

    // collect document links from import/export expressions
    let mut links = Vec::new();
    for expression_id in ast.tree.iter_nodes::<ast::Expression>() {
        let expression = ast.tree.get(expression_id);

        // extract the module specifier from the expression
        let specifier = match expression {
            ast::Expression::Import { target, .. } => Some(*target),
            ast::Expression::Export { target, .. } => *target,
            _ => None,
        };

        // skip expressions without specifiers
        let Some(specifier) = specifier else {
            continue;
        };

        // resolve the span for the string literal
        let range = ast
            .tree
            .get_main_span(expression_id)
            .unwrap_or_else(|| ast.tree.source_map.get(expression_id.id));

        // resolve the target for the specifier
        let specifier_text = ast.strings.get(specifier).to_string();
        let target = match document_link_target_for_specifier(base_dir, &specifier_text) {
            Some(target) => target,
            None => continue,
        };

        // emit the document link
        links.push(DocumentLink {
            range,
            target,
            tooltip: Some(format!("Go to {specifier_text}")),
        });
    }

    links
}

/// Resolve the target for a module specifier.
fn document_link_target_for_specifier(
    base_dir: &Path,
    specifier: &str,
) -> Option<DocumentLinkTarget> {
    // handle URLs directly
    if specifier.starts_with("http://") || specifier.starts_with("https://") {
        return Some(DocumentLinkTarget::Url {
            url: specifier.to_string(),
        });
    }

    // handle file urls
    if let Some(path) = specifier.strip_prefix("file://") {
        let path = PathBuf::from(path);
        return resolve_file_target(path);
    }

    // handle relative or absolute paths
    if specifier.starts_with('.') || specifier.starts_with('/') {
        let mut path = PathBuf::from(specifier);

        // resolve relative paths against the base directory
        if path.is_relative() {
            path = base_dir.join(path);
        }
        return resolve_file_target(path);
    }

    None
}

/// Resolve a file path to a document link target.
fn resolve_file_target(path: PathBuf) -> Option<DocumentLinkTarget> {
    // build candidate paths for the specifier
    let candidates = document_link_candidates(&path);

    // pick the first candidate that exists
    let resolved = candidates
        .into_iter()
        .find(|candidate| candidate.exists())
        .unwrap_or(path);

    // return the resolved target
    Some(DocumentLinkTarget::File {
        path: resolved.to_string_lossy().to_string(),
    })
}

/// Build candidate paths for a module specifier.
fn document_link_candidates(path: &Path) -> Vec<PathBuf> {
    // seed with the original path
    let mut candidates = Vec::new();
    candidates.push(path.to_path_buf());

    // skip extension probing when an extension is already present
    let has_extension = path.extension().is_some();
    if has_extension {
        return candidates;
    }

    // add extension variants
    let extensions = ["ds", "d.ts", "ts", "tsx"];
    for extension in extensions {
        candidates.push(path.with_extension(extension));
    }

    // add index file variants
    for extension in extensions {
        candidates.push(path.join(format!("index.{extension}")));
    }

    candidates
}

/// Resolve a document link (compute its target if deferred).
///
/// Some links defer resolution until clicked.
pub fn resolve_document_link(_session: &Session, link: &DocumentLink) -> DocumentLink {
    // currently all links are resolved immediately
    link.clone()
}
