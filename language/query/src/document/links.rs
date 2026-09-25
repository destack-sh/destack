use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_repository::RepositoryError;
use tspp_serde::Reflect;
use tspp_source::{FileId, Span};

use crate::{Module, ModuleQueryContext, QueryError, QueryResult};

/// A clickable link in a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Link {
    /// The range of the link in the module.
    pub range: Span,
    /// The resolved target source file.
    pub target: FileId,
}

impl Link {
    /// Create a file link.
    fn new(range: Span, target: FileId) -> Self {
        Self { range, target }
    }
}

/// A links request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LinksRequest {
    /// The queried module profile.
    pub module: Module,
    /// The queried source file.
    pub file_id: FileId,
}

/// A links response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LinksResponse {
    /// Links.
    pub links: Vec<Link>,
}

impl ModuleQueryContext<'_> {
    /// Return links for a file.
    pub fn links(&self, request: LinksRequest) -> QueryResult<LinksResponse> {
        let file_id = request.file_id;
        let view = self.view()?;
        let mut links = Vec::new();

        // collect import and export path links
        for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
            let relation = match expression {
                dir::Expression::Import { .. } => Some(dir::ModuleRelation::Import),
                dir::Expression::Export {
                    target: Some(_), ..
                } => Some(dir::ModuleRelation::ReExport),
                _ => None,
            };
            let Some(relation) = relation else {
                continue;
            };
            let node_id = expression_id.into_global_any(self.module_id());
            let Some(target_module_id) = self.modules()?.target_for_source(node_id, relation)
            else {
                continue;
            };
            let target_module = self
                .repository()
                .module(self.revision(), target_module_id)?
                .ok_or(RepositoryError::MissingModule {
                    module: target_module_id,
                })?;
            let span = self
                .node_selection_span(view, expression_id.into())?
                .ok_or(QueryError::missing(format!("link span: {node_id:?}")))?;
            if span.file != file_id {
                continue;
            }

            links.push(Link::new(span, target_module.file_id));
        }

        links.sort_by_key(|link| (link.range.start, link.range.end));

        Ok(LinksResponse { links })
    }
}
