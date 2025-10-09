use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Annotation, AnnotationPosition, NodeId};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower and attach all annotations for a source.
    pub fn attach_annotations(&mut self, source_id: SourceId, ast: &ast::NodeTree) {
        todo!("Compiler::attach_annotations")
    }

    /// Lower an annotation position into a DIR annotation position.
    pub fn lower_annotation_position(
        &mut self,
        annotation_position: ast::AnnotationPosition,
    ) -> AnnotationPosition {
        match annotation_position {
            ast::AnnotationPosition::BlockInfix => AnnotationPosition::Infix,
            ast::AnnotationPosition::BlockPrefix => AnnotationPosition::Prefix,
            ast::AnnotationPosition::BlockPostfix => AnnotationPosition::Postfix,
            ast::AnnotationPosition::LinePrefix => AnnotationPosition::Prefix,
            ast::AnnotationPosition::LinePostfix => AnnotationPosition::Postfix,
            ast::AnnotationPosition::LinePostfixBoundary => AnnotationPosition::Postfix,
        }
    }

    /// Lower an annotation to a DIR annotation.
    pub fn lower_annotation(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        annotation_id: ast::NodeId<ast::Annotation>,
    ) -> NodeId<Annotation> {
        let annotation = ast.get(annotation_id);
        let position = self.lower_annotation_position(annotation.position());
        match annotation {
            ast::Annotation::Blank { node, position } => {
                let blank = ast.get(*node);
                self.tree.allocate(
                    Annotation::Blank {
                        position,
                        lines: blank.lines,
                    },
                    source_id,
                    annotation_id,
                )
            }
            _ => todo!(),
        }
    }
}
