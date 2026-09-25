use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_dir::HeritageKind;
use tspp_serde::Reflect;

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
        let position = request.position;
        let Some(symbol) = self
            .cursor(position.file_id, position.offset)?
            .symbol(program)?
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
            for target_id in program.symbol_targets(symbol_id)? {
                // collect exact member implementations
                for implementation in program.member_implementations(target_id)? {
                    let module = program.module(implementation.module_id)?;
                    let target = module.navigation_target(program, implementation, origin)?;
                    targets.push(target);
                }

                // collect nominal implementations and subclasses
                let target_module = program.module(target_id.module_id)?;
                let symbols = target_module.bindings()?;
                let symbol = symbols.get_symbol(target_id.local_id);

                let heritage_kind = match symbol.kind {
                    dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface => {
                        HeritageKind::Implements
                    }
                    dir::SymbolKind::Class => HeritageKind::Extends,
                    _ => continue,
                };

                // match direct heritage edges against this target declaration
                for entry in program.base_heritage(target_id)? {
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
