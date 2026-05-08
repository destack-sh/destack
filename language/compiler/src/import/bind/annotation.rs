use crate::Compiler;
use destack_artifact::Ast;
use destack_ast::{self as ast, StringId};
use destack_dir::{
    DeclaredModule, Decorator, DecoratorPosition, Documentation, LocalNodeId, LocalNodeIdAny,
    LocalScopeId, LocalScopeMark, NodeType, SymbolSpace, SymbolTable, Tree, TypeTable,
};
use destack_source::File;
use destack_workspace::{Module, ProviderContext};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Derive normalized documentation text from raw AST comments and attach it to DIR nodes.
    pub(super) fn attach_documentation(
        &self,
        module: &Module,
        ast: &Ast,
        tree: &mut Tree,
        context: &dyn ProviderContext,
    ) {
        let file = self.file(context, module.file_id);
        let node_ids: Vec<_> = tree.iter_node_ids().collect();

        // scan every source-backed dir node once
        for node_id in node_ids {
            if node_id.ty == NodeType::Decorator || tree.has_documentation(node_id.id) {
                continue;
            }

            let source_id = tree.get_source(node_id.id);
            let source_span = ast.tree.get_span_by_id(source_id);
            let documentation = source_node_documentation(ast, file.as_ref(), source_span.start)
                .or_else(|| {
                    tree.get_decorators(node_id.id)
                        .first()
                        .copied()
                        .and_then(|decorator_id| {
                            let source_id = tree.get_source(decorator_id.id);
                            let source_span = ast.tree.get_span_by_id(source_id);

                            source_node_documentation(ast, file.as_ref(), source_span.start)
                        })
                });
            let Some(documentation) = documentation else {
                continue;
            };

            let documentation = Documentation {
                text: StringId::for_text(&documentation),
            };
            tree.set_documentation(node_id.id, documentation);
        }
    }

    /// Bind and attach all decorators for a module.
    pub(super) fn attach_annotations(
        &self,
        module: &Module,
        ast: &Ast,
        scope: (LocalScopeId, LocalScopeMark),
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        context: &dyn ProviderContext,
    ) {
        // bind and attach each decorator against its source-backed owner
        for (ast_node_id, ast_decorators) in ast.tree.get_all_decorators() {
            let Some(dir_node_id) = tree.get_node_id_by_source_id(*ast_node_id) else {
                continue;
            };
            for ast_decorator_id in ast_decorators {
                let Some(dir_decorator_id) = self.bind_annotation(
                    module,
                    ast,
                    scope,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    *ast_decorator_id,
                    Some(dir_node_id),
                    tree,
                    symbols,
                    types,
                ) else {
                    continue;
                };
                tree.append_decorator(dir_node_id, dir_decorator_id);
            }
        }
        self.attach_documentation(module, ast, tree, context);
    }

    /// Bind a decorator position into a DIR decorator position.
    pub(super) fn bind_annotation_position(
        &self,
        annotation_position: ast::DecoratorPosition,
    ) -> DecoratorPosition {
        match annotation_position {
            ast::DecoratorPosition::BlockInfix => DecoratorPosition::BlockInfix,
            ast::DecoratorPosition::BlockPrefix => DecoratorPosition::BlockPrefix,
            ast::DecoratorPosition::BlockPostfix => DecoratorPosition::BlockPostfix,
            ast::DecoratorPosition::LinePrefix => DecoratorPosition::LinePrefix,
            ast::DecoratorPosition::LinePostfix | ast::DecoratorPosition::LinePostfixBoundary => {
                DecoratorPosition::LinePostfix
            }
        }
    }

    /// Bind one AST decorator to one DIR decorator.
    pub(super) fn bind_annotation(
        &self,
        module: &Module,
        ast: &Ast,
        scope: (LocalScopeId, LocalScopeMark),
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        ast_annotation_id: ast::LocalNodeId<ast::Decorator>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalNodeId<Decorator>> {
        let ast_annotation = ast.tree.get(ast_annotation_id);
        let annotation_id =
            tree.reserve_from_source(NodeType::Decorator, ast_annotation_id.id, scope, parent_id);
        let position = self.bind_annotation_position(ast_annotation.position);
        let expression = self.bind_expression(
            module,
            ast,
            namespace_scope,
            global_scope,
            declared_modules,
            scope,
            ast_annotation.expression,
            Some(annotation_id),
            tree,
            symbols,
            types,
            SymbolSpace::Value,
        );
        let annotation = Decorator {
            position,
            expression,
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

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_bind_documentation_on_decorated_function_declaration() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
/// docs
@memo
function f() {}
"#,
        );

        test.bind_module(module_id);
        test.compile();
        test.check_clean();

        let dir = test.dir_declared(module_id);
        let tree = &dir.tree;
        let symbols = &dir.symbols;
        let declaration_symbol = test
            .declaration_symbol_by_name("test.d.ts", "f")
            .expect("expected f symbol");
        let declaration = symbols
            .get_symbol(declaration_symbol.into_local())
            .declaration
            .expect("expected f declaration");
        let documentation = tree
            .get_documentation(declaration.local_id.id)
            .expect("expected bound documentation");
        let documentation_text = test.program.strings.get(documentation.text);

        assert_eq!(documentation_text, "docs");
    }
}
