use dyst_dir::{Annotation, NodeId};

use crate::{Compiler, EvaluateResult};

impl<'a> Compiler<'a> {
    /// Evaluate an Annotation.
    pub fn evaluate_annotation(&mut self, annotation_id: NodeId<Annotation>) -> EvaluateResult<()> {
        let _annotation = self.tree.get(annotation_id);
        // todo!("evaluate_annotation({annotation:?})");
        Ok(())
    }
}
