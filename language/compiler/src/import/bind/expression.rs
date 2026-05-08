use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    AssignPattern, AssignPatternField, BindingKeyword, BindingScope, CastOrigin, Declarator,
    DeclaredModule, ExportKind, Expression, ForEachBinding, ForEachOperator, IfCondition, IfForm,
    ImportTarget, LetKind, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, LoopKind,
    MatchForm, MatchOrigin, Mutability, NodeType, Path, ScopeKind, StaticKey, SymbolBinding,
    SymbolForm, SymbolRole, SymbolSpace, SymbolTable, Tree, Type, TypeTable, UnevaluatedType,
    YieldCardinality,
};
use destack_workspace::Module;
use smallvec::smallvec;

use super::dependency::DependencySite;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind if form into a DIR if form.
    #[inline]
    pub(super) fn bind_if_form(&self, form: ast::IfForm) -> IfForm {
        match form {
            ast::IfForm::If => IfForm::If,
            ast::IfForm::Ternary => IfForm::Ternary,
        }
    }

    /// Map let kind into the binding scope used by the declaration.
    fn binding_scope_for_let_kind(&self, kind: ast::LetKind) -> BindingScope {
        match kind {
            ast::LetKind::Var => BindingScope::Function,
            ast::LetKind::Let | ast::LetKind::Const => BindingScope::Block,
        }
    }

    /// Bind an AST let kind into a DIR let kind.
    fn bind_let_kind(&self, kind: ast::LetKind) -> LetKind {
        match kind {
            ast::LetKind::Var => LetKind::Var,
            ast::LetKind::Let => LetKind::Let,
            ast::LetKind::Const => LetKind::Const,
        }
    }

    /// Bind one AST assignment pattern into a DIR assignment pattern.
    #[allow(clippy::too_many_arguments)]
    fn bind_assign_pattern(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_assign_pattern_id: ast::LocalNodeId<ast::AssignPattern>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<AssignPattern> {
        let ast_assign_pattern = ast.tree.get(ast_assign_pattern_id);
        let assign_pattern_id = tree.reserve_from_source(
            NodeType::AssignPattern,
            ast_assign_pattern_id.id,
            scope,
            parent_id,
        );

        let assign_pattern = match ast_assign_pattern {
            ast::AssignPattern::Expression { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(assign_pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );

                AssignPattern::Expression { value }
            }
            ast::AssignPattern::Assign { pattern, value } => {
                let pattern = self.bind_assign_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *pattern,
                    Some(assign_pattern_id),
                    tree,
                    symbols,
                    types,
                );
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(assign_pattern_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );

                AssignPattern::Assign { pattern, value }
            }
            ast::AssignPattern::Sequence { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| {
                        self.bind_assign_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *field_id,
                            Some(assign_pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                AssignPattern::Sequence { fields }
            }
            ast::AssignPattern::Object { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| {
                        self.bind_assign_pattern_field(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *field_id,
                            Some(assign_pattern_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                AssignPattern::Object { fields }
            }
        };

        tree.insert(assign_pattern_id, assign_pattern)
    }

    /// Bind one AST assignment pattern field into a DIR assignment pattern field.
    #[allow(clippy::too_many_arguments)]
    fn bind_assign_pattern_field(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_assign_pattern_field_id: ast::LocalNodeId<ast::AssignPatternField>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<AssignPatternField> {
        let ast_assign_pattern_field = ast.tree.get(ast_assign_pattern_field_id);
        let assign_pattern_field_id = tree.reserve_from_source(
            NodeType::AssignPatternField,
            ast_assign_pattern_field_id.id,
            scope,
            parent_id,
        );

        let assign_pattern_field = match ast_assign_pattern_field {
            ast::AssignPatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
                let name = self.bind_name(ast, *name);
                let pattern = pattern.map(|pattern_id| {
                    self.bind_assign_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        pattern_id,
                        Some(assign_pattern_field_id),
                        tree,
                        symbols,
                        types,
                    )
                });

                AssignPatternField::Named {
                    name,
                    is_shorthand: *is_shorthand,
                    pattern,
                }
            }
            ast::AssignPatternField::Computed { key, pattern } => {
                let key = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *key,
                    Some(assign_pattern_field_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                let pattern = self.bind_assign_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *pattern,
                    Some(assign_pattern_field_id),
                    tree,
                    symbols,
                    types,
                );

                AssignPatternField::Computed { key, pattern }
            }
            ast::AssignPatternField::Positional { pattern } => {
                let pattern = self.bind_assign_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *pattern,
                    Some(assign_pattern_field_id),
                    tree,
                    symbols,
                    types,
                );

                AssignPatternField::Positional { pattern }
            }
            ast::AssignPatternField::Spread { pattern } => {
                let pattern = pattern.map(|pattern_id| {
                    self.bind_assign_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        pattern_id,
                        Some(assign_pattern_field_id),
                        tree,
                        symbols,
                        types,
                    )
                });

                AssignPatternField::Spread { pattern }
            }
            ast::AssignPatternField::Elision => AssignPatternField::Elision,
        };

        tree.insert(assign_pattern_field_id, assign_pattern_field)
    }

    /// Bind one simple assignment target expression from an AST assignment pattern.
    #[allow(clippy::too_many_arguments)]
    fn bind_assign_target_expression(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_assign_pattern_id: ast::LocalNodeId<ast::AssignPattern>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Expression> {
        let ast_assign_pattern = ast.tree.get(ast_assign_pattern_id);

        match ast_assign_pattern {
            ast::AssignPattern::Expression { value } => self.bind_expression(
                module,
                ast,
                namespace_scope,
                global_scope,
                declared_modules,
                scope,
                *value,
                parent_id,
                tree,
                symbols,
                types,
                SymbolSpace::Value,
            ),
            _ => unreachable!("compound assignment target must be one expression target"),
        }
    }

    /// Bind an AST export kind into a DIR export kind.
    pub(super) fn bind_export_kind(&self, export: ast::ExportKind) -> ExportKind {
        match export {
            ast::ExportKind::Named => ExportKind::Named,
            ast::ExportKind::Default => ExportKind::Default,
        }
    }

    /// Bind a for each binding keyword into a DIR binding keyword.
    fn bind_binding_keyword(&self, keyword: ast::BindingKeyword) -> BindingKeyword {
        match keyword {
            ast::BindingKeyword::Var => BindingKeyword::Var,
            ast::BindingKeyword::Let => BindingKeyword::Let,
            ast::BindingKeyword::Const => BindingKeyword::Const,
        }
    }

    /// Map a for each binding keyword into the binding scope used by the declaration.
    fn binding_scope_for_binding_keyword(&self, keyword: BindingKeyword) -> BindingScope {
        match keyword {
            BindingKeyword::Var => BindingScope::Function,
            BindingKeyword::Let | BindingKeyword::Const => BindingScope::Block,
        }
    }

    /// Return true when a node type hosts declaration expressions in operand position.
    fn node_type_is_declaration_expression_container(&self, node_type: ast::NodeType) -> bool {
        matches!(
            node_type,
            ast::NodeType::Declarator
                | ast::NodeType::Argument
                | ast::NodeType::Property
                | ast::NodeType::Parameter
                | ast::NodeType::Pattern
                | ast::NodeType::PatternField
                | ast::NodeType::Member
                | ast::NodeType::WhereClause
        )
    }

    /// Return true when a declaration expression occurs in statement position.
    fn declaration_expression_is_statement_position(
        &self,
        ast: &Ast,
        ast_expression_id: ast::LocalNodeId<ast::Expression>,
    ) -> bool {
        // root expressions default to statement declarations
        let Some(parent_id) = ast.parents.get(ast_expression_id) else {
            return true;
        };

        let parent_type = ast.tree.get_node_type(parent_id);

        // expression parents are usually value-position containers
        if parent_type == ast::NodeType::Expression {
            let parent_expression = ast
                .tree
                .get(ast::LocalNodeId::<ast::Expression>::new(parent_id));

            // labelled bodies stay statement declarations
            if matches!(parent_expression, ast::Expression::Labelled { .. }) {
                return true;
            }

            // all other expression parents are expression declarations
            return false;
        }

        // operand containers always produce expression declarations
        if self.node_type_is_declaration_expression_container(parent_type) {
            return false;
        }

        // declarations under non-operand parents stay statement scoped
        true
    }

    destack_core::ensure_sufficient_stack! {
        /// Bind an expression to a DIR expression with a symbol space order.
        pub(super) fn bind_expression(
            &self,
            module: &Module,
            ast: &Ast,
            namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
            scope: (LocalScopeId, LocalScopeMark),
            ast_expression_id: ast::LocalNodeId<ast::Expression>,
            parent_id: Option<LocalNodeIdAny>,
            tree: &mut Tree,
            symbols: &mut SymbolTable,
            types: &mut TypeTable,
            space: SymbolSpace,
        ) -> LocalNodeId<Expression> {
            let ast_expression = ast.tree.get(ast_expression_id);

            let expression_id = tree.reserve_from_source(
                NodeType::Expression,
                ast_expression_id.id,
                scope,
                parent_id,
            );

            let expression = match ast_expression {
            ast::Expression::Block(block_id) => {
                let block_id = self.bind_block(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *block_id,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Block(block_id)
            }
            ast::Expression::Declaration(declaration_id) => {
                // statement declarations bind to scope end
                let is_statement_declaration =
                    self.declaration_expression_is_statement_position(ast, ast_expression_id);
                let declaration_scope = if is_statement_declaration {
                    (scope.0, LocalScopeMark::end())
                } else {
                    scope
                };
                let declaration_id = self.bind_declaration(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    declaration_scope,
                    *declaration_id,
                    is_statement_declaration,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Declaration(declaration_id)
            }
            ast::Expression::Labelled { label, body } => {
                let label = *label;
                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolRole::Local,
                    SymbolForm::Value,
                    SymbolSpace::Label,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(label)),
                    scope,
                    None,
                );
                let body_id = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *body,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::Labelled {
                    label,
                    body: body_id,
                    symbol: symbol_id,
                }
            }

            ast::Expression::Import {
                source,
                space,
                target,
                items,
                attributes,
                arguments,
            } => {
                let (target, dependency_target) = match target {
                    ast::ImportTarget::String(target) => {
                        let target = *target;
                        (ImportTarget::String(target), Some(target))
                    }
                    ast::ImportTarget::Expression { target } => {
                        let target = self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *target,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        );
                        (
                            ImportTarget::Expression { target },
                            None,
                        )
                    }
                };
                let source = self.bind_import_source(*source);

                // items
                let items = items.as_ref().map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            self.bind_dependency_item(
                                module,
                                ast,
                                namespace_scope,
                                global_scope,
                                declared_modules,
                                scope,
                                DependencySite::Import,
                                *space,
                                dependency_target,
                                *item,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });

                // attributes
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| self.bind_import_attribute_clause(module, ast, attributes));

                // arguments
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(
                                module,
                                ast,
                                namespace_scope,
                                global_scope,
                                declared_modules,
                                scope,
                                *argument,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                                SymbolSpace::Value,
                            )
                        })
                        .collect()
                });
                let space = self.bind_dependency_space(*space);
                Expression::Import {
                    source,
                    space,
                    target,
                    items,
                    attributes,
                    arguments,
                }
            }
            ast::Expression::Export {
                space,
                target,
                items,
                attributes,
            } => {
                let target = *target;
                // re-export from import
                if let Some(target) = target {
                    // items
                    let items: Vec<_> = items
                        .iter()
                        .map(|item| {
                            self.bind_dependency_item(
                                module,
                                ast,
                                namespace_scope,
                                global_scope,
                                declared_modules,
                                scope,
                                DependencySite::ReExport,
                                *space,
                                Some(target),
                                *item,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect();

                    // attributes
                    let attributes = attributes
                        .as_ref()
                        .map(|attributes| self.bind_import_attribute_clause(module, ast, attributes));

                    let space = self.bind_dependency_space(*space);
                    Expression::ReExport {
                        target,
                        space,
                        items,
                        attributes,
                    }
                }
                // export from module
                else {
                    // export = value
                    let items = {
                        // export items
                        items
                            .iter()
                            .map(|item| {
                                self.bind_dependency_item(
                                    module,
                                    ast,
                                    namespace_scope,
                                    global_scope,
                                    declared_modules,
                                    scope,
                                    DependencySite::Export,
                                    *space,
                                    None,
                                    *item,
                                    Some(expression_id),
                                    tree,
                                    symbols,
                                    types,
                                )
                            })
                            .collect()
                    };
                    let space = self.bind_dependency_space(*space);
                    let attributes = attributes
                        .as_ref()
                        .map(|attributes| self.bind_import_attribute_clause(module, ast, attributes));

                    Expression::Export {
                        space,
                        items,
                        attributes,
                    }
                }
            }
            ast::Expression::ExportNamespace { name } => {
                let name = *name;
                Expression::ExportNamespace { name }
            }
            ast::Expression::Let {
                kind,
                export,
                                    is_ambient,
                mutability,
                declarators: ast_declarators,
            } => {
                let binding_scope = self.binding_scope_for_let_kind(*kind);
                let export = export.map(|export| self.bind_export_kind(export));
                let is_ambient = *is_ambient;
                let binding = if is_ambient {
                    SymbolBinding::Ambient
                } else {
                    SymbolBinding::Runtime
                };
                let mutability = self.bind_mutability(*mutability);
                let mut declarator_scope = scope;
                let declarators: Vec<LocalNodeId<Declarator>> = ast_declarators
                    .iter()
                    .map(|ast_decl_id| {
                        let declarator = self.bind_declarator(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            declarator_scope,
                            export,
                            binding,
                            Some(mutability),
                            Some(binding_scope),
                            *ast_decl_id,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        );
                        // refresh scope mark so later declarators can reference earlier bindings
                        declarator_scope = (scope.0, symbols.get_scope_mark(scope.0));
                        declarator
                    })
                    .collect();

                return tree.insert(
                    expression_id,
                    Expression::Let {
                        export,
                        is_ambient,
                        mutability,
                        declarators,
                    },
                );
            }
            ast::Expression::LetElse {
                kind,
                mutability,
                declarator: ast_declarator,
                else_branch,
            } => {
                let binding_scope = self.binding_scope_for_let_kind(*kind);
                let mutability = self.bind_mutability(*mutability);
                let declarator = self.bind_declarator(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    None,
                    SymbolBinding::Runtime,
                    Some(mutability),
                    Some(binding_scope),
                    *ast_declarator,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let else_branch = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *else_branch,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );

                return tree.insert(
                    expression_id,
                    Expression::LetElse {
                        kind: self.bind_let_kind(*kind),
                        mutability,
                        declarator,
                        else_branch,
                    },
                );
            }
            ast::Expression::Using {
                asynchrony,
                export,
                                    is_ambient,
                declarators: ast_declarators,
            } => {
                let export = export.map(|export| self.bind_export_kind(export));
                let is_ambient = *is_ambient;
                let binding = if is_ambient {
                    SymbolBinding::Ambient
                } else {
                    SymbolBinding::Runtime
                };
                let asynchrony = self.bind_asynchrony(*asynchrony);
                let mutability = Mutability::Immutable;
                let binding_scope = BindingScope::Block;
                let mut declarator_scope = scope;
                let declarators: Vec<LocalNodeId<Declarator>> = ast_declarators
                    .iter()
                    .map(|ast_decl_id| {
                        let declarator = self.bind_declarator(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            declarator_scope,
                            export,
                            binding,
                            Some(mutability),
                            Some(binding_scope),
                            *ast_decl_id,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        );

                        // refresh scope mark so later declarators can reference earlier bindings
                        declarator_scope = (scope.0, symbols.get_scope_mark(scope.0));

                        declarator
                    })
                    .collect();

                return tree.insert(
                    expression_id,
                    Expression::Using {
                        asynchrony,
                        export,
                        is_ambient,
                        declarators,
                    },
                );
            }
            ast::Expression::Unary { operator, right } => {
                let right = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let operator = self.bind_unary_operator(*operator);
                Expression::Unary { operator, right }
            }

            ast::Expression::As {
                expression,
                target_type,
            } => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );

                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );

                Expression::As {
                    operator: None,
                    source: CastOrigin::Explicit,
                    expression,
                    target_type,
                }
            }

            ast::Expression::Satisfies {
                expression,
                target_type,
            } => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );

                Expression::Satisfies {
                    expression,
                    target_type,
                }
            }

            ast::Expression::MoveOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let variance = variance.map(|variance| self.bind_variance_bound(variance));
                let right = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::MoveOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::BorrowOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let variance = variance.map(|variance| self.bind_variance_bound(variance));
                let right = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::BorrowOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::Is { value, target_type } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );

                Expression::Is { value, target_type }
            }
            ast::Expression::InstanceOf { value, target } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let target = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );

                Expression::InstanceOf { value, target }
            }
            ast::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let right = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let operator = self.bind_binary_operator(*operator);
                Expression::Binary {
                    left,
                    operator,
                    right,
                }
            }
            ast::Expression::Assign {
                left,
                operator,
                right,
            } => {
                let right = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let operator = self.bind_assign_operator(*operator);

                if let Some(operator) = operator {
                    let left = self.bind_assign_target_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        *left,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    );

                    Expression::AssignBinary {
                        left,
                        operator,
                        right,
                    }
                } else {
                    let left = self.bind_assign_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        *left,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    );

                    Expression::Assign { left, right }
                }
            }

            ast::Expression::Member { left, name } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let name = name.map(|name| name);

                Expression::Member { left, name }
            }
            ast::Expression::PrivateMember { left, name } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let name = name.map(|name| name);

                Expression::PrivateMember { left, name }
            }
            ast::Expression::Call {
                position: _,
                left,
                generic_arguments,
                arguments,
            } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let generic_argument_space_order = if module.is_destack() {
                    SymbolSpace::Value
                } else {
                    SymbolSpace::Type
                };
                let generic_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            generic_argument_space_order,
                        )
                    })
                    .collect();
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        )
                    })
                    .collect();
                Expression::Call {
                    left,
                    generic_arguments,
                    arguments,
                }
            }
            ast::Expression::New {
                left,
                generic_arguments,
                arguments,
            } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let generic_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
                        )
                    })
                    .collect();
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        )
                    })
                    .collect();
                Expression::New {
                    left,
                    generic_arguments,
                    arguments,
                }
            }
            ast::Expression::Delete { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::Delete { value }
            }
            ast::Expression::Index {
                position: _,
                left,
                index,
            } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let index = index.map(|index| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        index,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                Expression::Index { left, right: index }
            }
            ast::Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let generic_argument_space_order = if module.is_destack() {
                    SymbolSpace::Value
                } else {
                    SymbolSpace::Type
                };
                let generic_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            generic_argument_space_order,
                        )
                    })
                    .collect();
                Expression::Instantiation {
                    left,
                    generic_arguments,
                }
            }
            ast::Expression::Maybe { position: _, left } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::Maybe { left }
            }
            ast::Expression::Must { position: _, left } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::Must { left }
            }

            ast::Expression::Identifier { name } => {
                let path = Path {
                    segments: smallvec![*name],
                };
                Expression::Path {
                    path,
                    generic_arguments: Vec::new(),
                }
            }

            ast::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => {
                let path = self.bind_path(module, ast, path);
                let generic_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Type,
                        )
                    })
                    .collect();
                Expression::Path {
                    path,
                    generic_arguments,
                }
            }
            ast::Expression::PrivateIdentifier { name } => {
                let name = *name;
                Expression::PrivateIdentifier { name }
            }
            ast::Expression::ScalarLiteral(value) => {
                let value = self.bind_scalar_literal(module, ast, value);
                Expression::ScalarLiteral { value }
            }
            ast::Expression::TemplateExpression { value } => {
                let value = self.bind_template_literal(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::TemplateExpression { value }
            }
            ast::Expression::TaggedTemplateExpression {
                tag,
                generic_arguments,
                value,
            } => {
                let tag = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *tag,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let generic_argument_space_order = if module.is_destack() {
                    SymbolSpace::Value
                } else {
                    SymbolSpace::Type
                };
                let generic_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            generic_argument_space_order,
                        )
                    })
                    .collect();
                let value = self.bind_template_literal(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );

                Expression::TaggedTemplateExpression {
                    tag,
                    generic_arguments,
                    value,
                }
            }
            ast::Expression::ObjectExpression { ty, properties } => {
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *property,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                if let Some(ty_id) = ty {
                    let ty = self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        *ty_id,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    );
                    Expression::TaggedObjectExpression { ty, properties }
                } else {
                    Expression::ObjectExpression {
                        ty: None,
                        properties,
                    }
                }
            }
            ast::Expression::TupleExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *element,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        )
                    })
                    .collect();
                Expression::TupleExpression { elements }
            }
            ast::Expression::SequenceExpression { expressions } => {
                let expressions = expressions
                    .iter()
                    .map(|expr_id| {
                        self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *expr_id,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();
                Expression::SequenceExpression { expressions }
            }
            ast::Expression::ArrayExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *element,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        )
                    })
                    .collect();
                Expression::ArrayExpression { elements }
            }
            ast::Expression::TreeExpression {
                left,
                generic_arguments,
                arguments,
                elements,
            } => {
                let left = left.map(|left| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        left,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let generic_argument_space_order = if module.is_destack() {
                    SymbolSpace::Value
                } else {
                    SymbolSpace::Type
                };
                let generic_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            generic_argument_space_order,
                        )
                    })
                    .collect();
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(
                                module,
                                ast,
                                namespace_scope,
                                global_scope,
                                declared_modules,
                                scope,
                                *argument,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                                SymbolSpace::Value,
                            )
                        })
                        .collect()
                });
                let elements = elements.as_ref().map(|elements| {
                    elements
                        .iter()
                        .map(|element| {
                            self.bind_argument(
                                module,
                                ast,
                                namespace_scope,
                                global_scope,
                                declared_modules,
                                scope,
                                *element,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                                SymbolSpace::Value,
                            )
                        })
                        .collect()
                });
                Expression::TreeExpression {
                    left,
                    generic_arguments,
                    arguments,
                    elements,
                }
            }
            ast::Expression::Parenthesized { expression } => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::Parenthesized { expression }
            }
            ast::Expression::Type { value } => {
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Type,
                );
                types.insert_type_from(Type::Unevaluated(UnevaluatedType { expression: value }), value);

                Expression::Type { value }
            }

            ast::Expression::If {
                form,
                condition,
                then_expression,
                else_expression,
            } => {
                let form = self.bind_if_form(*form);
                let (condition, then_expression, else_expression) = match condition {
                    ast::IfCondition::Expression { condition } => {
                        let condition = self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *condition,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            space,
                        );
                        let then_expression = self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *then_expression,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            space,
                        );
                        let else_expression = else_expression.map(|else_expression| {
                            self.bind_expression(
                                module,
                                ast,
                                namespace_scope,
                                global_scope,
                                declared_modules,
                                scope,
                                else_expression,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                                space,
                            )
                        });
                        (
                            IfCondition::Expression { condition },
                            then_expression,
                            else_expression,
                        )
                    }
                    ast::IfCondition::Let {
                        kind,
                        mutability,
                        declarator,
                    } => {
                        let mutability = self.bind_mutability(*mutability);
                        let binding_scope = self.binding_scope_for_let_kind(*kind);
                        let outer_scope = scope;
                        let if_scope_id =
                            symbols.insert_scope(ScopeKind::Block, Some(outer_scope), None);
                        let if_scope = (if_scope_id, symbols.get_scope_mark(if_scope_id));
                        let declarator = self.bind_declarator(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            if_scope,
                            None,
                            SymbolBinding::Runtime,
                            Some(mutability),
                            Some(binding_scope),
                            *declarator,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        );
                        let if_scope = (if_scope_id, symbols.get_scope_mark(if_scope_id));
                        let then_expression = self.bind_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            if_scope,
                            *then_expression,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            space,
                        );
                        let else_expression = else_expression.map(|else_expression| {
                            let else_scope_id =
                                symbols.insert_scope(ScopeKind::Block, Some(outer_scope), None);
                            let else_scope = (else_scope_id, symbols.get_scope_mark(else_scope_id));
                            self.bind_expression(
                                module,
                                ast,
                                namespace_scope,
                                global_scope,
                                declared_modules,
                                else_scope,
                                else_expression,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                                space,
                            )
                        });
                        (
                            IfCondition::Let {
                                kind: self.bind_let_kind(*kind),
                                mutability,
                                declarator,
                            },
                            then_expression,
                            else_expression,
                        )
                    }
                };
                Expression::If {
                    form,
                    condition,
                    then_expression,
                    else_expression,
                }
            }
            ast::Expression::While {
                form,
                condition,
                body,
            } => {
                let kind = match *form {
                    ast::WhileForm::While => LoopKind::PreTest,
                    ast::WhileForm::DoWhile => LoopKind::PostTest,
                };
                let condition = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *condition,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let body = self.bind_block(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Loop {
                    kind,
                    condition: Some(condition),
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::ForEach {
                asynchrony,
                operator,
                binding,
                iterator,
                body,
            } => {
                let asynchrony = self.bind_asynchrony(*asynchrony);
                let operator = match *operator {
                    ast::ForEachOperator::In => ForEachOperator::In,
                    ast::ForEachOperator::Of => ForEachOperator::Of,
                };
                let iterator = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *iterator,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let binding = match binding {
                    ast::ForEachBinding::Pattern {
                        pattern,
                        keyword,
                    } => {
                        let keyword =
                            keyword.map(|keyword| self.bind_binding_keyword(keyword));
                        let binding_scope = keyword
                            .map(|keyword| self.binding_scope_for_binding_keyword(keyword));
                        let pattern = self.bind_pattern(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            None,
                            SymbolBinding::Runtime,
                            None,
                            binding_scope,
                            *pattern,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        );
                        ForEachBinding::Pattern {
                            pattern,
                            keyword,
                        }
                    }
                    ast::ForEachBinding::Using { asynchrony, pattern } => {
                        let pattern = self.bind_pattern(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            None,
                            SymbolBinding::Runtime,
                            Some(Mutability::Immutable),
                            Some(BindingScope::Block),
                            *pattern,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        );
                        ForEachBinding::Using {
                            asynchrony: self.bind_asynchrony(*asynchrony),
                            pattern,
                        }
                    }
                };
                let body = self.bind_block(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::ForEach {
                    asynchrony,
                    operator,
                    binding,
                    iterator,
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let initialization = initialization.map(|initialization| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        initialization,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let condition = condition.map(|condition| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        condition,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let increment = increment.map(|increment| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        increment,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let body = self.bind_block(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::For {
                    initialization,
                    condition,
                    increment,
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::Loop { body } => {
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let body = self.bind_block(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Loop {
                    kind: LoopKind::NoTest,
                    condition: None,
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::Try {
                try_expression,
                catch_pattern,
                catch_ty,
                catch_expression,
                finally_expression,
            } => {
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let try_expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *try_expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let catch_pattern = catch_pattern.map(|catch_pattern| {
                    self.bind_pattern(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        None,
                        SymbolBinding::Runtime,
                        None,
                        Some(BindingScope::Parameter),
                        catch_pattern,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let catch_ty = catch_ty.map(|catch_ty| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        catch_ty,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });
                let catch_expression = catch_expression.map(|catch_expression| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        catch_expression,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let finally_expression = finally_expression.map(|finally_expression| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        finally_expression,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                Expression::Try {
                    try_expression,
                    catch_pattern,
                    catch_ty,
                    catch_expression,
                    finally_expression,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::Match { form, value, cases } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let cases = cases
                    .iter()
                    .map(|case| {
                        self.bind_match_case(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *case,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let form = match *form {
                    ast::MatchForm::Match => MatchForm::Match,
                    ast::MatchForm::Switch => MatchForm::Switch,
                };
                Expression::Match {
                    form,
                    value,
                    cases,
                    source: MatchOrigin::Match,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }

            ast::Expression::Break { label, value } => {
                let label =
                    label.map(|label| label);
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        value,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                if let Some(label) = label {
                    Expression::Break {
                        target: Some(label),
                        value,
                    }
                } else {
                    Expression::Break {
                        target: None,
                        value,
                    }
                }
            }
            ast::Expression::Continue { label } => {
                let label =
                    label.map(|label| label);
                if let Some(label) = label {
                    Expression::Continue {
                        target: Some(label),
                    }
                } else {
                    Expression::Continue { target: None }
                }
            }
            ast::Expression::Return { value } => {
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        value,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                Expression::Return { value }
            }
            ast::Expression::Await { expression } => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::Await { expression }
            }
            ast::Expression::AwaitMaybe { expression } => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::AwaitMaybe { expression }
            }
            ast::Expression::Comptime { body } => {
                let body = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *body,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::Comptime { body }
            }
            ast::Expression::Yield { cardinality, value } => {
                let cardinality = match *cardinality {
                    ast::YieldCardinality::Generator => YieldCardinality::Generator,
                    ast::YieldCardinality::Scalar => YieldCardinality::Scalar,
                };
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        value,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                Expression::Yield { cardinality, value }
            }
            ast::Expression::Throw { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space,
                );
                Expression::Throw { value }
            }

            ast::Expression::This => Expression::This,
            ast::Expression::Super => Expression::Super,
            ast::Expression::ImportMeta => Expression::ImportMeta,
            ast::Expression::NewTarget => Expression::NewTarget,
            ast::Expression::Debugger => Expression::Debugger,
            ast::Expression::Missing => Expression::Missing,
            ast::Expression::Stub => Expression::Stub,
            ast::Expression::Error => Expression::Error,
        };

        // expression
        if let Some(symbol_id) = expression.symbol() {
            let expression_id = tree.insert(expression_id, expression);
            symbols
                .get_symbol_mut(symbol_id)
                .declare(expression_id);
            expression_id
        } else {
            tree.insert(expression_id, expression)
        }
        }
    }

    /// Bind a declarator to a DIR declarator.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bind_declarator(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        binding: SymbolBinding,
        binding_mutability: Option<Mutability>,
        binding_scope: Option<BindingScope>,
        ast_declarator_id: ast::LocalNodeId<ast::Declarator>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Declarator> {
        let ast_declarator = ast.tree.get(ast_declarator_id);
        let declarator_id =
            tree.reserve_from_source(NodeType::Declarator, ast_declarator_id.id, scope, parent_id);
        let ast::Declarator { pattern, ty, value } = ast_declarator;

        let pattern = self.bind_pattern(
            module,
            ast,
            namespace_scope,
            global_scope,
            declared_modules,
            scope,
            export,
            binding,
            binding_mutability,
            binding_scope,
            *pattern,
            Some(declarator_id),
            tree,
            symbols,
            types,
        );
        let bound_ty = ty.map(|ty_id| {
            self.bind_type_expression(
                module,
                ast,
                namespace_scope,
                global_scope,
                declared_modules,
                scope,
                ty_id,
                Some(declarator_id),
                tree,
                symbols,
                types,
                SymbolSpace::Type,
            )
        });

        // JS/TS initializers can reference the bound declarator name
        let value_scope = if module.is_ecmascript() {
            (scope.0, symbols.get_scope_mark(scope.0))
        } else {
            scope
        };
        let value = value.map(|v| {
            self.bind_expression(
                module,
                ast,
                namespace_scope,
                global_scope,
                declared_modules,
                value_scope,
                v,
                Some(declarator_id),
                tree,
                symbols,
                types,
                SymbolSpace::Value,
            )
        });
        // set declared type on declarator if type annotation is present
        if let Some(ty_id) = bound_ty {
            let declared_ty = types.insert_type_from(
                Type::Unevaluated(UnevaluatedType { expression: ty_id }),
                ty_id,
            );
            types.set_declared_type(declarator_id.into_global(module.id), declared_ty);
        }
        let declarator = Declarator {
            pattern,
            ty: bound_ty,
            value,
        };
        tree.insert(declarator_id, declarator)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;
    use crate::{assert_node, assert_path};
    use destack_dir::{
        Declarator, Expression, StaticKey, SymbolSpace, SymbolTable, TypeExpression,
    };

    // Test that infer type variables are visible in the then-branch of conditional types.
    #[test]
    fn test_bind_infer_type_variable_in_conditional_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
type ThisParameterType<T> = T extends (this: infer U, ...args: never) => any ? U : unknown;
type OmitThisParameter<T> = unknown extends ThisParameterType<T> ? T : T extends (...args: infer A) => infer R ? (...args: A) => R : T;
"#,
        );
        test.bind_module(module_id);
        test.compile();
        test.check_clean();
    }

    // Test that infer in nested positions (object types, function types) works correctly.
    #[test]
    fn test_bind_infer_type_variable_nested_in_object_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
type ExtractCallback<T> = T extends { callback: (x: infer U) => void } ? U : never;
"#,
        );
        test.bind_module(module_id);
        test.compile();
        test.check_clean();
    }

    // test that infer variables from mapped key clauses are visible in conditional then branches
    #[test]
    fn test_bind_infer_type_variable_in_mapped_key_conditional_clause() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
type SearchValue<T> = T extends { [K in 'query']: infer Query } ? Query : never;
"#,
        );

        test.bind_module(module_id);
        test.compile();
        test.check_clean();
    }

    // test that nested conditional infer variables resolve in the nearest conditional scope
    #[test]
    fn test_bind_infer_type_variable_in_nearest_nested_conditional_scope() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
type Nested<T> = T extends { value: unknown }
  ? T["value"] extends { inner: infer Inner }
    ? Inner
    : never
  : never;
"#,
        );

        test.bind_module(module_id);
        test.compile();
        test.check_clean();
    }
    // test that infer outside conditional extends clauses reports an analyze error
    #[test]
    fn test_bind_infer_type_variable_outside_conditional_scope() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
type Invalid = infer U;
"#,
        );

        test.bind_module(module_id);
        test.compile();
        test.check_clean();
    }

    // check declarator paths are shaped by expression kind
    #[test]
    fn test_bind_declarator_path_kinds() {
        // setup module
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let Foo: Foo = Foo;
"#,
        );

        // bind and compile
        test.bind_module(module_id);
        test.compile();
        test.check_clean();

        // load bound tree
        let dir = test.dir_declared(module_id);
        let tree = &dir.tree;

        // select the root expression
        let root_id = test.expect_root_expression(module_id);

        // assert declarator path kinds
        assert_node!(
            tree,
            root_id,
            Expression::Let { declarators, .. } => {
                let declarator_id = declarators.first().copied().expect("expected declarator");
                assert_node!(
                    tree,
                    declarator_id,
                    Declarator {
                        ty: Some(ty_id),
                        value: Some(value_id),
                        ..
                    } => {
                        assert_node!(
                            tree,
                            *ty_id,
                            TypeExpression::Reference {
                                path,
                                generic_arguments: _,
                            } => {
                                assert_path!(test.program, path, "Foo");
                            }
                        );

                        assert_node!(
                            tree,
                            *value_id,
                            Expression::Path {
                                path,
                                generic_arguments: _,
                            } => {
                                assert_path!(test.program, path, "Foo");
                            }
                        );
                    }
                );
            }
        );
    }

    // ensure import type bindings use type space only
    #[test]
    fn test_bind_import_type_uses_type_space() {
        let test = TestProgram::memory_sequential();
        let _dep_id = test.add_module("dep.d.ts", "export type Foo = number;");
        let main_id = test.add_module("main.d.ts", r#"import type { Foo } from "./dep";"#);

        test.bind_module(main_id);
        test.compile();
        test.check_clean();

        let dir = test.dir_declared(main_id);
        let symbols = &dir.symbols;
        let name = test.program.strings.intern("Foo");
        let key = StaticKey::Name(name);
        let (type_count, value_count) = count_symbol_spaces(symbols, key);

        assert_eq!(type_count, 1);
        assert_eq!(value_count, 0);
    }

    fn count_symbol_spaces(symbols: &SymbolTable, key: StaticKey) -> (usize, usize) {
        let mut type_count = 0;
        let mut value_count = 0;

        for local_id in symbols.symbol_ids() {
            let symbol = symbols.get_symbol(local_id);
            if symbol.key != Some(key) {
                continue;
            }

            match symbol.space {
                SymbolSpace::Type => type_count += 1,
                SymbolSpace::Value => value_count += 1,
                SymbolSpace::Label => {}
            }
        }

        (type_count, value_count)
    }
}
