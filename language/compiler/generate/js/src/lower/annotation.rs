use crate::{Annotation, CodegenJsError, CodegenJsResult, LocalNodeId, ModuleLowerer};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower an annotation from DIR into JS AST.
    pub fn lower_annotation(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        annotation_id: dir::LocalNodeId<dir::Annotation>,
    ) -> CodegenJsResult<LocalNodeId<Annotation>> {
        // NOTE #Incomplete: properly generate JS decorators after elaborate phase
        //  (or would that be a special js::Expression::Decorated or something..?)
        Err(CodegenJsError::UnsupportedConstruct {
            node: annotation_id.into_global_any(self.module.id),
            message: None,
        })
    }
}
