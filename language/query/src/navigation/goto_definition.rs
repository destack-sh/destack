use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    ModuleQueryContext, NavigationTarget, ProgramQueryContext, QueryPosition, QueryRange,
    QueryResult, sort_and_dedup_navigation_targets,
};

/// Request goto definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDefinitionRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for goto definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDefinitionResponse {
    /// Definition targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find the definition of the symbol at one position.
    pub fn goto_definition(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Vec<NavigationTarget>> {
        let Some(occurrence) = self.symbol_at_offset(program, file_id, offset)? else {
            return Ok(Vec::new());
        };
        let origin = QueryRange {
            module: self.module(),
            span: occurrence.span,
        };

        // collect each exact definition target
        let mut targets = Vec::new();
        for symbol_id in occurrence.symbols {
            for symbol_id in program.canonical_symbols(symbol_id)? {
                let module = program.module(symbol_id.module_id)?;

                targets.push(module.navigation_target(program, symbol_id, origin)?);
            }
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(targets)
    }
}
