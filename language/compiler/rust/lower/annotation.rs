use crate::Compiler;
use dyst_ast::{self as ast};
use dyst_dir::{Annotation, AnnotationPosition, Module, NodeId};

impl<'a> Compiler<'a> {
    /// Lower and attach all annotations for a module.
    pub fn attach_annotations(&mut self, module: &Module) {
        // lower them
        for ast_annotation_id in module.get_nodes::<ast::Annotation>() {
            self.lower_annotation(module, ast_annotation_id);
        }

        // attach them
        for (ast_node_id, ast_annotations) in module.ast.get_all_annotations() {
            let Some(dir_node_id) = self.tree.get_node_id_by_ast_id(module.id, *ast_node_id) else {
                continue;
            };
            for ast_annotation_id in ast_annotations {
                let Some(dir_annotation_id) = self
                    .tree
                    .get_node_id_by_ast_id(module.id, ast_annotation_id.id)
                else {
                    continue; // skipped by lower_annotation
                };
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
    fn lower_annotation(
        &mut self,
        module: &Module,
        annotation_id: ast::NodeId<ast::Annotation>,
    ) -> Option<NodeId<Annotation>> {
        let annotation = module.get(annotation_id);
        let annotation = match annotation {
            ast::Annotation::Blank { .. } => {
                return None;
            }
            ast::Annotation::Doc { node, position } => {
                let doc = module.get(*node);
                let position = self.lower_annotation_position(*position);
                let string = self.strings.intern_from(&module.strings, doc.string);
                Annotation::Doc { position, string }
            }
            ast::Annotation::Comment { node, position } => {
                let comment = module.get(*node);
                let position = self.lower_annotation_position(*position);
                let string = self.strings.intern_from(&module.strings, comment.string);
                Annotation::Comment { position, string }
            }
            ast::Annotation::Tag { node, position } => {
                let tag = module.get(*node);
                let position = self.lower_annotation_position(*position);
                let receiver = self.lower_path(module, &tag.receiver);
                let arguments = tag.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                Annotation::Tag {
                    position,
                    receiver,
                    arguments,
                }
            }
            ast::Annotation::Decorator { node, position } => {
                let decorator = module.get(*node);
                let position = self.lower_annotation_position(*position);
                let receiver = self.lower_path(module, &decorator.receiver);
                let arguments = decorator.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                Annotation::Decorator {
                    position,
                    receiver,
                    arguments,
                }
            }
        };
        Some(
            self.tree
                .insert_from_ast(annotation, module.id, annotation_id),
        )
    }
}
