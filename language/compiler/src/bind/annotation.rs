use crate::Compiler;
use destack_ast::{self as ast};
use destack_dir::{
    Annotation, AnnotationPosition, Expression, LocalNodeId, LocalNodeIdAny, LocalScopeId,
    LocalScopeMark, NodeTree, NodeType, SymbolTable, TypeTable,
};
use destack_workspace::{Module, ModuleAst};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind and attach all annotations for a module.
    /// nocheckin TODO #Incomplete: attach DIR annotations in compiler
    pub fn attach_annotations(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) {
        // bind them
        for ast_annotation_id in ast.tree.get_nodes::<ast::Annotation>() {
            let ast_parent_id = ast.parents.get(ast_annotation_id);
            let dir_parent_id = ast_parent_id
                .and_then(|ast_parent_id| tree.get_node_id_by_source_id(ast_parent_id));
            self.bind_annotation(
                module,
                ast,
                scope,
                ast_annotation_id,
                dir_parent_id,
                tree,
                symbols,
                types,
            );
        }

        // attach them
        for (ast_node_id, ast_annotations) in ast.tree.get_all_annotations() {
            let Some(dir_node_id) = tree.get_node_id_by_source_id(*ast_node_id) else {
                continue;
            };
            for ast_annotation_id in ast_annotations {
                let Some(dir_annotation_id) = tree.get_node_id_by_source_id(ast_annotation_id.id)
                else {
                    continue; // skipped by bind_annotation
                };
                tree.append_annotation(dir_node_id, LocalNodeId::new(dir_annotation_id.id));
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
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        ast_annotation_id: ast::LocalNodeId<ast::Annotation>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalNodeId<Annotation>> {
        let ast_annotation = ast.tree.get(ast_annotation_id);

        // skip blank annotations without reserving
        if matches!(ast_annotation, ast::Annotation::Blank { .. }) {
            return None;
        }

        let annotation_id =
            tree.reserve_from_source(NodeType::Annotation, ast_annotation_id.id, scope, parent_id);
        let annotation = match ast_annotation {
            ast::Annotation::Blank { .. } => unreachable!(),
            ast::Annotation::Doc { node, position } => {
                let doc = ast.tree.get(*node);
                let position = self.bind_annotation_position(*position);
                let string = self.program.strings.intern_from(&ast.strings, doc.string);
                Annotation::Doc { position, string }
            }
            ast::Annotation::Comment { node, position } => {
                let comment = ast.tree.get(*node);
                let position = self.bind_annotation_position(*position);
                let string = self
                    .program
                    .strings
                    .intern_from(&ast.strings, comment.string);
                Annotation::Comment { position, string }
            }
            ast::Annotation::Decorator { node, position } => {
                let decorator = ast.tree.get(*node);
                let position = self.bind_annotation_position(*position);

                // bind the path as a Path expression
                let path = self.bind_path(module, ast, &decorator.left);
                let left_id = tree.reserve_from_source(
                    NodeType::Expression,
                    node.id, // use decorator node as source
                    scope,
                    Some(annotation_id),
                );
                tree.insert(
                    left_id,
                    Expression::UnresolvedPath {
                        path,
                        static_arguments: None,
                    },
                );
                let left = LocalNodeId::new(left_id.id);

                let arguments = decorator.arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(
                                module,
                                ast,
                                scope,
                                *argument,
                                Some(annotation_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                Annotation::Decorator {
                    position,
                    left,
                    arguments,
                }
            }
        };
        Some(tree.insert(annotation_id, annotation))
    }
}
