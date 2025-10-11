use crate::Compiler;
use dyst_ast::{self as ast};
use dyst_dir::{Annotation, AnnotationPosition, NodeId};
use dyst_package::DocumentBody;
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Attach all annotations..
    pub fn attach_all_annotations(&mut self) {
        for document in self.workspace.documents() {
            let DocumentBody::Text { ast, .. } = &document.body else {
                continue;
            };
            self.attach_annotations(document.id, ast);
        }
    }

    /// Lower and attach all annotations for a source.
    pub fn attach_annotations(&mut self, source_id: SourceId, ast: &ast::NodeTree) {
        // lower them
        for ast_annotation_id in ast.get_nodes::<ast::Annotation>() {
            self.lower_annotation(source_id, ast, ast_annotation_id);
        }

        // attach them
        for (ast_node_id, ast_annotations) in ast.get_all_annotations() {
            let Some(dir_node_id) = self.tree.get_node_id_by_ast_id(source_id, *ast_node_id) else {
                continue;
            };
            for ast_annotation_id in ast_annotations {
                let dir_annotation_id = self
                    .tree
                    .get_node_id_by_ast_id(source_id, ast_annotation_id.id)
                    .unwrap_or_else(|| {
                        panic!("annotation node not found: {source_id:?}/{ast_annotation_id:?}")
                    });
                self.tree
                    .append_annotation(dir_node_id, NodeId::new(dir_annotation_id));
            }
        }
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
        let annotation = match annotation {
            ast::Annotation::Blank { node, position } => {
                let blank = ast.get(*node);
                let position = self.lower_annotation_position(*position);
                let lines = blank.lines;
                Annotation::Blank { position, lines }
            }
            ast::Annotation::Doc { node, position } => {
                let doc = ast.get(*node);
                let position = self.lower_annotation_position(*position);
                let string = self.intern_string(source_id, doc.string);
                Annotation::Doc { position, string }
            }
            ast::Annotation::Comment { node, position } => {
                let comment = ast.get(*node);
                let position = self.lower_annotation_position(*position);
                let string = self.intern_string(source_id, comment.string);
                Annotation::Comment { position, string }
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
                Annotation::Tag {
                    position,
                    receiver,
                    arguments,
                }
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
                Annotation::Decorator {
                    position,
                    receiver,
                    arguments,
                }
            }
        };
        self.tree.insert(annotation, source_id, annotation_id)
    }
}
