use crate::Compiler;
use dyst_ast::{self as ast};
use dyst_dir::{Annotation, AnnotationPosition, Module, NodeId, ScopeId};

impl<'a> Compiler<'a> {
    /// Lower and attach all annotations for a module.
    pub fn attach_annotations(&mut self, module: &Module, scope_id: ScopeId) {
        // lower them
        for ast_annotation_id in module.get_nodes::<ast::Annotation>() {
            self.lower_annotation(module, scope_id, ast_annotation_id);
        }

        // attach them
        let mut tree = self.session.tree.write();
        for (ast_node_id, ast_annotations) in module.ast.get_all_annotations() {
            let Some(dir_node_id) = tree.get_node_id_by_source_id(module.id, *ast_node_id) else {
                continue;
            };
            for ast_annotation_id in ast_annotations {
                let Some(dir_annotation_id) =
                    tree.get_node_id_by_source_id(module.id, ast_annotation_id.id)
                else {
                    continue; // skipped by lower_annotation
                };
                tree.append_annotation(dir_node_id, NodeId::new(dir_annotation_id),);
            }
        }
    }

    /// Lower an annotation position into a DIR annotation position.
    pub(super) fn lower_annotation_position(
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
        scope_id: ScopeId,
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
                let string = self
                    .session
                    .strings
                    .intern_from(&module.strings, doc.string);
                Annotation::Doc { position, string }
            }
            ast::Annotation::Comment { node, position } => {
                let comment = module.get(*node);
                let position = self.lower_annotation_position(*position);
                let string = self
                    .session
                    .strings
                    .intern_from(&module.strings, comment.string);
                Annotation::Comment { position, string }
            }
            ast::Annotation::Tag { node, position } => {
                let tag = module.get(*node);
                let position = self.lower_annotation_position(*position);
                let left = self.lower_path(module, &tag.left);
                let arguments = tag.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, scope_id, *argument))
                        .collect()
                });
                Annotation::UnresolvedTag {
                    position,
                    left,
                    arguments,
                }
            }
            ast::Annotation::Decorator { node, position } => {
                let decorator = module.get(*node);
                let position = self.lower_annotation_position(*position);
                let left = self.lower_path(module, &decorator.left);
                let arguments = decorator.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, scope_id, *argument))
                        .collect()
                });
                Annotation::UnresolvedDecorator {
                    position,
                    left,
                    arguments,
                }
            }
        };
        Some(
            self.session
                .tree
                .insert_from_source(annotation, module.id, annotation_id),
        )
    }
}
