use crate::Compiler;
use dyst_ast::{self as ast};
use dyst_dir::{
    Annotation, AnnotationPosition, LocalNodeId, LocalScopeId, Module, NodeTree, SymbolTable,
    TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind and attach all annotations for a module.
    pub fn attach_annotations(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) {
        // bind them
        for ast_annotation_id in module.get_nodes::<ast::Annotation>() {
            self.bind_annotation(module, scope_id, ast_annotation_id, tree, symbols, types);
        }

        // attach them
        for (ast_node_id, ast_annotations) in module.ast.get_all_annotations() {
            let Some(dir_node_id) = tree.get_node_id_by_source_id(*ast_node_id) else {
                continue;
            };
            for ast_annotation_id in ast_annotations {
                let Some(dir_annotation_id) = tree.get_node_id_by_source_id(ast_annotation_id.id)
                else {
                    continue; // skipped by bind_annotation
                };
                tree.append_annotation(dir_node_id, LocalNodeId::new(dir_annotation_id));
            }
        }
    }

    /// Bind an annotation position into a DIR annotation position.
    pub(super) fn bind_annotation_position(
        &self,
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

    /// Bind an annotation to a DIR annotation.
    pub(super) fn bind_annotation(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        annotation_id: ast::LocalNodeId<ast::Annotation>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalNodeId<Annotation>> {
        let annotation = module.get(annotation_id);
        let annotation = match annotation {
            ast::Annotation::Blank { .. } => {
                return None;
            }
            ast::Annotation::Doc { node, position } => {
                let doc = module.get(*node);
                let position = self.bind_annotation_position(*position);
                let string = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, doc.string);
                Annotation::Doc { position, string }
            }
            ast::Annotation::Comment { node, position } => {
                let comment = module.get(*node);
                let position = self.bind_annotation_position(*position);
                let string = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, comment.string);
                Annotation::Comment { position, string }
            }
            ast::Annotation::Tag { node, position } => {
                let tag = module.get(*node);
                let position = self.bind_annotation_position(*position);
                let left = self.bind_path(module, &tag.left);
                let arguments = tag.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(module, scope_id, *argument, tree, symbols, types)
                        })
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
                let position = self.bind_annotation_position(*position);
                let left = self.bind_path(module, &decorator.left);
                let arguments = decorator.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(module, scope_id, *argument, tree, symbols, types)
                        })
                        .collect()
                });
                Annotation::UnresolvedDecorator {
                    position,
                    left,
                    arguments,
                }
            }
        };
        Some(tree.insert_from_source(annotation, annotation_id, scope_id))
    }
}
