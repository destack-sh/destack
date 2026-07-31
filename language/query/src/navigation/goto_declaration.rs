use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    ModuleQueryContext, NavigationTarget, ProgramQueryContext, QueryPosition, QueryRange,
    QueryResult, sort_and_dedup_navigation_targets,
};

/// Request goto declaration at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDeclarationRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for goto declaration queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDeclarationResponse {
    /// Declaration targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find the declaration of the symbol at one position.
    pub fn goto_declaration(
        &self,
        query: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Vec<NavigationTarget>> {
        let Some(occurrence) = self.declaration_at_offset(file_id, offset)? else {
            return Ok(Vec::new());
        };
        let origin = QueryRange {
            module: self.module(),
            span: occurrence.span,
        };

        // collect each exact declaration target
        let mut targets = Vec::new();
        for symbol_id in occurrence.symbols {
            let module = query.module(symbol_id.module_id)?;
            let target = module.navigation_target(symbol_id, origin)?;

            targets.push(target);
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(targets)
    }
}
