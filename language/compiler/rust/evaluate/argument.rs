use dyst_dir::{Argument, NodeId};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Evaluate an Argument.
    pub fn evaluate_argument(&mut self, argument_id: NodeId<Argument>) {
        let argument = self.tree.get(argument_id);
        todo!("evaluate_argument({argument:?})")
    }
}
