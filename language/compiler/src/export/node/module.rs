use destack_dir as dir;

use crate::export::state::ExportState;
use crate::{Compiler, ExportResult};

impl Compiler {
    /// Collect export entries from one DIR view.
    pub(in crate::export) fn collect_exports(
        &self,
        state: &mut ExportState<'_>,
        roots: &[dir::LocalNodeId<dir::Expression>],
    ) -> ExportResult<()> {
        // collect declaration exports
        self.collect_declaration_exports(state)?;

        // scan explicit export declarations
        for root in roots {
            let expression = state.view.get(*root);
            self.collect_expression_exports(state, *root, expression)?;
        }

        Ok(())
    }
}
