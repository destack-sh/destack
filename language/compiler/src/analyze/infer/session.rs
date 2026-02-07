use crate::{AnalyzeOptions, InferContext};
use destack_dir::InferTable;
use destack_workspace::ProfileId;

/// Stage-owned mutable state for one infer task execution.
#[derive(Debug)]
pub struct InferSession {
    /// Inference variables and constraints for this infer run.
    table: InferTable,
    /// Contextual and flow-sensitive infer state for this infer run.
    context: InferContext,
}

impl InferSession {
    /// Create a fresh infer session for one module/profile run.
    pub fn new(profile: ProfileId, options: AnalyzeOptions) -> Self {
        // initialize infer-owned state
        Self {
            table: InferTable::default(),
            context: InferContext::new(profile, options),
        }
    }

    /// Borrow the infer table.
    pub fn table(&self) -> &InferTable {
        &self.table
    }

    /// Borrow the infer table mutably.
    pub fn table_mut(&mut self) -> &mut InferTable {
        &mut self.table
    }

    /// Borrow the infer context.
    pub fn context(&self) -> &InferContext {
        &self.context
    }

    /// Borrow the infer context mutably.
    pub fn context_mut(&mut self) -> &mut InferContext {
        &mut self.context
    }

    /// Borrow infer table and context mutably as disjoint fields.
    pub fn parts_mut(&mut self) -> (&mut InferTable, &mut InferContext) {
        (&mut self.table, &mut self.context)
    }
}
