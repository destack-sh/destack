use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a DIR annotation position into a JS annotation position.
    pub fn lower_annotation_position(
        &self,
        position: dir::AnnotationPosition,
    ) -> js::AnnotationPosition {
        match position {
            dir::AnnotationPosition::Prefix => js::AnnotationPosition::Prefix,
            dir::AnnotationPosition::Infix => js::AnnotationPosition::Infix,
            dir::AnnotationPosition::Postfix => js::AnnotationPosition::Postfix,
        }
    }
    /// Lower an annotation from DIR into JS AST.
    pub fn lower_annotation(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        annotation_id: dir::LocalNodeId<dir::Annotation>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Annotation>> {
        let annotation = self.dir_tree.get(annotation_id);
        let annotation = match annotation {
            dir::Annotation::Doc { position, string } => {
                let position = self.lower_annotation_position(*position);
                let string = self.strings.intern_from(self.source_strings, *string);
                js::Annotation::Doc { position, string }
            }
            dir::Annotation::Decorator {
                position: _,
                expression: _,
            } => {
                // NOTE #Incomplete: properly generate JS decorators after elaborate phase
                //  (or would that be a special js::Expression::Decorated or something..?)
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
