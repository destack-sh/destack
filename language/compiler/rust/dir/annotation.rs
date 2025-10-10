use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Annotation, AnnotationPosition, NodeId};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower and attach all annotations for a source.
    pub fn attach_annotations(&mut self, source_id: SourceId, ast: &ast::NodeTree) {
        todo!("Compiler::attach_annotations");
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
        match annotation {
            ast::Annotation::Blank { node, position } => {
                let blank = ast.get(*node);
                let position = self.lower_annotation_position(*position);
                let lines = blank.lines;
                self.tree.allocate(
                    Annotation::Blank { position, lines },
                    source_id,
                    annotation_id,
                )
            }
            ast::Annotation::Doc { node, position } => {
                let doc = ast.get(*node);
                let position = self.lower_annotation_position(*position);
                let string = self.intern_string(source_id, doc.string);
                self.tree.allocate(
                    Annotation::Doc { position, string },
                    source_id,
                    annotation_id,
                )
            }
            ast::Annotation::Comment { node, position } => {
                let comment = ast.get(*node);
                let position = self.lower_annotation_position(*position);
                let string = self.intern_string(source_id, comment.string);
                self.tree.allocate(
                    Annotation::Comment { position, string },
                    source_id,
                    annotation_id,
                )
            }
            ast::Annotation::Tag { node, position } => {
                let tag = ast.get(*node);
                let position = self.lower_annotation_position(*position);
                let receiver = self.lower_path(source_id, ast, &tag.receiver);
                let arguments = tag.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(source_id, ast, *argument))
                        .collect()
                });
                self.tree.allocate(
                    Annotation::Tag {
                        position,
                        receiver,
                        arguments,
                    },
                    source_id,
                    annotation_id,
                )
            }
            ast::Annotation::Decorator { node, position } => {
                let decorator = ast.get(*node);
                let position = self.lower_annotation_position(*position);
                let receiver = self.lower_path(source_id, ast, &decorator.receiver);
                let arguments = decorator.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(source_id, ast, *argument))
                        .collect()
                });
                self.tree.allocate(
                    Annotation::Decorator {
                        position,
                        receiver,
                        arguments,
                    },
                    source_id,
                    annotation_id,
                )
            }
        }
    }
}
