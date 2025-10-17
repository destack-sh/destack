use dyst_dir::{Annotation, NodeId};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Annotation.
    pub fn resolve_annotation(&mut self, annotation_id: NodeId<Annotation>) -> ResolveResult<()> {
        let annotation = self.tree.get(annotation_id);
        // todo!("resolve_annotation({annotation:?})");
        Ok(())
    }
}
