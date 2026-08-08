use destack_dir as dir;
use destack_dir::HeritageKind;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    ModuleQueryContext, NavigationTarget, ProgramQueryContext, QueryPosition, QueryRange,
    QueryResult, sort_and_dedup_navigation_targets,
};

/// A goto implementation request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoImplementationRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A goto implementation response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoImplementationResponse {
    /// Implementation targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Find implementations of the symbol at the given position.
    pub fn goto_implementation(
        &self,
        request: GotoImplementationRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<GotoImplementationResponse> {
        // FUGU #Incomplete: index MemberConformance for implementations and overrides
        let position = request.position;
        let Some(symbol) = self.symbol_at_offset(program, position.file_id, position.offset)?
        else {
            return Ok(GotoImplementationResponse {
                targets: Vec::new(),
            });
        };
        let origin = QueryRange {
            module: self.module(),
            span: symbol.span,
        };
        let mut targets = Vec::new();

        // collect implementations for every exact declaration named by the occurrence
        for symbol_id in symbol.symbols {
            for canonical_id in program.canonical_symbols(symbol_id)? {
                let target_module = program.module(canonical_id.module_id)?;
                let symbols = target_module.bindings()?;
                let symbol = symbols.get_symbol(canonical_id.local_id);

                let heritage_kind = match symbol.kind {
                    dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface => {
                        HeritageKind::Implements
                    }
                    dir::SymbolKind::Class => HeritageKind::Extends,
                    _ => continue,
                };

                // match cached direct edges against this canonical declaration
                for entry in program.base_heritage(canonical_id)? {
                    if entry.kind == heritage_kind {
                        let declaration_module = program.module(entry.declaration.module_id)?;
                        let target = declaration_module.navigation_target(
                            program,
                            entry.declaration,
                            origin,
                        )?;
                        targets.push(target);
                    }
                }
            }
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(GotoImplementationResponse { targets })
    }
}
