use destack_dir as dir;
use destack_dir::HeritageKind;
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    ModuleQueryContext, NavigationTarget, ProgramQueryContext, QueryPosition, QueryRange,
    QueryResult, sort_and_dedup_navigation_targets,
};

/// Request goto implementation at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoImplementationRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for goto implementation queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoImplementationResponse {
    /// Implementation targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find implementations of the symbol at the given position.
    ///
    /// Interfaces return implementing classes.
    /// Classes return subclasses.
    pub fn goto_implementation(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Vec<NavigationTarget>> {
        let Some(symbol) = self.symbol_at_offset(file_id, offset)? else {
            return Ok(Vec::new());
        };
        let origin = QueryRange {
            module: self.module(),
            span: symbol.span,
        };
        let Some(symbol_id) = symbol.symbol() else {
            return Ok(Vec::new());
        };
        let Some(canonical_id) = program.canonical_symbol(symbol_id)? else {
            return Ok(Vec::new());
        };
        let target_module = program.module(canonical_id.module_id)?;

        // FUGU #Incomplete: index exact checked member implementation relations
        let heritage_kind = {
            let symbols = target_module.symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);

            match symbol.kind {
                dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface => {
                    HeritageKind::Implements
                }
                dir::SymbolKind::Class => HeritageKind::Extends,
                _ => return Ok(Vec::new()),
            }
        };

        let mut targets = Vec::new();

        // match cached direct edges against the canonical target
        for entry in program.base_heritage(canonical_id)? {
            if entry.kind == heritage_kind {
                let declaration_module = program.module(entry.declaration.module_id)?;
                let target = declaration_module.navigation_target(entry.declaration, origin)?;
                targets.push(target);
            }
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(targets)
    }
}
