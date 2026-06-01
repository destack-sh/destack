use destack_dir as dir;

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Collect module edges from one DIR view.
    pub(in crate::import) fn collect_modules(
        &self,
        state: &mut ImportState<'_>,
        roots: &[dir::LocalNodeId<dir::Expression>],
    ) -> CompilerResult<()> {
        // scan active expressions
        for root in roots {
            state.stats.roots += 1;

            let expression = state.view.get(*root);
            self.collect_expression_modules(state, *root, expression)?;
        }

        Ok(())
    }
}
