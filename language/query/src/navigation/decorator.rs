use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::ProfileId;
use serde::{Deserialize, Serialize};

use crate::{Module, ProgramQueryContext, Target};

/// Scope for decorator queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DecoratorScope {
    /// One module.
    Module(Module),
    /// Program profiles.
    Program {
        /// The profiles to search.
        profile_ids: Vec<ProfileId>,
    },
}

impl DecoratorScope {
    /// Return whether this scope includes one module.
    fn contains(&self, module: Module) -> bool {
        match self {
            Self::Module(target) => *target == module,
            Self::Program { profile_ids } => profile_ids.contains(&module.profile_id),
        }
    }
}

/// Role of one decorator expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DecoratorRole {
    /// Compiler language item decorator.
    LanguageItem,
    /// User-defined decorator symbol.
    Symbol,
    /// Unresolved or non-symbol decorator.
    Unresolved,
}

impl From<dir::DecoratorResolution> for DecoratorRole {
    fn from(resolution: dir::DecoratorResolution) -> Self {
        match resolution {
            dir::DecoratorResolution::LanguageItem(_) => Self::LanguageItem,
            dir::DecoratorResolution::Symbol(_) => Self::Symbol,
            dir::DecoratorResolution::Unresolved => Self::Unresolved,
        }
    }
}

/// Request payload for decorator queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorsRequest {
    /// The query scope.
    pub scope: DecoratorScope,
    /// The decorator name filter.
    pub name: Option<String>,
}

/// One decorator query item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorItem {
    /// The decorator name when syntactically known.
    pub name: Option<String>,
    /// The decorator expression target.
    pub decorator: Target,
    /// The decorated target.
    pub target: Target,
    /// The resolved decorator role.
    pub role: DecoratorRole,
}

/// Response payload for decorator queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorsResponse {
    /// Matching decorators.
    pub decorators: Vec<DecoratorItem>,
}

impl ProgramQueryContext<'_> {
    /// Search decorator entries visible to a program query.
    pub fn decorators(&self, scope: &DecoratorScope, name: Option<&str>) -> Vec<DecoratorItem> {
        self.search_decorator_candidates(name)
            .into_iter()
            .filter_map(|(profile_id, entry)| {
                let query_module = Module {
                    module_id: entry.decorator.module_id,
                    profile_id,
                };
                if !scope.contains(query_module) {
                    return None;
                }

                let source_module = self.module_context(entry.decorator.module_id, profile_id);
                let view = source_module.view();
                let decorator_span = source_module.get_main_span(view, entry.decorator.local_id);
                let target_span = source_module.get_main_span(view, entry.target.local_id);

                Some(DecoratorItem {
                    name: entry.name,
                    decorator: Target::new(query_module, decorator_span)
                        .with_node_id(entry.decorator),
                    target: Target::new(query_module, target_span).with_node_id(entry.target),
                    role: entry.resolution.into(),
                })
            })
            .collect()
    }
}
