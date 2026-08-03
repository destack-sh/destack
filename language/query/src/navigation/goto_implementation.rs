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
        let mut targets = Vec::new();

        // collect implementations for every exact declaration named by the occurrence
        for symbol_id in symbol.symbols {
            for canonical_id in program.canonical_symbols(symbol_id)? {
                let target_module = program.module(canonical_id.module_id)?;
                let symbols = target_module.bindings()?;
                let symbol = symbols.get_symbol(canonical_id.local_id);

                // navigate interface members to their implementing declarations
                if let Some(declaration) = symbol.declaration.filter(|declaration| {
                    matches!(
                        declaration.local_id.ty,
                        dir::NodeType::Member | dir::NodeType::TypeMember
                    )
                }) {
                    let Some(owner) = symbols.symbol_owner(canonical_id.local_id) else {
                        continue;
                    };
                    let owner = owner.into_global(canonical_id.module_id);
                    self.collect_member_implementations(
                        program,
                        origin,
                        owner,
                        declaration,
                        &mut targets,
                    )?;

                    continue;
                }

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
                        let target =
                            declaration_module.navigation_target(entry.declaration, origin)?;
                        targets.push(target);
                    }
                }
            }
        }

        sort_and_dedup_navigation_targets(&mut targets);

        Ok(targets)
    }

    /// Collect declarations implementing one interface member requirement.
    fn collect_member_implementations(
        &self,
        program: &ProgramQueryContext<'_>,
        origin: QueryRange,
        owner: dir::GlobalSymbolId,
        requirement: dir::GlobalNodeIdAny,
        targets: &mut Vec<NavigationTarget>,
    ) -> QueryResult<()> {
        // build a navigation target for each implementing declaration
        let mut symbols = Vec::new();
        self.member_implementation_symbols(program, owner, requirement, &mut symbols)?;
        for symbol in symbols {
            let declaration_module = program.module(symbol.module_id)?;
            let target = declaration_module.navigation_target(symbol, origin)?;
            targets.push(target);
        }

        Ok(())
    }
}
