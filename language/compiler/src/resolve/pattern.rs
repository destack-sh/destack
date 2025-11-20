use dyst_dir::{ModuleId, NodeId, NodeTree, PatternField};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a PatternField.
    pub fn resolve_pattern_field(
        &self,
        _module_id: ModuleId,
        pattern_field_id: NodeId<PatternField>,
        _tree: &mut NodeTree,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: pattern_field_id.into(),
        })
    }
}
