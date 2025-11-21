use dyst_dir::{LocalNodeId, ModuleId, NodeTree, PatternField, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a PatternField.
    pub(super) fn resolve_pattern_field(
        &self,
        module_id: ModuleId,
        pattern_field_id: LocalNodeId<PatternField>,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: pattern_field_id.into_global_any(module_id),
        })
    }
}
