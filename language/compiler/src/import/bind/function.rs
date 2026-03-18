use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    Asynchrony, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature, Generics, LocalNodeIdAny, LocalScopeId, LocalScopeMark, ModuleBinding,
    NodeTree, StaticKey, SymbolSpace, SymbolSpaceOrder, SymbolTable, TypeTable,
};
use destack_workspace::{Ast, Module};

#[allow(clippy::too_many_arguments)]
impl Compiler {
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
        let arguments_name = self.program.strings.intern("arguments");
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

    /// Bind function abstraction into a DIR function abstraction.
    #[inline]
    pub(super) fn bind_function_abstraction(
        &self,
        abstraction: ast::FunctionAbstraction,
    ) -> FunctionAbstraction {
        match abstraction {
            ast::FunctionAbstraction::Abstract => FunctionAbstraction::Abstract,
            ast::FunctionAbstraction::AbstractOverride => FunctionAbstraction::AbstractOverride,
            ast::FunctionAbstraction::ConcreteOverride => FunctionAbstraction::ConcreteOverride,
            ast::FunctionAbstraction::Concrete => FunctionAbstraction::Concrete,
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
        let abstraction = self.bind_function_abstraction(signature.abstraction);
        let asynchrony = self.bind_asynchrony(signature.asynchrony);
        let cardinality = self.bind_function_cardinality(signature.cardinality);
        let mode = signature.mode.map(|mode| self.bind_function_mode(mode));
        let kind = self.bind_function_kind(signature.kind);

        // bind static parameters and defer where clauses until parameters are in scope
        let mut static_parameters = None;
        let mut where_clauses = None;
        let mut ast_where_clauses = None;
        if let Some(generics) = signature.generics.as_ref() {
            static_parameters = generics
                .static_parameters
                .as_ref()
                .map(|static_parameters| {
                    static_parameters
                        .iter()
                        .map(|static_parameter| {
                            self.bind_parameter(
                                module,
                                ast,
                                namespace_scope,
                                global_augmentation_scope,
                                module_bindings,
                                scope,
                                SymbolSpace::Type,
                                *static_parameter,
                                parent_id,
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
            ast_where_clauses = generics.where_clauses.as_ref();
        }

        // refresh scope mark so static parameters are visible to later bindings
        let scope = (scope.0, symbols.get_scope_mark(scope.0));

        // resolve the explicit this parameter when present
        let (explicit_this_parameter, dynamic_parameters) = {
            let mut this_parameter = signature.this_parameter;
            let mut dynamic_parameters = signature.dynamic_parameters.as_slice();

            // treat a leading this parameter as the explicit this parameter
            if this_parameter.is_none()
                && let Some(first_id) = dynamic_parameters.first().copied()
            {
                let this_name = self.program.strings.intern("this");
                let ast_parameter = ast.tree.get(first_id);
                let is_explicit_this = match ast_parameter {
                    ast::Parameter::Named { name, .. } => {
                        let name = self.program.strings.intern_from(&ast.strings, *name);
                        name == this_name
                    }
                    _ => false,
                };
                if is_explicit_this {
                    this_parameter = Some(first_id);
                    dynamic_parameters = &dynamic_parameters[1..];
                }
            }

            (this_parameter, dynamic_parameters)
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
        let dynamic_parameters = dynamic_parameters
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
            let arguments_name = self.program.strings.intern("arguments");
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
        let return_type = signature.return_type.map(|return_type| {
            self.bind_expression(
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
        if let Some(ast_where_clauses) = ast_where_clauses {
            where_clauses = Some(
                ast_where_clauses
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
                    .collect(),
            );
        }

        // assemble the generics payload once the where clauses are bound
        let generics = if static_parameters.is_some() || where_clauses.is_some() {
            Some(Generics {
                static_parameters,
                where_clauses,
            })
        } else {
            None
        };
        FunctionSignature {
            abstraction,
            asynchrony,
            cardinality,
            mode,
            kind,
            generics,
            this_parameter,
            dynamic_parameters,
            return_type,
        }
    }
}
