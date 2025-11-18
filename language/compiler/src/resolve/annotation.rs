use dyst_dir::{Annotation, ModuleId, NodeId};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Annotation.
    pub fn resolve_annotation(
        &mut self,
        _module_id: ModuleId,
        annotation_id: NodeId<Annotation>,
    ) -> ResolveResult<()> {
        let _annotation = self.session.tree.get(annotation_id);
        // todo!("resolve_annotation({annotation:?})");
        Ok(())
    }
}
