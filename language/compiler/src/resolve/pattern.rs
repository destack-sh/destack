use dyst_dir::{ModuleId, NodeId, Pattern, PatternField};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Pattern.
    pub fn resolve_pattern(
        &mut self,
        _module_id: ModuleId,
        pattern_id: NodeId<Pattern>,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: pattern_id.into(),
        })
    }

    /// Resolve a PatternField.
    pub fn resolve_pattern_field(
        &mut self,
        _module_id: ModuleId,
        pattern_field_id: NodeId<PatternField>,
    ) -> ResolveResult<()> {
        Err(ResolveError::UnsupportedNode {
            node: pattern_field_id.into(),
        })
    }
}
