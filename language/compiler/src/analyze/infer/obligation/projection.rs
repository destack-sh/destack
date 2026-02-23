use crate::analyze::StaticMemberSymbolKind;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{GlobalSymbolId, NodeTree, SymbolTable};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Return true when one projection needs an associated comptime obligation.
    pub(crate) fn projection_requires_associated_comptime_obligation(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        receiver_has_static_arguments: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<bool> {
        // check member kind metadata when available
        let kind = self
            .query_static_member_symbol_kind_for_symbol(
                module,
                profile,
                member_symbol,
                tree,
                symbols,
            )
            .map_err(AnalyzeError::from)?;
        if matches!(kind, Some(StaticMemberSymbolKind::AssociatedComptimeConst)) {
            return Ok(true);
        }

        // keep obligations when kind metadata is not available yet
        if kind.is_none() && receiver_has_static_arguments {
            return Ok(true);
        }

        Ok(false)
    }
}
