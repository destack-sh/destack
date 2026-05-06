use destack_dir::Expression;
use destack_source::{FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};
use {destack_ast as ast, destack_dir as dir};

use super::specifier::{SpecifierLinkTarget, resolve_document_link_target};
use crate::ast::{main_or_enclosing_span_for_dir_node, string_literal_span_in_enclosing};
use crate::core::{with_ast_query_for_file, with_query_context_for_file};
use crate::dir::module_specifier_in_expression;

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
pub fn document_links(
    repository: &Repository,
    revision: Revision,
    file: FileId,
) -> Vec<DocumentLink> {
    // prefer dir based resolution when possible
    if let Some(mut links) = document_links_with_dir(repository, revision, file) {
        let fallback = document_links_with_ast(repository, revision, file);
        for link in fallback {
            if links.iter().any(|existing| existing.range == link.range) {
                continue;
            }
            if !links.contains(&link) {
                links.push(link);
            }
        }

        return links;
    }

    // fall back to ast only links
    document_links_with_ast(repository, revision, file)
}

/// Build document links using DIR data when available.
fn document_links_with_dir(
    repository: &Repository,
    revision: Revision,
    file: FileId,
) -> Option<Vec<DocumentLink>> {
    with_query_context_for_file(repository, revision, file, |ctx| {
        // find all import and re-export statements
        let dir_tree = ctx.dir().tree();
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
                    let Some(target_module) = repository
                        .module(revision, *target_module_id)
                        .ok()
                        .flatten()
                    else {
                        continue;
                    };
                    let Some(ref path) = target_module.path else {
                        continue;
                    };

                    // get the span of this import expression
                    let enclosing =
                        main_or_enclosing_span_for_dir_node(ctx.ast(), dir_tree, expr_id.into());
                    let Some(file) = repository.file(revision, ctx.file_id()).ok().flatten() else {
                        continue;
                    };
                    let import_path = ctx.dir().strings().get(*target).to_string();
                    let span = string_literal_span_in_enclosing(
                        &file,
                        ctx.ast().tokens(),
                        enclosing,
                        &import_path,
                    )
                    .unwrap_or(enclosing);

                    // make the link
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
fn document_links_with_ast(
    repository: &Repository,
    revision: Revision,
    file: FileId,
) -> Vec<DocumentLink> {
    with_ast_query_for_file(repository, revision, file, |ast| {
        // resolve the source file path
        let Some(source_file) = repository.file(revision, file).ok().flatten() else {
            return Vec::new();
        };
        let Some(path) = source_file.path.as_ref() else {
            return Vec::new();
        };
        let Some(base_dir) = path.parent() else {
            return Vec::new();
        };

        // collect document links from import/export expressions
        let mut links = Vec::new();
        for expression_id in ast.tree().iter_nodes::<ast::Expression>() {
            let expression = ast.tree().get(expression_id);

            // resolve the module specifier and dependency kind
            let Some((specifier, _kind)) = module_specifier_in_expression(ast.tree(), expression)
            else {
                continue;
            };

            // resolve the span for the string literal
            let enclosing = ast
                .tree()
                .get_main_span(expression_id)
                .unwrap_or_else(|| ast.source_map().get(expression_id.id));
            let specifier_text = ast.strings().get(specifier).to_string();
            let range = string_literal_span_in_enclosing(
                &source_file,
                ast.tokens(),
                enclosing,
                &specifier_text,
            )
            .unwrap_or(enclosing);

            // resolve the target for the specifier
            let target = resolve_document_link_target(
                &**repository.file_system(),
                base_dir,
                &specifier_text,
            );
            let Some(target) = target else {
                continue;
            };

            // emit the document link
            let target = match target {
                SpecifierLinkTarget::File { path } => DocumentLinkTarget::File {
                    path: path.to_string_lossy().to_string(),
                },
                SpecifierLinkTarget::Url { url } => DocumentLinkTarget::Url { url },
            };
            links.push(DocumentLink {
                range,
                target,
                tooltip: Some(format!("Go to {specifier_text}")),
            });
        }

        links
    })
    .unwrap_or_default()
}

/// Resolve a document link (compute its target if deferred).
///
/// Some links defer resolution until clicked.
pub fn resolve_document_link(link: &DocumentLink) -> DocumentLink {
    // currently all links are resolved immediately
    link.clone()
}
