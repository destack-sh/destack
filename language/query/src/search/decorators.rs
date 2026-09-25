use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::ProfileId;

use crate::{Module, ProgramQueryContext, QueryError, QueryResult, Target};

/// Scope for decorator queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DecoratorScope {
    /// One module.
    Module(Module),
    /// Every module in one program.
    Program(ProfileId),
}

impl DecoratorScope {
    /// Return whether this scope includes one module.
    fn contains(&self, module: Module) -> bool {
        match self {
            Self::Module(target) => *target == module,
            Self::Program(profile_id) => *profile_id == module.profile_id,
        }
    }
}

/// One decorator query item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorItem {
    /// The decorator name when syntactically known.
    pub name: Option<String>,
    /// The decorator application source.
    pub application: Target,
    /// The decorator application node.
    pub application_node: dir::GlobalNodeId<dir::Decorator>,
    /// The decorated owner source.
    pub owner: Target,
    /// The decorated owner node.
    pub owner_node: dir::GlobalNodeIdAny,
    /// The exact checked decorator declaration.
    pub declaration: dir::DecoratorTarget,
}

/// A decorators request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorsRequest {
    /// The query scope.
    pub scope: DecoratorScope,
    /// The decorator name filter.
    pub name: Option<String>,
}

/// A decorators response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorsResponse {
    /// Matching decorators.
    pub decorators: Vec<DecoratorItem>,
}

impl ProgramQueryContext<'_> {
    /// Search decorator entries visible to a program query.
    pub fn decorators(&self, request: DecoratorsRequest) -> QueryResult<DecoratorsResponse> {
        let mut decorators = Vec::new();

        // collect indexed decorator occurrences in the requested scope
        for (profile_id, entry) in self.search_decorator_candidates(request.name.as_deref())? {
            let query_module = Module {
                module_id: entry.decorator.module_id,
                profile_id,
            };
            if !request.scope.contains(query_module) {
                continue;
            }

            let source_module = self.module(entry.decorator.module_id)?;
            let view = source_module.view()?;
            let decorator_span = source_module
                .node_selection_span(view, entry.decorator.local_id.into_any())?
                .ok_or(QueryError::missing(format!(
                    "decorator span: {:?}",
                    entry.decorator.into_any()
                )))?;
            let target_span = source_module
                .node_selection_span(view, entry.owner.local_id)?
                .ok_or(QueryError::missing(format!(
                    "decorator span: {:?}",
                    entry.owner
                )))?;

            decorators.push(DecoratorItem {
                name: entry.name,
                application: Target::new(query_module, decorator_span),
                application_node: entry.decorator,
                owner: Target::new(query_module, target_span),
                owner_node: entry.owner,
                declaration: entry.target,
            });
        }

        Ok(DecoratorsResponse { decorators })
    }
}
