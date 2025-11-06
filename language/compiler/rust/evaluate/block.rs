use dyst_dir::{BlockTarget, NodeIdAny};

use crate::{Compiler, EvaluateResult};

impl<'a> Compiler<'a> {
    /// Evaluate a BlockTarget.
    pub fn evaluate_target(
        &mut self,
        _scope_id: NodeIdAny,
        destination: BlockTarget,
    ) -> EvaluateResult<BlockTarget> {
        todo!("evaluate_target({destination:?})")
    }
}
