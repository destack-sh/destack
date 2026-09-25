use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    ModuleQueryContext, NavigationTarget, ProgramQueryContext, QueryPosition, QueryRange,
    QueryResult, sort_and_dedup_navigation_targets,
};

/// A goto definition request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDefinitionRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A goto definition response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDefinitionResponse {
    /// Definition targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find the definition of the symbol at one position.
    pub fn goto_definition(
        &self,
        request: GotoDefinitionRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<GotoDefinitionResponse> {
        let position = request.position;
        let Some(occurrence) = self
            .cursor(position.file_id, position.offset)?
            .symbol(program)?
        else {
            return Ok(GotoDefinitionResponse {
                targets: Vec::new(),
            });
        };
        let origin = QueryRange {
            module: self.module(),
            span: occurrence.span,
        };

        // collect each exact definition target
        let mut targets = Vec::new();
        for symbol_id in occurrence.symbols {
            for symbol_id in program.symbol_targets(symbol_id)? {
                let module = program.module(symbol_id.module_id)?;

                targets.push(module.navigation_target(program, symbol_id, origin)?);
            }
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(GotoDefinitionResponse { targets })
    }
}
