use crate::Compiler;
use destack_artifact::Ast;
use destack_ast::{self as ast};
use destack_dir::{
    Annotation, AnnotationPosition, Documentation, LocalNodeId, LocalNodeIdAny, LocalScopeId,
    LocalScopeMark, ModuleBinding, NodeTree, NodeType, SymbolSpaceOrder, SymbolTable, TypeTable,
};
use destack_source::File;
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Derive normalized documentation text from raw AST comments and attach it to DIR nodes.
    pub(super) fn attach_documentation(&self, module: &Module, ast: &Ast, tree: &mut NodeTree) {
        let file = self.program.files.get(module.file_id);
        let node_ids: Vec<_> = tree.iter_node_ids().collect();

        // scan every source-backed dir node once
        for node_id in node_ids {
            if node_id.ty == NodeType::Annotation || tree.has_documentation(node_id.id) {
                continue;
            }

            let source_id = tree.get_source(node_id.id);
            let source_span = ast.tree.get_span_by_id(source_id);
            let Some(documentation) =
                source_node_documentation(ast, file.as_ref(), source_span.start)
            else {
                continue;
            };

            let documentation = Documentation {
                text: self.program.strings.intern(&documentation),
            };
            tree.set_documentation(node_id.id, documentation);
        }
    }

    /// Bind and attach all annotations for a module.
    pub(super) fn attach_annotations(
        &self,
        module: &Module,
        ast: &Ast,
        scope: (LocalScopeId, LocalScopeMark),
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
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
                namespace_scope,
                global_augmentation_scope,
                module_bindings,
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
        self.attach_documentation(module, ast, tree);
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
        ast: &Ast,
        scope: (LocalScopeId, LocalScopeMark),
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        ast_annotation_id: ast::LocalNodeId<ast::Annotation>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalNodeId<Annotation>> {
        let ast_annotation = ast.tree.get(ast_annotation_id);
        let annotation_id =
            tree.reserve_from_source(NodeType::Annotation, ast_annotation_id.id, scope, parent_id);
        let annotation = match ast_annotation {
            ast::Annotation::Decorator { node, position } => {
                let decorator = ast.tree.get(*node);
                let position = self.bind_annotation_position(*position);
                let expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    decorator.expression,
                    Some(annotation_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueThenType,
                );
                Annotation::Decorator {
                    position,
                    expression,
                }
            }
        };
        Some(tree.insert(annotation_id, annotation))
    }
}

/// Collect normalized documentation attached to one source start offset.
fn source_node_documentation(ast: &Ast, file: &File, source_start: u32) -> Option<String> {
    let mut lines = Vec::new();

    // collect all leading doc comments attached to this source start
    for comment in ast.tree.comments().iter().copied() {
        if !comment.is_leading() || comment.attached_to != source_start {
            continue;
        }

        let raw_text = file.span_str(comment.span);
        let raw_text = raw_text.trim_start();
        if !raw_text.starts_with("///") && !raw_text.starts_with("/**") {
            continue;
        }

        let text = ast::normalize_comment_payload(raw_text);
        for line in text.lines() {
            lines.push(line.trim_end().to_string());
        }
    }

    // trim empty outer lines while preserving inner paragraph breaks
    let Some(start) = lines.iter().position(|line| !line.trim().is_empty()) else {
        return None;
    };
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .unwrap_or(start);

    Some(lines[start..=end].join("\n"))
}
