use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    AbstractionModifier, AccessorKind, Argument, BindingAnchor, BindingCategory, BindingKind,
    BindingModifier, BindingOperator, DeclarationKind, Expression, LocalNodeId, LocalNodeIdAny,
    LocalScopeId, LocalScopeMark, ModuleBinding, Mutability, NodeTree, NodeType, Parameter,
    ProvenanceReason, StaticKey, SymbolBinding, SymbolSpace, SymbolSpaceOrder, SymbolTable, Timing,
    Type, TypeTable, VarianceModifier, Visibility,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind visibility into a DIR visibility.
    #[inline]
    pub(super) fn bind_visibility(&self, visibility: ast::Visibility) -> destack_dir::Visibility {
        match visibility {
            ast::Visibility::Public => destack_dir::Visibility::Public,
            ast::Visibility::Protected => destack_dir::Visibility::Protected,
            ast::Visibility::Private => destack_dir::Visibility::Private,
        }
    }

    /// Bind a parameter into a DIR parameter.
    pub(super) fn bind_parameter(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        symbol_space: SymbolSpace,
        ast_parameter_id: ast::LocalNodeId<ast::Parameter>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Parameter> {
        let ast_parameter = ast.tree.get(ast_parameter_id);
        let parameter_id =
            tree.reserve_from_source(NodeType::Parameter, ast_parameter_id.id, scope, parent_id);

        // defaults on type-space parameters should also parse in type space first
        let default_space_order = match symbol_space {
            SymbolSpace::Type => SymbolSpaceOrder::TypeThenValue,
            SymbolSpace::Value | SymbolSpace::TypeValue | SymbolSpace::Label => {
                SymbolSpaceOrder::ValueThenType
            }
        };

        // pattern bindings still need a runtime binding mutability
        let binding_mutability = self.default_binding_mutability(module);

        match ast_parameter {
            ast::Parameter::Named {
                name,
                visibility,
                is_readonly,
                is_optional,
                declared_type,
                default,
            } => {
                let name = self.repository.strings.intern_from(&ast.strings, *name);
                let visibility = visibility.map(|visibility| self.bind_visibility(visibility));
                let declared_type = declared_type.map(|declared_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        declared_type,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(parameter_id),
                        tree,
                        symbols,
                        types,
                        default_space_order,
                    )
                });
                let (symbol_id, _) = self.bind_named_item(
                    module,
                    ast,
                    symbol_space,
                    destack_dir::StaticKey::Name(name),
                    scope,
                    None,
                    symbols,
                );
                let parameter = Parameter::Named {
                    name,
                    visibility,
                    is_readonly: *is_readonly,
                    is_optional: *is_optional,
                    declared_type,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);

                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));

                self.apply_binding_mutability(symbols, symbol_id, binding_mutability);
                self.apply_binding_category(symbols, symbol_id, BindingCategory::Parameter);

                if let Some(declared_type) = declared_type {
                    let declared_type_id =
                        types.insert_type_from(Type::Unevaluated(declared_type), declared_type);
                    types.set_declared_type(
                        parameter_id.into_global_any(module.id),
                        declared_type_id,
                    );
                }

                parameter_id
            }
            ast::Parameter::Pattern {
                pattern,
                is_optional,
                declared_type,
                default,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    None,
                    SymbolBinding::Runtime,
                    Some(binding_mutability),
                    Some(BindingCategory::Parameter),
                    *pattern,
                    Some(parameter_id),
                    tree,
                    symbols,
                    types,
                );
                let declared_type = declared_type.map(|declared_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        declared_type,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(parameter_id),
                        tree,
                        symbols,
                        types,
                        default_space_order,
                    )
                });
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, symbol_space, scope, None, symbols);
                let parameter = Parameter::Pattern {
                    pattern,
                    is_optional: *is_optional,
                    declared_type,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);

                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));

                self.apply_binding_mutability(symbols, symbol_id, binding_mutability);
                self.apply_binding_category(symbols, symbol_id, BindingCategory::Parameter);

                if let Some(declared_type) = declared_type {
                    let declared_type_id =
                        types.insert_type_from(Type::Unevaluated(declared_type), declared_type);
                    types.set_declared_type(
                        parameter_id.into_global_any(module.id),
                        declared_type_id,
                    );
                }

                parameter_id
            }
            ast::Parameter::VariadicNamed {
                name,
                visibility,
                is_readonly,
                declared_type,
            } => {
                let name = self.repository.strings.intern_from(&ast.strings, *name);
                let visibility = visibility.map(|visibility| self.bind_visibility(visibility));
                let declared_type = declared_type.map(|declared_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        declared_type,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let (symbol_id, _) = self.bind_named_item(
                    module,
                    ast,
                    symbol_space,
                    destack_dir::StaticKey::Name(name),
                    scope,
                    None,
                    symbols,
                );
                let parameter = Parameter::VariadicNamed {
                    name,
                    visibility,
                    is_readonly: *is_readonly,
                    declared_type,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);

                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));

                self.apply_binding_mutability(symbols, symbol_id, binding_mutability);
                self.apply_binding_category(symbols, symbol_id, BindingCategory::Parameter);

                if let Some(declared_type) = declared_type {
                    let declared_type_id =
                        types.insert_type_from(Type::Unevaluated(declared_type), declared_type);
                    types.set_declared_type(
                        parameter_id.into_global_any(module.id),
                        declared_type_id,
                    );
                }

                parameter_id
            }
            ast::Parameter::VariadicPattern {
                pattern,
                declared_type,
            } => {
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    None,
                    SymbolBinding::Runtime,
                    Some(binding_mutability),
                    Some(BindingCategory::Parameter),
                    *pattern,
                    Some(parameter_id),
                    tree,
                    symbols,
                    types,
                );
                let declared_type = declared_type.map(|declared_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        declared_type,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, symbol_space, scope, None, symbols);
                let parameter = Parameter::VariadicPattern {
                    pattern,
                    declared_type,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);

                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));

                self.apply_binding_mutability(symbols, symbol_id, binding_mutability);
                self.apply_binding_category(symbols, symbol_id, BindingCategory::Parameter);

                if let Some(declared_type) = declared_type {
                    let declared_type_id =
                        types.insert_type_from(Type::Unevaluated(declared_type), declared_type);
                    types.set_declared_type(
                        parameter_id.into_global_any(module.id),
                        declared_type_id,
                    );
                }

                parameter_id
            }
            ast::Parameter::Error => {
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, symbol_space, scope, None, symbols);
                let parameter = Parameter::Error { symbol: symbol_id };
                let parameter_id = tree.insert(parameter_id, parameter);

                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));

                self.apply_binding_category(symbols, symbol_id, BindingCategory::Parameter);

                parameter_id
            }
        }
    }

    /// Bind an argument into a DIR argument.
    pub(super) fn bind_argument(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_argument_id: ast::LocalNodeId<ast::Argument>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        space_order: SymbolSpaceOrder,
    ) -> LocalNodeId<Argument> {
        let ast_argument = ast.tree.get(ast_argument_id);
        let argument_id =
            tree.reserve_from_source(NodeType::Argument, ast_argument_id.id, scope, parent_id);

        match ast_argument {
            ast::Argument::Named { name, value } => {
                let name = self.bind_name(ast, *name);
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                tree.insert(argument_id, Argument::Named { name, value })
            }
            ast::Argument::Labeled { label, value } => {
                let label = self.repository.strings.intern_from(&ast.strings, *label);
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                tree.insert(argument_id, Argument::Labeled { label, value })
            }
            ast::Argument::Positional { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                tree.insert(argument_id, Argument::Positional { value })
            }
            ast::Argument::Spread { label, value } => {
                let label =
                    label.map(|label| self.repository.strings.intern_from(&ast.strings, label));
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                tree.insert(argument_id, Argument::Spread { label, value })
            }
            ast::Argument::Error => {
                let error_expression_id = tree.reserve_from(
                    NodeType::Expression,
                    argument_id,
                    scope,
                    Some(argument_id),
                    Some(ProvenanceReason::Bound),
                );
                let error_expression = tree.insert(error_expression_id, Expression::Error);
                tree.insert(
                    argument_id,
                    Argument::Error {
                        value: error_expression,
                    },
                )
            }
        }
    }
}
