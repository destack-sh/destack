use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a DIR decorator position into a JS annotation position.
    pub fn lower_decorator_position(
        &self,
        position: dir::DecoratorPosition,
    ) -> js::AnnotationPosition {
        match position {
            dir::DecoratorPosition::BlockPrefix | dir::DecoratorPosition::LinePrefix => {
                js::AnnotationPosition::Prefix
            }
            dir::DecoratorPosition::BlockInfix => js::AnnotationPosition::Infix,
            dir::DecoratorPosition::BlockPostfix | dir::DecoratorPosition::LinePostfix => {
                js::AnnotationPosition::Postfix
            }
        }
    }

    /// Lower an annotation from DIR into JS AST.
    pub fn lower_annotation(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        annotation_id: dir::LocalNodeId<dir::Annotation>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Annotation>> {
        match self.dir_tree.get(annotation_id) {
            dir::Annotation::Decorator {
                position: _,
                expression: _,
            } => {
                // NOTE #Incomplete: properly generate JS decorators after elaborate phase
                //  (or would that be a special js::Expression::Decorated or something..?)
                Err(CodegenJsError::UnsupportedConstruct {
                    node: annotation_id.into_global_any(self.module.id),
                    message: None,
                })
            }
        }
    }

    /// Lower a decorator from DIR into JS AST.
    pub fn lower_decorator(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Annotation>> {
        let _ = self.dir_tree.get(decorator_id);

        // decorators are still rejected during JS lowering
        Err(CodegenJsError::UnsupportedConstruct {
            node: decorator_id.into_global_any(self.module.id),
            message: None,
        })
    }
}
