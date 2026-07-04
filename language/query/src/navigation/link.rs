use destack_core::StringId;
use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::source::string_literal_span_in_enclosing;
use crate::{Module, ModuleQueryContext};

/// Module relation data for import and export expressions.
trait LinkExpression {
    /// Return the module relation and specifier target for links.
    fn module_relation_target(&self) -> Option<(dir::ModuleRelation, StringId)>;
}

impl LinkExpression for dir::Expression {
    fn module_relation_target(&self) -> Option<(dir::ModuleRelation, StringId)> {
        match self {
            dir::Expression::Import { target, .. } => Some((dir::ModuleRelation::Import, *target)),
            dir::Expression::Export { target, .. } => {
                target.map(|target| (dir::ModuleRelation::ReExport, target))
            }
            _ => None,
        }
    }
}

/// A clickable link in a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Link {
    /// The range of the link in the module.
    pub range: Span,
    /// The target of the link.
    pub target: LinkTarget,
    /// Tooltip text (shown on hover).
    pub tooltip: Option<String>,
}

/// The target of a link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LinkTarget {
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

impl Link {
    /// Create a file link.
    pub fn file(range: Span, path: impl Into<String>) -> Self {
        Self {
            range,
            target: LinkTarget::File { path: path.into() },
            tooltip: None,
        }
    }

    /// Create a URL link.
    pub fn url(range: Span, url: impl Into<String>) -> Self {
        Self {
            range,
            target: LinkTarget::Url { url: url.into() },
            tooltip: None,
        }
    }

    /// Add a tooltip.
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Request links for a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LinksRequest {
    /// The queried module.
    pub module: Module,
}

/// Response payload for links queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LinksResponse {
    /// Links.
    pub links: Vec<Link>,
}

impl ModuleQueryContext<'_> {
    /// Return links for a file.
    pub fn links(&self) -> Vec<Link> {
        let view = self.view();
        let mut links = Vec::new();

        // collect import and export path links
        for (expression_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            let Some((relation, target)) = expression.module_relation_target() else {
                continue;
            };
            let node_id = expression_id.into_global_any(self.module_id());
            let Some(target_module_id) = self.modules().target_for_source(node_id, relation) else {
                continue;
            };
            let target_module = self
                .repository()
                .module(self.revision(), target_module_id)
                .unwrap_or_else(|error| {
                    panic!("failed to read linked module {target_module_id:?}: {error}")
                })
                .unwrap_or_else(|| panic!("missing linked module {target_module_id:?}"));
            let Some(path) = target_module.path.as_ref() else {
                continue;
            };

            let file = self.source_file();
            let enclosing = self.get_main_span(view, expression_id.into());
            let import_path = self.strings().get(target).to_string();
            let Some(span) =
                string_literal_span_in_enclosing(&file, self.tokens(), enclosing, &import_path)
            else {
                continue;
            };

            links.push(
                Link::file(span, path.to_string_lossy().to_string())
                    .with_tooltip(format!("Go to {import_path}")),
            );
        }

        links
    }
}
