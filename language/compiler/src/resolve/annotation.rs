use dyst_dir::{Annotation, LocalNodeId, Module, NodeTree, SymbolTable};

use crate::{Compiler, ResolveResult};

impl Compiler {
    /// Resolve an Annotation.
    pub fn resolve_annotation(
        &self,
        _module: &Module,
        annotation_id: LocalNodeId<Annotation>,
        tree: &mut NodeTree,
        _symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        let _annotation = tree.get(annotation_id);
        // todo!("resolve_annotation({annotation:?})");
        Ok(())
    }
}
