use crate::{
    Annotation, AnnotationPosition, CodegenJsError, CodegenJsResult, LocalNodeId, ModuleLowerer,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a DIR annotation position into a JS annotation position.
    pub fn lower_annotation_position(
        &self,
        position: dir::AnnotationPosition,
    ) -> AnnotationPosition {
        match position {
            dir::AnnotationPosition::Prefix => AnnotationPosition::Prefix,
            dir::AnnotationPosition::Infix => AnnotationPosition::Infix,
            dir::AnnotationPosition::Postfix => AnnotationPosition::Postfix,
        }
    }

    /// Lower an annotation from DIR into JS AST.
    pub fn lower_annotation(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        annotation_id: dir::LocalNodeId<dir::Annotation>,
    ) -> CodegenJsResult<LocalNodeId<Annotation>> {
        let annotation = self.dir_tree.get(annotation_id);
        let annotation = match annotation {
            dir::Annotation::Doc { position, string } => {
                let position = self.lower_annotation_position(*position);
                let string = self.strings.intern_from(&self.ast.strings, *string);
                Annotation::Doc { position, string }
            }
            dir::Annotation::Comment { position, string } => {
                let position = self.lower_annotation_position(*position);
                let string = self.strings.intern_from(&self.ast.strings, *string);
                Annotation::Comment { position, string }
            }
            dir::Annotation::Decorator {
                position: _,
                left: _,
                arguments: _,
            } => {
                // NOTE #Incomplete: properly generate JS decorators after elaborate phase
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: annotation_id.into_global_any(self.module.id),
                    message: None,
                });
            }
        };
        let annotation_id = self
            .tree
            .insert_from_source(annotation, self.module.id, annotation_id);
        Ok(annotation_id)
    }
}
