use destack_dir::{Annotation, LocalNodeId, Module, NodeTree, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

impl Compiler {
    /// Resolve an Annotation.
    pub fn resolve_annotation(
        &self,
        module: &Module,
        annotation_id: LocalNodeId<Annotation>,
        _tree: &mut NodeTree,
        _symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        // TODO #Incomplete: resolve annotations
        Err(ResolveError::UnsupportedNode {
            node: annotation_id.into_global_any(module.id),
        })
    }
}
