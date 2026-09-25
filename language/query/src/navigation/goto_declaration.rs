use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    ModuleQueryContext, NavigationTarget, ProgramQueryContext, QueryPosition, QueryRange,
    QueryResult, sort_and_dedup_navigation_targets,
};

/// A goto declaration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDeclarationRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A goto declaration response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoDeclarationResponse {
    /// Declaration targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find the declaration of the symbol at one position.
    pub fn goto_declaration(
        &self,
        request: GotoDeclarationRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<GotoDeclarationResponse> {
        let position = request.position;
        let Some(occurrence) = self
            .cursor(position.file_id, position.offset)?
            .declaration(program)?
        else {
            return Ok(GotoDeclarationResponse {
                targets: Vec::new(),
            });
        };
        let origin = QueryRange {
            module: self.module(),
            span: occurrence.span,
        };

        // collect each exact declaration target
        let mut targets = Vec::new();
        for symbol_id in occurrence.symbols {
            let module = program.module(symbol_id.module_id)?;
            let target = module.navigation_target(program, symbol_id, origin)?;

            targets.push(target);
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(GotoDeclarationResponse { targets })
    }
}
