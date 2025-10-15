use dyst_dir::{Destination, NodeIdAny};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Evaluate a Destination.
    pub fn evaluate_destination(
        &mut self,
        scope_id: NodeIdAny,
        destination: Destination,
    ) -> Destination {
        todo!("evaluate_destination({destination:?})")
    }
}
