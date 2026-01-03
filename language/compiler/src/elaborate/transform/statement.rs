use destack_dir::NodeTree;

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Normalize value-position control flow into explicit statements.
    pub(super) fn transform_normalize_value_expressions(
        &self,
        _tree: &mut NodeTree,
    ) -> ElaborateResult<()> {
        // TODO #Incomplete: normalize if, match, block, sequence, and labeled break values
        Ok(())
    }
}
