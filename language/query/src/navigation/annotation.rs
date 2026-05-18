use destack_source::ProfileId;
use serde::{Deserialize, Serialize};

use crate::core::{QueryModule, QueryTarget, WorkspaceQueryContext, search_annotation_candidates};
use crate::source::main_span_for_dir_node;

/// Scope for annotation queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationScope {
    /// One module.
    Module(QueryModule),
    /// Workspace profiles.
    Workspace {
        /// The profiles to search.
        profile_ids: Vec<ProfileId>,
    },
}

/// Role of one annotation expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationRole {
    /// Metadata annotation.
    Annotation,
    /// Behavior decorator.
    Decorator,
    /// Unresolved annotation or decorator.
    Unknown,
}

/// Request payload for annotation queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationsRequest {
    /// The query scope.
    pub scope: AnnotationScope,
    /// The annotation name filter.
    pub name: Option<String>,
}

/// One annotation query item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationItem {
    /// The annotation name when syntactically known.
    pub name: Option<String>,
    /// The decorator expression target.
    pub decorator: QueryTarget,
    /// The annotated target.
    pub target: QueryTarget,
    /// The resolved annotation role.
    pub role: AnnotationRole,
}

/// Response payload for annotation queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationsResponse {
    /// Matching annotations.
    pub annotations: Vec<AnnotationItem>,
}

/// Search annotation entries visible to a workspace query.
pub fn annotations(
    ctx: &WorkspaceQueryContext<'_>,
    scope: &AnnotationScope,
    name: Option<&str>,
) -> Vec<AnnotationItem> {
    search_annotation_candidates(ctx, name)
        .into_iter()
        .filter_map(|(profile_id, entry)| {
            if let AnnotationScope::Module(module) = scope
                && (module.module_id != entry.module_id || module.profile_id != profile_id)
            {
                return None;
            }

            let module = QueryModule {
                module_id: entry.module_id,
                profile_id,
            };
            let module_ctx = ctx.module_context(entry.module_id, profile_id)?;
            let dir = module_ctx.dir();
            let view = dir.view();
            let decorator_span = main_span_for_dir_node(dir, view, entry.decorator_id.local_id)?;
            let target_span = main_span_for_dir_node(dir, view, entry.target_id.local_id)?;

            Some(AnnotationItem {
                name: entry.name,
                decorator: QueryTarget::span(module, decorator_span).with_node(entry.decorator_id),
                target: QueryTarget::span(module, target_span).with_node(entry.target_id),
                role: AnnotationRole::Unknown,
            })
        })
        .collect()
}
