use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;

use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryRange, QueryResult, Target};

/// One navigation target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct NavigationTarget {
    /// The authored occurrence that initiated navigation.
    pub origin: QueryRange,
    /// The target source.
    pub target: Target,
    /// The resolved target symbol.
    pub symbol_id: dir::GlobalSymbolId,
}

impl ModuleQueryContext<'_> {
    /// Build one source target from an exact declaration symbol.
    pub(crate) fn declaration_target(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Target> {
        let span = self
            .symbol_local_declaration_span(program, symbol_id)?
            .ok_or(QueryError::missing(format!(
                "declaration target: {symbol_id:?}"
            )))?;
        let selection_span = self
            .symbol_local_definition_span(program, symbol_id)?
            .ok_or(QueryError::missing(format!(
                "declaration selection: {symbol_id:?}"
            )))?;

        Target::new(self.module(), span).with_selection_span(selection_span)
    }

    /// Build one navigation target from an exact declaration symbol.
    pub(crate) fn navigation_target(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        origin: QueryRange,
    ) -> QueryResult<NavigationTarget> {
        let target = self.declaration_target(program, symbol_id)?;

        Ok(NavigationTarget {
            origin,
            target,
            symbol_id,
        })
    }
}

/// Sort and deduplicate navigation targets.
pub(crate) fn sort_and_dedup_navigation_targets(targets: &mut Vec<NavigationTarget>) {
    targets.sort_by_key(|target| {
        (
            target.target.module.profile_id,
            target.target.module.module_id,
            target.target.span.file,
            target.target.span.start,
            target.target.span.end,
            target.symbol_id,
        )
    });
    targets.dedup();
}
