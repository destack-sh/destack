use dyst_dir::{Destination, NodeIdAny};

use crate::{Compiler, EvaluateResult};

impl<'a> Compiler<'a> {
    /// Evaluate a Destination.
    pub fn evaluate_destination(
        &mut self,
        scope_id: NodeIdAny,
        destination: Destination,
    ) -> EvaluateResult<Destination> {
        todo!("evaluate_destination({destination:?})")
    }
}
