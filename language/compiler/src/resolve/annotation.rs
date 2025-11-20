use dyst_dir::{Annotation, ModuleId, NodeId, NodeTree};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Annotation.
    pub fn resolve_annotation(
        &self,
        _module_id: ModuleId,
        annotation_id: NodeId<Annotation>,
        tree: &mut NodeTree,
    ) -> ResolveResult<()> {
        let _annotation = tree.get(annotation_id);
        // todo!("resolve_annotation({annotation:?})");
        Ok(())
    }
}
