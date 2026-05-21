use destack_dir as dir;

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Collect dependency edges from one DIR view.
    pub(in crate::import) fn collect_dependencies(
        &self,
        state: &mut ImportState<'_>,
        roots: &[dir::LocalNodeId<dir::Expression>],
    ) -> CompilerResult<()> {
        // scan active expressions
        for root in roots {
            let expression = state.view.get(*root);
            self.collect_expression_dependencies(state, *root, expression)?;
        }

        Ok(())
    }
}
