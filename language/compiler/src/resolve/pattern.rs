use dyst_dir::{ModuleId, LocalNodeId, NodeTree, PatternField};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a PatternField.
    pub(super) fn resolve_pattern_field(
        &self,
        _module_id: ModuleId,
        pattern_field_id: LocalNodeId<PatternField>,
        _tree: &mut NodeTree,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: pattern_field_id.into(),
        })
    }
}
