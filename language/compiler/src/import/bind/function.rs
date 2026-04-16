use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    Asynchrony, FunctionCardinality, FunctionKind, FunctionMode, FunctionSignature,
    GenericParameter, LocalNodeIdAny, LocalScopeId, LocalScopeMark, ModuleBinding, NodeTree,
    NodeType, StaticKey, SymbolBinding, SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable,
    SymbolType, Type, TypeExpression, TypeTable, VarianceModifier,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a generic parameter into a DIR generic parameter.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bind_generic_parameter(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_parameter_id: ast::LocalNodeId<ast::GenericParameter>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> destack_dir::LocalNodeId<GenericParameter> {
        let ast_parameter = ast.tree.get(ast_parameter_id);
        let parameter_id = tree.reserve_from_source(
            NodeType::GenericParameter,
            ast_parameter_id.id,
            scope,
            parent_id,
        );

        match ast_parameter {
            ast::GenericParameter::Type {
                name,
                variance,
                constraint,
                default,
            } => {
                let name = self.repository.strings.intern_from(&ast.strings, *name);
                let variance = variance.map(|variance| match variance {
                    ast::VarianceModifier::In => VarianceModifier::In,
                    ast::VarianceModifier::Out => VarianceModifier::Out,
                    ast::VarianceModifier::InOut => VarianceModifier::InOut,
                });

                let constraint = constraint.map(|constraint| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        constraint,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let default = default.map(|default| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });

                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolKind::Local,
                    SymbolType::TypeAlias,
                    SymbolSpace::Type,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(name)),
                    scope,
                    None,
                );

                let parameter = GenericParameter::Type {
                    name,
                    variance,
                    constraint,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);
                symbols
                    .get_symbol_mut(symbol_id)
                    .declare_primary(parameter_id);
                parameter_id
            }
            ast::GenericParameter::Value {
                name,
                declared_type,
                default,
                is_comptime,
            } => {
                let name = self.repository.strings.intern_from(&ast.strings, *name);
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
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });

                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolKind::Local,
                    SymbolType::Void,
                    SymbolSpace::Value,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(name)),
                    scope,
                    None,
                );

                let parameter = GenericParameter::Value {
                    name,
                    declared_type,
                    default,
                    is_comptime: *is_comptime,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);
                symbols
                    .get_symbol_mut(symbol_id)
                    .declare_primary(parameter_id);

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
            ast::GenericParameter::Error => {
                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolKind::Local,
                    SymbolType::Void,
                    SymbolSpace::Value,
                    SymbolBinding::Runtime,
                    None,
                    scope,
                    None,
                );
                let parameter = GenericParameter::Error { symbol: symbol_id };
                let parameter_id = tree.insert(parameter_id, parameter);
                symbols
                    .get_symbol_mut(symbol_id)
                    .declare_primary(parameter_id);
                parameter_id
            }
        }
    }

    /// Return true when a function receives a runtime `arguments` binding.
    fn function_has_runtime_arguments(&self, module: &Module, kind: FunctionKind) -> bool {
        if kind != FunctionKind::Function {
            return false;
        }

        module.language_type.is_javascript() || module.language_type.is_typescript()
    }

    /// Return true when the function scope already contains an `arguments` binding.
    fn function_scope_has_arguments_binding(
        &self,
        scope_id: LocalScopeId,
        symbols: &SymbolTable,
    ) -> bool {
        let arguments_name = self.repository.strings.intern("arguments");
        let arguments_key = StaticKey::Name(arguments_name);
        let scope = symbols.get_scope_by_id(scope_id);
        scope
            .find_up_to(arguments_key, LocalScopeMark::end())
            .is_some()
    }

    /// Bind function kind into a DIR function kind.
    #[inline]
    pub(super) fn bind_function_kind(&self, kind: ast::FunctionKind) -> FunctionKind {
        match kind {
            ast::FunctionKind::Function => FunctionKind::Function,
            ast::FunctionKind::Lambda => FunctionKind::Lambda,
        }
    }

    /// Bind asynchrony into a DIR asynchrony.
    #[inline]
    pub(super) fn bind_asynchrony(&self, asynchrony: ast::Asynchrony) -> Asynchrony {
        match asynchrony {
            ast::Asynchrony::Sync => Asynchrony::Sync,
            ast::Asynchrony::Async => Asynchrony::Async,
        }
    }

    /// Bind function cardinality into a DIR function cardinality.
    #[inline]
    pub(super) fn bind_function_cardinality(
        &self,
        cardinality: ast::FunctionCardinality,
    ) -> FunctionCardinality {
        match cardinality {
            ast::FunctionCardinality::Scalar => FunctionCardinality::Scalar,
            ast::FunctionCardinality::Generator => FunctionCardinality::Generator,
        }
    }

    /// Bind function mode into a DIR function mode.
    #[inline]
    pub(super) fn bind_function_mode(&self, mode: ast::FunctionMode) -> FunctionMode {
        match mode {
            ast::FunctionMode::Getter => FunctionMode::Getter,
            ast::FunctionMode::Setter => FunctionMode::Setter,
            ast::FunctionMode::Constructor => FunctionMode::Constructor,
            ast::FunctionMode::New => FunctionMode::New,
            ast::FunctionMode::Call => FunctionMode::Call,
        }
    }

    /// Bind function signature into a DIR function signature.
    pub(super) fn bind_function_signature(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        signature: &ast::FunctionSignature,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> FunctionSignature {
        let is_abstract = signature.is_abstract;
        let is_override = signature.is_override;
        let asynchrony = self.bind_asynchrony(signature.asynchrony);
        let cardinality = self.bind_function_cardinality(signature.cardinality);
        let mode = signature.mode.map(|mode| self.bind_function_mode(mode));
        let kind = self.bind_function_kind(signature.kind);

        // bind generic parameters first so they are in scope for later clauses
        let generic_parameters = signature
            .generic_parameters
            .iter()
            .map(|parameter| {
                self.bind_generic_parameter(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *parameter,
                    parent_id,
                    tree,
                    symbols,
                    types,
                )
            })
            .collect();

        // refresh scope mark so generic parameters are visible to later bindings
        let scope = (scope.0, symbols.get_scope_mark(scope.0));

        // resolve the explicit this parameter when present
        let (explicit_this_parameter, parameters) = {
            let mut this_parameter = signature.this_parameter;
            let mut parameters = signature.parameters.as_slice();

            // treat a leading this parameter as the explicit this parameter
            if this_parameter.is_none()
                && let Some(first_id) = parameters.first().copied()
            {
                let this_name = self.repository.strings.intern("this");
                let ast_parameter = ast.tree.get(first_id);
                let is_explicit_this = match ast_parameter {
                    ast::Parameter::Named { name, .. } => {
                        let name = self.repository.strings.intern_from(&ast.strings, *name);
                        name == this_name
                    }
                    _ => false,
                };
                if is_explicit_this {
                    this_parameter = Some(first_id);
                    parameters = &parameters[1..];
                }
            }

            (this_parameter, parameters)
        };

        // bind this and dynamic parameters
        let this_parameter = explicit_this_parameter.map(|this_parameter| {
            self.bind_parameter(
                module,
                ast,
                namespace_scope,
                global_augmentation_scope,
                module_bindings,
                scope,
                SymbolSpace::Value,
                this_parameter,
                parent_id,
                tree,
                symbols,
                types,
            )
        });
        let parameters = parameters
            .iter()
            .map(|parameter| {
                self.bind_parameter(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    SymbolSpace::Value,
                    *parameter,
                    parent_id,
                    tree,
                    symbols,
                    types,
                )
            })
            .collect();

        // bind JS/TS runtime arguments for non-arrow functions
        if self.function_has_runtime_arguments(module, kind)
            && !self.function_scope_has_arguments_binding(scope.0, symbols)
        {
            let arguments_name = self.repository.strings.intern("arguments");
            let arguments_key = StaticKey::Name(arguments_name);
            self.bind_named_local(
                module,
                ast,
                SymbolSpace::Value,
                arguments_key,
                scope,
                symbols,
            );
        }

        // refresh scope mark so parameters are visible to return types and where clauses
        let scope = (scope.0, symbols.get_scope_mark(scope.0));

        // bind the return type
        let return_type: Option<destack_dir::LocalNodeId<TypeExpression>> =
            signature.return_type.map(|return_type| {
                self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    return_type,
                    parent_id,
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                )
            });

        // bind where clauses with parameter scope
        let where_clauses = signature
            .where_clauses
            .iter()
            .map(|where_clause| {
                self.bind_where_clause(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *where_clause,
                    parent_id,
                    tree,
                    symbols,
                    types,
                )
            })
            .collect();

        FunctionSignature {
            is_abstract,
            is_override,
            asynchrony,
            cardinality,
            mode,
            kind,
            generic_parameters,
            where_clauses,
            this_parameter,
            parameters,
            return_type,
        }
    }
}
