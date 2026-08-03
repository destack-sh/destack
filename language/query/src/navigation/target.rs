use destack_dir as dir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{ModuleQueryContext, QueryError, QueryRange, QueryResult, Target};

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
    pub(crate) fn symbol_target(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<Target> {
        // navigate generated symbols to their generating member declaration
        let Some(span) = self.symbol_local_declaration_span(symbol_id)? else {
            return self.generated_symbol_target(symbol_id);
        };
        let selection_span =
            self.symbol_local_definition_span(symbol_id)?
                .ok_or(QueryError::missing(format!(
                    "declaration selection: {symbol_id:?}"
                )))?;

        Target::new(self.module(), span).with_selection_span(selection_span)
    }

    /// Build one source target from a generated symbol's defining member.
    fn generated_symbol_target(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<Target> {
        let (_, _, member) = self
            .definitions()
            .member(symbol_id)
            .ok_or(QueryError::missing(format!(
                "generated symbol member: {symbol_id:?}"
            )))?;
        let source = member.source();
        let span = self
            .view()
            .get_span_by_id(source.local_id.id)
            .ok_or(QueryError::missing(format!(
                "generated member span: {source:?}"
            )))?;

        Ok(Target::new(self.module(), span))
    }

    /// Build one navigation target from an exact declaration symbol.
    pub(crate) fn navigation_target(
        &self,
        symbol_id: dir::GlobalSymbolId,
        origin: QueryRange,
    ) -> QueryResult<NavigationTarget> {
        let target = self.symbol_target(symbol_id)?;

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
