use std::collections::HashSet;

use crate::analyze::common::CanonicalSymbolMode;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Argument, BinaryOperator, BindingKind, Declaration, DependencyItem, DynamicKey, EnumFieldValue,
    Expression, FunctionMode, FunctionSignature, GlobalSymbolId, IntrinsicType, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, Mutability, NodeTree, NodeType, NodeVisitor, NodeVisitorOptions,
    Path, PrimitiveType, Property, Resolution, ScalarLiteral, StaticArgument, StaticExpression,
    StaticKey, StaticParameterKind, StaticProperty, SymbolSpace, SymbolSpaceOrder, SymbolTable,
    Type, TypeElement, TypeField, TypeIndexSignature, TypeLiteral, TypeMappedParameter, TypeTable,
    TypeUnaryOperator, UnaryOperator, walk_expression,
};
use destack_workspace::{Module, ModuleSource, ProfileId};

/// Visitor for validating static value parameter usage in type expressions.
#[derive(Debug)]
struct StaticValueParameterValidator<'a> {
    /// The compiler shared state.
    compiler: &'a Compiler,
    /// The module being checked.
    module: &'a Module,
    /// The profile id used for evaluation.
    profile: ProfileId,
    /// The symbol table for this module.
    symbols: &'a SymbolTable,
    /// The type table for this module.
    types: &'a mut TypeTable,
    /// Whether to validate static argument bounds.
    validate_static_argument_bounds: bool,
    /// Whether to enforce implicit managed semantics.
    enforce_implicit_managed: bool,
    /// Track the first error encountered while walking.
    result: AnalyzeResult<()>,
    /// Node visitor options (unused, but required by trait).
    options: NodeVisitorOptions,
}

impl<'a> StaticValueParameterValidator<'a> {
    /// Create a new static value parameter validator.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
        types: &'a mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> Self {
        // build the validator state
        Self {
            compiler,
            module,
            profile,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
            result: Ok(()),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Check if this validator should continue.
    fn should_continue(&self) -> bool {
        // stop once an error is recorded
        self.result.is_ok()
    }

    /// Record a validation result, preserving the first error.
    fn record_result(&mut self, result: AnalyzeResult<()>) {
        // keep the first error in the validator
        if self.result.is_ok() && result.is_err() {
            self.result = result;
        }
    }

    /// Finish the walk and return the result.
    fn finish(self) -> AnalyzeResult<()> {
        // return the recorded result
        self.result
    }
}

impl NodeVisitor for StaticValueParameterValidator<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // stop on first error
        if !self.should_continue() {
            return;
        }

        // validate array-size usage for type index expressions
        if let Expression::TypeIndex { left, index } = expression {
            // resolve the left type to decide between index access and array sizes
            let left_id = match self.compiler.try_evaluate_expression_to_type(
                self.module,
                self.profile,
                *left,
                tree,
                self.symbols,
                self.types,
                self.validate_static_argument_bounds,
                self.enforce_implicit_managed,
            ) {
                Ok(left_id) => left_id,
                Err(error) => {
                    self.record_result(Err(error));
                    return;
                }
            };

            let supports_index_access = match self.compiler.type_supports_index_access(
                self.module,
                self.profile,
                left_id,
                tree,
                self.symbols,
                self.types,
            ) {
                Ok(supports) => supports,
                Err(error) => {
                    self.record_result(Err(error));
                    return;
                }
            };
            let is_primitive_literal = self.compiler.type_is_primitive_literal(left_id, self.types);
            let is_index_access = supports_index_access && !is_primitive_literal;

            if !is_index_access {
                let result = self.compiler.resolve_array_size_parameter_type(
                    self.module,
                    self.profile,
                    *index,
                    tree,
                    self.symbols,
                    self.types,
                    self.validate_static_argument_bounds,
                    self.enforce_implicit_managed,
                );
                self.record_result(result.map(|_| ()));
            }
        }

        // walk nested expression nodes
        destack_base::ensure_sufficient_stack(|| {
            walk_expression(self, tree, id, expression);
        });
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Evaluate a Type (in-place).
    /// Converts Type::Unevaluated to the actual Type value.
    pub(crate) fn evaluate_type(
        &self,
        module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // pull the unevaluated expression id when needed
        let expression_id = {
            let ty = types.get_type(ty_id);
            let Type::Unevaluated(expression_id) = *ty else {
                return Ok(());
            };
            expression_id
        };

        // evaluate and update in place
        let evaluated_ty = self.try_evaluate_expression_to_type_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            true,
            true,
            true,
            true,
        )?;
        let ty = types.get_type_mut(ty_id);
        *ty = evaluated_ty;

        // invalidate normalization cache after in-place updates
        types.invalidate_normalization_cache();

        Ok(())
    }

    /// Try to evaluate an Expression as a Type.
    /// Returns the evaluated Type value, or a Type::Unevaluated if it fails.
    /// Set validate_static_argument_bounds to false to defer bound checks.
    /// Set enforce_implicit_managed to false to skip noImplicitManaged enforcement.
    /// Set resolve_static_arguments to false to preserve alias argument structure.
    /// Set use_declared_cache to false to skip declared cache lookups.
    pub(crate) fn try_evaluate_expression_to_type_value(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
        resolve_static_arguments: bool,
        use_declared_cache: bool,
    ) -> AnalyzeResult<Type> {
        // reuse cached declared types when requested
        let global_node_id = expression_id.into_global_any(module.id);
        if use_declared_cache
            && let Some(existing) = types.get_declared_type_id(global_node_id)
            && !matches!(types.get_type(existing), Type::Unevaluated(_))
        {
            // allow cached unknown references to resolve to their referenced symbols
            let is_unknown = matches!(
                types.get_type(existing),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown
                }
            );
            let is_reference = matches!(
                tree.get(expression_id),
                Expression::LocalReference { .. }
                    | Expression::ModuleReference { .. }
                    | Expression::GlobalReference { .. }
            );

            if !(is_unknown && is_reference) {
                return Ok(types.get_type(existing).clone());
            }
        }

        // avoid recursive evaluation loops
        if types.is_expression_type_in_progress(global_node_id) {
            return Ok(Type::Unevaluated(expression_id));
        }
        types.mark_expression_type_in_progress(global_node_id);

        let result = self.evaluate_expression_to_type(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
            resolve_static_arguments,
        );

        types.clear_expression_type_in_progress(global_node_id);

        // evaluate to a concrete type when possible
        let ty = result?.unwrap_or(Type::Unevaluated(expression_id));
        Ok(ty)
    }

    /// Evaluate an expression into a static integer literal when possible.
    fn evaluate_integer_static_literal(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<i64>> {
        let value = self.evaluate_static_expression_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            None,
        )?;
        let literal = match value {
            Some(StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            }) => Some(value),
            _ => None,
        };
        Ok(literal)
    }

    /// Validate that array sizes only use comptime static parameters.
    pub(crate) fn validate_static_value_parameter_usage_in_type_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<()> {
        // skip comptime checks outside Destack modules
        if !module.language_type.is_destack() {
            return Ok(());
        }

        let mut validator = StaticValueParameterValidator::new(
            self,
            module,
            profile,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        );

        // walk the expression tree to validate index usages
        let expression = tree.get(expression_id);
        validator.visit_expression(tree, expression_id, expression);
        validator.finish()
    }

    /// Register a scalar literal type for a static integer expression.
    fn set_integer_literal_type(
        &self,
        module_id: destack_source::ModuleId,
        expression_id: LocalNodeId<Expression>,
        value: i64,
        types: &mut TypeTable,
    ) {
        let literal_type = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
        };
        let literal_type_id = types.insert_type_from(literal_type, expression_id);
        types.set_inferred_type(expression_id.into_global_any(module_id), literal_type_id);
    }

    /// Resolve the inferred type for a static value parameter used as an array size.
    fn resolve_array_size_parameter_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip comptime size validation outside destack modules
        if !module.language_type.is_destack() {
            return Ok(None);
        }

        // skip expressions that are not static parameter references
        let kind = if let Some(kind) = self.static_parameter_reference_kind(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
        ) {
            kind
        } else {
            let index_ty_id = self.try_evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )?;
            let Type::Reference { symbol, .. } = types.get_type(index_ty_id) else {
                return Ok(None);
            };
            if !self.symbol_is_static_parameter(module, profile, *symbol, symbols, types) {
                return Ok(None);
            }
            self.static_parameter_kind_for_symbol(module, profile, *symbol, tree, symbols, types)
        };

        // require comptime for value usage
        if kind != StaticParameterKind::Value {
            self.error(AnalyzeError::StaticParameterRequiresComptime {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
            return Ok(None);
        }

        // resolve the parameter type and cache it as the inferred type
        let index_id = self.try_evaluate_expression_to_type(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;
        types.set_inferred_type(expression_id.into_global_any(module.id), index_id);
        Ok(Some(index_id))
    }

    /// Check whether a type supports indexed access in a type expression.
    fn type_supports_index_access(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<bool> {
        // track visited types to avoid cycles
        let mut visited = HashSet::new();
        let mut current_type_id = type_id;

        loop {
            // stop on cycles
            if !visited.insert(current_type_id) {
                return Ok(false);
            }

            // resolve the current type
            let ty = types.get_type(current_type_id).clone();
            match ty {
                Type::Tuple { .. } | Type::Array { .. } | Type::ArraySized { .. } => {
                    return Ok(true);
                }
                Type::TypeLiteral {
                    value: TypeLiteral::Any | TypeLiteral::Unknown,
                } => {
                    // allow type indexing for unknown and any to avoid array-size misclassification
                    return Ok(true);
                }
                Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                } => {
                    return Ok(!fields.is_empty()
                        || !call_signatures.is_empty()
                        || !construct_signatures.is_empty()
                        || !index_signatures.is_empty());
                }
                Type::Reference { symbol, .. } => {
                    // follow static parameter constraints when available
                    if let Some(constraint) = self.static_parameter_constraint_type(
                        module,
                        profile,
                        symbol,
                        types.get_type_source(current_type_id),
                        symbols,
                        types,
                    ) {
                        current_type_id = constraint;
                        continue;
                    }

                    // follow alias targets when available
                    if let Some(alias_target) = types.get_alias_target_type_id(symbol) {
                        current_type_id = alias_target;
                        continue;
                    }

                    // follow instance types when available
                    if let Some(instance_type) = types.get_instance_type_id(symbol) {
                        current_type_id = instance_type;
                        continue;
                    }

                    return Ok(false);
                }
                Type::Unevaluated(_) => {
                    // evaluate before checking index support
                    self.evaluate_type(module, profile, current_type_id, tree, symbols, types)?;
                }
                _ => return Ok(false),
            }
        }
    }

    /// Check whether a type resolves to a primitive literal.
    fn type_is_primitive_literal(&self, type_id: LocalTypeId, types: &TypeTable) -> bool {
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(_),
            } => true,
            Type::Reference { symbol, .. } => {
                if let Some(alias_id) = types.get_alias_target_type_id(*symbol) {
                    matches!(
                        types.get_type(alias_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(_),
                        }
                    )
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Resolve the static parameter kind for a reference expression.
    fn static_parameter_reference_kind(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<StaticParameterKind> {
        // only treat references as static parameters in Destack modules
        if !module.language_type.is_destack() {
            return None;
        }

        // unwrap type unary wrappers to reach the reference
        if let Expression::TypeUnary { right, .. } = tree.get(expression_id) {
            return self
                .static_parameter_reference_kind(module, profile, *right, tree, symbols, types);
        }

        // resolve the referenced symbol first
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tree.get(expression_id)
        else {
            return None;
        };

        // prefer local symbol tables for local references
        if target_symbol.module_id == module.id {
            return self.static_parameter_reference_kind_in_symbols(
                module,
                profile,
                *target_symbol,
                tree,
                symbols,
                types,
            );
        }

        // use the owning module to avoid indexing the wrong symbol table
        let remote_module = self.program.modules.get(target_symbol.module_id);
        let remote_module = remote_module.read();
        let remote_tree = remote_module.dir(profile).tree.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        self.static_parameter_reference_kind_in_symbols(
            &remote_module,
            profile,
            *target_symbol,
            &remote_tree,
            &remote_symbols,
            types,
        )
    }

    /// Resolve the static parameter kind for a symbol within a symbol table.
    fn static_parameter_reference_kind_in_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<StaticParameterKind> {
        // resolve direct static parameter references
        if self.symbol_is_static_parameter(module, profile, target_symbol, symbols, types) {
            return Some(self.static_parameter_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
                types,
            ));
        }

        // fall back to a same-scope static parameter with the same key
        let symbol_entry = symbols.get_symbol(target_symbol.local_id);
        let key = symbol_entry.key?;
        let mut scope_cursor = Some(symbol_entry.scope);
        while let Some((scope_id, mark)) = scope_cursor {
            let scope = symbols.get_scope_by_id(scope_id);
            let limit = mark.0 as usize;
            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().take(limit).rev()
            {
                if *candidate_key != key {
                    continue;
                }
                let candidate_symbol = symbols.get_symbol(*candidate_symbol_id);
                if !candidate_symbol.is_active || !candidate_symbol.is_static_parameter() {
                    continue;
                }
                let candidate_global = candidate_symbol_id.into_global(module.id);
                return Some(self.static_parameter_kind_for_symbol(
                    module,
                    profile,
                    candidate_global,
                    tree,
                    symbols,
                    types,
                ));
            }

            scope_cursor = scope.parent;
        }

        None
    }

    /// Try to evaluate an Expression as a Type id.
    /// Set validate_static_argument_bounds to false to defer bound checks.
    /// Set enforce_implicit_managed to false to skip noImplicitManaged enforcement.
    pub(crate) fn try_evaluate_expression_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        // reuse cached expression types when available
        let global_node_id = expression_id.into_global_any(module.id);
        if let Some(existing) = types.get_declared_type_id(global_node_id) {
            // avoid reusing unvalidated instantiations when bounds are required
            let has_static_arguments = matches!(
                types.get_type(existing),
                Type::Reference {
                    static_arguments: Some(arguments),
                    ..
                } if !arguments.is_empty()
            );
            if validate_static_argument_bounds && has_static_arguments {
                // fall through to re-evaluate with bound validation enabled
            } else {
                let is_unknown = matches!(
                    types.get_type(existing),
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown
                    }
                );
                let is_reference = matches!(
                    tree.get(expression_id),
                    Expression::LocalReference { .. }
                        | Expression::ModuleReference { .. }
                        | Expression::GlobalReference { .. }
                );
                if !(is_unknown && is_reference) {
                    return Ok(existing);
                }
            }
        }

        // evaluate to a concrete type when possible
        let ty = self.try_evaluate_expression_to_type_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
            true,
            true,
        )?;
        let ty_id = types.insert_type_from(ty.clone(), expression_id);

        // cache resolved type expressions for reuse
        let should_cache = !matches!(ty, Type::Unevaluated(_))
            && (validate_static_argument_bounds
                || !matches!(
                    ty,
                    Type::Reference {
                        static_arguments: Some(arguments),
                        ..
                    } if !arguments.is_empty()
                ));
        if should_cache {
            types.set_declared_type(global_node_id, ty_id);
        }

        Ok(ty_id)
    }

    /// Evaluate a function signature into a Type.
    pub(super) fn evaluate_function_signature_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        signature: &FunctionSignature,
        source_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // collect static parameter placeholders
        let static_parameters =
            self.static_parameter_placeholders_for_signature(module, signature, tree, types);

        // evaluate parameter types
        let mut dynamic_parameters = Vec::with_capacity(signature.dynamic_parameters.len());
        for parameter_id in signature.dynamic_parameters.iter() {
            let declared_type_id = types
                .get_declared_type_id(parameter_id.into_global(module.id).into())
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, *parameter_id)
                });

            self.evaluate_type(module, profile, declared_type_id, tree, symbols, types)?;

            dynamic_parameters.push(declared_type_id);
        }

        // evaluate this parameter when present
        let this_parameter = if let Some(this_parameter_id) = signature.this_parameter {
            let declared_type_id = types
                .get_declared_type_id(this_parameter_id.into_global(module.id).into())
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, this_parameter_id)
                });

            self.evaluate_type(module, profile, declared_type_id, tree, symbols, types)?;

            Some(declared_type_id)
        } else {
            None
        };

        // evaluate return type
        let return_type = if let Some(return_type_id) = signature.return_type {
            Some(self.try_evaluate_expression_to_type(
                module,
                profile,
                return_type_id,
                tree,
                symbols,
                types,
                true,
                true,
            )?)
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            Some(types.insert_type_from_any(ty, source_id))
        };

        // build function type
        Ok(Type::Function {
            asynchrony: signature.asynchrony,
            cardinality: signature.cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        })
    }

    /// Evaluate static arguments for a type reference.
    fn evaluate_static_arguments(
        &self,
        module: &Module,
        _profile: ProfileId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // skip when there are no static arguments
        let Some(static_arguments) = static_arguments else {
            return Ok(None);
        };

        let mut evaluated_arguments = Vec::with_capacity(static_arguments.len());

        // defer static argument evaluation until parameter kinds are known
        for argument_id in static_arguments {
            evaluated_arguments.push(StaticArgument::Unevaluated {
                node: argument_id.into_global_any(module.id),
            });
        }

        Ok(Some(evaluated_arguments))
    }

    /// Evaluate a template literal span expression into a type id.
    fn evaluate_template_literal_span_type(
        &self,
        module: &Module,
        profile: ProfileId,
        span_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve direct references to avoid caching template spans as unknown
        match tree.get(span_id) {
            Expression::LocalReference {
                target_symbol,
                static_arguments,
                ..
            }
            | Expression::ModuleReference {
                target_symbol,
                static_arguments,
                ..
            }
            | Expression::GlobalReference {
                target_symbol,
                static_arguments,
                ..
            } => {
                let ty = self.evaluate_template_literal_span_reference(
                    module,
                    profile,
                    span_id,
                    *target_symbol,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                )?;
                return Ok(ty);
            }
            Expression::UnresolvedPath {
                path,
                static_arguments,
                space_order,
            } => {
                let resolved_symbol = self.resolve_template_literal_span_path(
                    module,
                    span_id,
                    path,
                    *space_order,
                    tree,
                    symbols,
                );
                if let Some(resolved_symbol) = resolved_symbol {
                    let ty = self.evaluate_template_literal_span_reference(
                        module,
                        profile,
                        span_id,
                        resolved_symbol,
                        static_arguments.as_deref(),
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                    )?;
                    return Ok(ty);
                }
            }
            _ => {}
        }

        // bypass declared caches to avoid collapsing span references to unknown
        let ty = self.try_evaluate_expression_to_type_value(
            module,
            profile,
            span_id,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
            true,
            false,
        )?;

        Ok(types.insert_type_from(ty, span_id))
    }

    /// Resolve a template literal span to a reference type.
    fn evaluate_template_literal_span_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        span_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        // follow dependency items for local imports before canonicalization
        let mut target_symbol = target_symbol;
        if target_symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(target_symbol.local_id);
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::DependencyItem
            {
                let item_id = primary_declaration.local_id.into_typed::<DependencyItem>();
                if let DependencyItem::Local {
                    target_symbol: dependency_target,
                    ..
                }
                | DependencyItem::Remote {
                    target_symbol: dependency_target,
                    ..
                } = tree.get(item_id)
                {
                    target_symbol = *dependency_target;
                }
            }
        }

        // preserve alias identity while resolving the reference
        let target_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            target_symbol,
            CanonicalSymbolMode::PreserveAliases,
        );
        let target_symbol = self.merged_type_symbol_id(module, symbols, profile, target_symbol);

        // reject value static parameters in template spans
        if self.symbol_is_static_parameter(module, profile, target_symbol, symbols, types) {
            let kind = self.static_parameter_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
                types,
            );
            if kind == StaticParameterKind::Value {
                self.error(AnalyzeError::StaticParameterRequiresComptime {
                    node: span_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from(ty, span_id));
            }
        }

        // resolve static arguments for the referenced span
        let options = self.analyze_context_options_for_module(module.id);
        let static_arguments = self.evaluate_static_arguments(
            module,
            profile,
            static_arguments,
            tree,
            symbols,
            types,
        )?;
        let resolved_arguments = self.resolve_type_reference_static_arguments(
            module,
            profile,
            span_id.into_any(),
            target_symbol,
            static_arguments.as_deref(),
            validate_static_argument_bounds,
            &options,
            tree,
            symbols,
            types,
        )?;

        let ty = Type::Reference {
            symbol: target_symbol,
            static_arguments: resolved_arguments,
        };
        Ok(types.insert_type_from(ty, span_id))
    }

    /// Resolve a template literal span path to a symbol id.
    fn resolve_template_literal_span_path(
        &self,
        module: &Module,
        span_id: LocalNodeId<Expression>,
        path: &Path,
        space_order: SymbolSpaceOrder,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // only handle single segment identifiers
        if path.segments.len() != 1 {
            return None;
        }

        let key = StaticKey::Name(path.segments[0]);
        let (scope_id, scope, _mark) = symbols.get_scope(span_id, tree);
        let mut scope_cursor = Some((scope_id, scope));
        let mut fallback = None;
        let preferred_spaces = space_order.spaces();

        // walk scopes from inner to outer
        while let Some((_scope_id, scope)) = scope_cursor {
            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().rev() {
                if *candidate_key != key {
                    continue;
                }
                let candidate = symbols.get_symbol(*candidate_symbol_id);
                if !candidate.is_active {
                    continue;
                }

                let candidate_space = candidate.space;
                if candidate_space == SymbolSpace::TypeValue
                    || preferred_spaces.contains(&candidate_space)
                {
                    return Some(candidate_symbol_id.into_global(module.id));
                }

                if fallback.is_none() {
                    fallback = Some(candidate_symbol_id.into_global(module.id));
                }
            }

            scope_cursor = scope
                .parent
                .map(|(parent_id, _parent_mark)| (parent_id, symbols.get_scope_by_id(parent_id)));
        }

        fallback
    }

    /// Collect element types for a binary union or intersection expression.
    /// (This is a faster and deterministic alternative for the elementwise combinators.)
    fn collect_binary_type_elements(
        &self,
        module: &Module,
        profile: ProfileId,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
        operator: BinaryOperator,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // seed the work list right-to-left so left is processed first
        let mut pending_expressions = Vec::new();
        pending_expressions.push(right);
        pending_expressions.push(left);

        // walk the binary tree
        let mut elements = Vec::new();
        while let Some(expression_id) = pending_expressions.pop() {
            let expression = tree.get(expression_id);

            // unwrap parenthesized expressions
            if let Expression::Parenthesized { expression } = expression {
                pending_expressions.push(*expression);
                continue;
            }

            // flatten nested union or intersection expressions
            if let Expression::Binary {
                left,
                operator: nested_operator,
                right,
                ..
            } = expression
                && *nested_operator == operator
            {
                // push right first to preserve left-to-right order
                pending_expressions.push(*right);
                pending_expressions.push(*left);
                continue;
            }

            // evaluate the leaf expression to a type id
            let element_id = self.try_evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )?;

            // flatten nested union or intersection types
            match (operator, types.get_type(element_id)) {
                (
                    BinaryOperator::ElementwiseOr,
                    Type::Union {
                        elements: union_elements,
                    },
                ) => {
                    elements.extend_from_slice(union_elements);
                }
                (
                    BinaryOperator::ElementwiseAnd,
                    Type::Intersection {
                        elements: intersection_elements,
                    },
                ) => {
                    elements.extend_from_slice(intersection_elements);
                }
                _ => {
                    elements.push(element_id);
                }
            }
        }

        Ok(elements)
    }

    /// Evaluate an expression into a static value expression.
    pub(crate) fn evaluate_static_expression_value(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        enum_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        let mut visited = HashSet::new();
        self.evaluate_static_expression_value_inner(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            enum_symbol,
            &mut visited,
        )
    }

    /// Evaluate an expression into a static value expression.
    #[allow(clippy::only_used_in_recursion)]
    fn evaluate_static_expression_value_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        enum_symbol: Option<GlobalSymbolId>,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        let expression = tree.get(expression_id);

        let value = match expression {
            Expression::ScalarLiteral { value } => StaticExpression::ScalarLiteral {
                value: value.clone(),
            },
            Expression::TypeLiteral { value } => StaticExpression::TypeLiteral {
                value: value.clone(),
            },
            Expression::Type { value } => StaticExpression::Type { ty: *value },
            Expression::Parenthesized { expression } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *expression,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    visited,
                );
            }
            Expression::Cast { value, .. } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *value,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    visited,
                );
            }
            Expression::OwnershipCast { value, .. } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *value,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    visited,
                );
            }
            Expression::Unary { operator, right } => {
                let right_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *right,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    visited,
                )?;
                let Some(StaticExpression::ScalarLiteral { value }) = right_value else {
                    return Ok(None);
                };
                let ScalarLiteral::Integer(value) = value else {
                    return Ok(None);
                };
                let value = match operator {
                    UnaryOperator::Plus => value,
                    UnaryOperator::Negate => -value,
                    UnaryOperator::ElementwiseNot => !value,
                    _ => return Ok(None),
                };
                StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(value),
                }
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *left,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    visited,
                )?;
                let right_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *right,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    visited,
                )?;
                let (left_value, right_value) = match (left_value, right_value) {
                    (
                        Some(StaticExpression::ScalarLiteral {
                            value: ScalarLiteral::Integer(left_value),
                        }),
                        Some(StaticExpression::ScalarLiteral {
                            value: ScalarLiteral::Integer(right_value),
                        }),
                    ) => (left_value, right_value),
                    _ => return Ok(None),
                };

                let value = match operator {
                    BinaryOperator::Add => left_value.checked_add(right_value),
                    BinaryOperator::Subtract => left_value.checked_sub(right_value),
                    BinaryOperator::Multiply => left_value.checked_mul(right_value),
                    BinaryOperator::Divide => {
                        if right_value == 0 {
                            None
                        } else {
                            let remainder = left_value % right_value;
                            if remainder != 0 {
                                None
                            } else {
                                left_value.checked_div(right_value)
                            }
                        }
                    }
                    BinaryOperator::Remainder => left_value.checked_rem(right_value),
                    BinaryOperator::ShiftLeft => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| left_value.checked_shl(shift)),
                    BinaryOperator::ShiftRight => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| left_value.checked_shr(shift)),
                    BinaryOperator::UnsignedShiftRight => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| (left_value as u64).checked_shr(shift))
                        .and_then(|shifted| i64::try_from(shifted).ok()),
                    BinaryOperator::ElementwiseAnd => Some(left_value & right_value),
                    BinaryOperator::ElementwiseOr => Some(left_value | right_value),
                    BinaryOperator::ElementwiseXor => Some(left_value ^ right_value),
                    _ => None,
                };

                let Some(value) = value else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(value),
                }
            }
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                // static parameter references
                if let Some(kind) = self.static_parameter_reference_kind(
                    module,
                    profile,
                    expression_id,
                    tree,
                    symbols,
                    types,
                ) {
                    if kind == StaticParameterKind::Value {
                        let reference_type = Type::Reference {
                            symbol: *target_symbol,
                            static_arguments: None,
                        };
                        let ty = types.insert_type_from(reference_type, expression_id);
                        return Ok(Some(StaticExpression::Type { ty }));
                    }

                    self.error(AnalyzeError::StaticParameterRequiresComptime {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                    return Ok(None);
                }

                // unwrap import/export dependency items before canonicalizing
                let mut lookup_symbol = *target_symbol;
                if lookup_symbol.module_id == module.id {
                    let symbol_entry = symbols.get_symbol(lookup_symbol.local_id);
                    if let Some(primary_declaration) = symbol_entry.primary_declaration
                        && primary_declaration.local_id.ty == NodeType::DependencyItem
                    {
                        let item_id = primary_declaration.local_id.into_typed::<DependencyItem>();
                        if let DependencyItem::Local { target_symbol, .. }
                        | DependencyItem::Remote { target_symbol, .. } = tree.get(item_id)
                        {
                            lookup_symbol = *target_symbol;
                        }
                    }
                }

                if let Some(value) = self.static_expression_from_constant_reference(
                    module,
                    profile,
                    lookup_symbol,
                    tree,
                    symbols,
                    types,
                    visited,
                )? {
                    return Ok(Some(value));
                }

                let Some(enum_symbol) = enum_symbol else {
                    return Ok(None);
                };
                let value = self.enum_field_value_for_symbol_reference(
                    module,
                    profile,
                    enum_symbol,
                    *target_symbol,
                    symbols,
                    types,
                );
                let Some(value) = value else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: match value {
                        EnumFieldValue::Int(value) => ScalarLiteral::Integer(value),
                        EnumFieldValue::String(value) => ScalarLiteral::String(value),
                    },
                }
            }
            Expression::Member {
                left: _,
                name,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Ok(None);
                }
                let Some(enum_symbol) = enum_symbol else {
                    return Ok(None);
                };

                let node_id = expression_id.into_global_any(module.id);
                let resolution_id = types.get_resolution_for_node(node_id);
                let target_symbol = if let Some(resolution_id) = resolution_id {
                    let resolution = types.get_resolution(resolution_id);
                    let Resolution::Static { candidate, .. } = resolution else {
                        return Ok(None);
                    };
                    candidate.target_symbol
                } else {
                    let Some(field_symbol) = self.enum_field_symbol_for_name(
                        module,
                        profile,
                        enum_symbol,
                        *name,
                        tree,
                        symbols,
                    ) else {
                        return Ok(None);
                    };

                    field_symbol
                };

                let value = self.enum_field_value_for_symbol_reference(
                    module,
                    profile,
                    enum_symbol,
                    target_symbol,
                    symbols,
                    types,
                );
                let Some(value) = value else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: match value {
                        EnumFieldValue::Int(value) => ScalarLiteral::Integer(value),
                        EnumFieldValue::String(value) => ScalarLiteral::String(value),
                    },
                }
            }
            Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let start_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *start,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    visited,
                )?;
                let end_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *end,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    visited,
                )?;

                let (Some(start_value), Some(end_value)) = (start_value, end_value) else {
                    return Ok(None);
                };

                StaticExpression::RangeExpression {
                    start: Box::new(start_value),
                    end: Box::new(end_value),
                    is_inclusive: *is_inclusive,
                }
            }
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = tree.get(*element_id);
                    // reject sparse array holes
                    if matches!(tree.get(element.value()), Expression::Stub) {
                        return Err(AnalyzeError::ArrayLiteralHole {
                            node: element
                                .value()
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
                    let value = self.evaluate_static_expression_value_inner(
                        module,
                        profile,
                        element.value(),
                        tree,
                        symbols,
                        types,
                        enum_symbol,
                        visited,
                    )?;
                    let Some(value) = value else {
                        return Ok(None);
                    };
                    values.push(value);
                }

                StaticExpression::ArrayExpression { elements: values }
            }
            Expression::TupleExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value_inner(
                        module,
                        profile,
                        element.value(),
                        tree,
                        symbols,
                        types,
                        enum_symbol,
                        visited,
                    )?;
                    let Some(value) = value else {
                        return Ok(None);
                    };
                    values.push(value);
                }

                StaticExpression::TupleExpression { elements: values }
            }
            Expression::ObjectExpression { properties } => {
                let mut evaluated_properties = Vec::with_capacity(properties.len());
                for property_id in properties {
                    let property = tree.get(*property_id).clone();
                    let evaluated_property = match property {
                        Property::Field {
                            modifiers,
                            key,
                            value,
                            default,
                            symbol,
                        } => {
                            let Some(value_id) = value else {
                                return Ok(None);
                            };
                            let value = self.evaluate_static_expression_value_inner(
                                module,
                                profile,
                                value_id,
                                tree,
                                symbols,
                                types,
                                enum_symbol,
                                visited,
                            )?;
                            let Some(value) = value else {
                                return Ok(None);
                            };
                            let default = if let Some(default_id) = default {
                                let default_value = self.evaluate_static_expression_value_inner(
                                    module,
                                    profile,
                                    default_id,
                                    tree,
                                    symbols,
                                    types,
                                    enum_symbol,
                                    visited,
                                )?;
                                let Some(default_value) = default_value else {
                                    return Ok(None);
                                };
                                Some(default_value)
                            } else {
                                None
                            };

                            StaticProperty::Field {
                                modifiers,
                                key,
                                value,
                                default,
                                symbol,
                            }
                        }
                        Property::Method { .. } | Property::Spread { .. } => {
                            return Ok(None);
                        }
                    };
                    evaluated_properties.push(evaluated_property);
                }

                StaticExpression::ObjectExpression {
                    properties: evaluated_properties,
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(value))
    }

    /// Resolve constant bindings into static expressions when possible.
    fn static_expression_from_constant_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        // evaluate using the owning module context
        if symbol.module_id != module.id {
            self.require_analyze_module_infer(symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;

            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_dir = remote_module.dir(profile);
            let remote_tree = remote_dir.tree.read();
            let remote_symbols = remote_dir.symbols.read();
            let mut remote_types = remote_dir.types.write();
            return self.static_expression_from_constant_reference(
                &remote_module,
                profile,
                symbol,
                &remote_tree,
                &remote_symbols,
                &mut remote_types,
                visited,
            );
        }

        // avoid recursive constant evaluation
        if !visited.insert(symbol) {
            return Ok(None);
        }

        // ensure dependency items are resolved before evaluating local constants
        if symbol.module_id == module.id {
            self.require_resolve_module_direct(module.id, profile)
                .map_err(AnalyzeError::from)?;
        }

        let value = if let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(module, symbol, tree, symbols)
        {
            let declarator = tree.get(declarator_id);

            // require immutable bindings for static arguments
            let parent_id = tree.get_parent(declarator_id.id);
            let Some(parent_id) = parent_id else {
                visited.remove(&symbol);
                return Ok(None);
            };
            if parent_id.ty != NodeType::Expression {
                visited.remove(&symbol);
                return Ok(None);
            }

            let expression_id = parent_id.into_typed::<Expression>();
            let Expression::Let { mutability, .. } = tree.get(expression_id) else {
                visited.remove(&symbol);
                return Ok(None);
            };
            if *mutability != Mutability::Immutable {
                visited.remove(&symbol);
                return Ok(None);
            }

            // evaluate the initializer as a static expression
            let Some(value_id) = declarator.value else {
                visited.remove(&symbol);
                return Ok(None);
            };
            self.evaluate_static_expression_value_inner(
                module, profile, value_id, tree, symbols, types, None, visited,
            )?
        } else {
            let symbol_entry = symbols.get_symbol(symbol.local_id);

            // follow export/import dependency targets when available
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::DependencyItem
                && let item_id = primary_declaration.local_id.into_typed::<DependencyItem>()
                && let DependencyItem::Local { target_symbol, .. }
                | DependencyItem::Remote { target_symbol, .. } = tree.get(item_id)
                && let Some(value) = self.static_expression_from_constant_reference(
                    module,
                    profile,
                    *target_symbol,
                    tree,
                    symbols,
                    types,
                    visited,
                )?
            {
                visited.remove(&symbol);
                return Ok(Some(value));
            }

            // follow export expressions that wrap dependency items
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::Expression
            {
                let expression_id = primary_declaration.local_id.into_typed::<Expression>();
                if let Expression::Export { items, .. } = tree.get(expression_id) {
                    for item_id in items {
                        match tree.get(*item_id) {
                            DependencyItem::Local {
                                symbol: Some(local_symbol),
                                target_symbol,
                                ..
                            }
                            | DependencyItem::Remote {
                                symbol: Some(local_symbol),
                                target_symbol,
                                ..
                            } if *local_symbol == symbol.local_id => {
                                if let Some(value) = self
                                    .static_expression_from_constant_reference(
                                        module,
                                        profile,
                                        *target_symbol,
                                        tree,
                                        symbols,
                                        types,
                                        visited,
                                    )?
                                {
                                    visited.remove(&symbol);
                                    return Ok(Some(value));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            if let Some(target_symbol) = symbol_entry.target_symbol {
                self.static_expression_from_constant_reference(
                    module,
                    profile,
                    target_symbol,
                    tree,
                    symbols,
                    types,
                    visited,
                )?
            } else if let Some(canonical_symbol) = symbol_entry.canonical_symbol {
                self.static_expression_from_constant_reference(
                    module,
                    profile,
                    canonical_symbol,
                    tree,
                    symbols,
                    types,
                    visited,
                )?
            } else {
                // fall back to inferred literal value types
                if let Some(value_type_id) = types.get_value_type_id(symbol)
                    && let Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value),
                    } = types.get_type(value_type_id)
                {
                    Some(StaticExpression::ScalarLiteral {
                        value: value.clone(),
                    })
                } else {
                    None
                }
            }
        };

        visited.remove(&symbol);
        Ok(value)
    }

    /// Evaluate an Expression into a Type with validation controls.
    fn evaluate_expression_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
        resolve_static_arguments: bool,
    ) -> AnalyzeResult<Option<Type>> {
        let options = self.analyze_context_options_for_module(module.id);
        let is_user_module = matches!(module.source, ModuleSource::User);

        // clone to avoid holding a tree borrow across recursive evaluation
        let expression = tree.get(expression_id).clone();

        let ty = match expression {
            Expression::ScalarLiteral { value } => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value.clone()),
            },
            Expression::TypeLiteral { value } => {
                // reject forbidden type literals in user code
                if is_user_module {
                    // disallow explicit any
                    if options.no_any && matches!(value, TypeLiteral::Any) {
                        return Err(AnalyzeError::AnyTypeDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }

                    // disallow explicit unknown
                    if options.no_unknown && matches!(value, TypeLiteral::Unknown) {
                        return Err(AnalyzeError::UnknownTypeDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }

                    // disallow imprecise primitives
                    if options.no_imprecise_primitives
                        && matches!(value, TypeLiteral::Primitive(PrimitiveType::Number))
                    {
                        return Err(AnalyzeError::ImprecisePrimitiveDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
                }

                // map builtin iterator return to configured strictness
                if let TypeLiteral::Intrinsic(IntrinsicType::BuiltinIteratorReturn) = value {
                    let profile = self.program.profile(profile);
                    let mapped = if profile.key.flags.strict_builtin_iterator_return {
                        TypeLiteral::Undefined
                    } else {
                        TypeLiteral::Any
                    };
                    Type::TypeLiteral { value: mapped }
                } else {
                    Type::TypeLiteral {
                        value: value.clone(),
                    }
                }
            }
            Expression::This => Type::This,
            Expression::Parenthesized { expression } => {
                return self.evaluate_expression_to_type(
                    module,
                    profile,
                    expression,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                    resolve_static_arguments,
                );
            }

            Expression::Declaration {
                declaration: declaration_id,
            } => {
                let declaration = tree.get(declaration_id).clone();
                if let Declaration::Function { signature, .. } = declaration {
                    self.evaluate_function_signature_to_type(
                        module,
                        profile,
                        &signature,
                        declaration_id.into_any(),
                        tree,
                        symbols,
                        types,
                    )?
                } else {
                    // #Incomplete: only function declarations are evaluable as types (?)
                    return Ok(None);
                }
            }

            // not
            Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Unary {
                    operator: TypeUnaryOperator::Not,
                    right: type_id,
                }
            }
            // maybe
            Expression::Maybe { .. } => {
                return Ok(None); // cannot be evaluated to a type here (not supported in type contexts)
            }
            // must
            Expression::Must { left } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Unary {
                    operator: TypeUnaryOperator::Must,
                    right: type_id,
                }
            }
            // value
            Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::ValueOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // reference
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::ReferenceOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // pointer
            Expression::PointerOf { mutability, right } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::PointerOf {
                    mutability,
                    right: type_id,
                }
            }
            // unary
            Expression::TypeUnary { operator, right } => {
                if operator == TypeUnaryOperator::Typeof {
                    return Ok(Some(self.evaluate_typeof_expression(
                        module,
                        profile,
                        expression_id,
                        right,
                        tree,
                        symbols,
                        types,
                    )?));
                }

                let right_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Unary {
                    operator,
                    right: right_id,
                }
            }
            // binary
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let right_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Binary {
                    left: left_id,
                    operator,
                    right: right_id,
                }
            }
            Expression::TypeConditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                let left_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let right_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let distributive_symbol = self
                    .conditional_left_distributive_symbol(module, profile, left_id, symbols, types);
                let should_validate_branches = !self.type_contains_static_parameters(
                    module,
                    profile,
                    left_id,
                    symbols,
                    types,
                    &mut HashSet::new(),
                ) && validate_static_argument_bounds;
                let then_type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    then_type,
                    tree,
                    symbols,
                    types,
                    should_validate_branches,
                    enforce_implicit_managed,
                )?;
                let else_type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    else_type,
                    tree,
                    symbols,
                    types,
                    should_validate_branches,
                    enforce_implicit_managed,
                )?;
                Type::Conditional {
                    distributive_symbol,
                    left: left_id,
                    right: right_id,
                    then_type: then_type_id,
                    else_type: else_type_id,
                }
            }
            Expression::TypeMapped {
                parameter,
                modifiers,
                value,
            } => {
                let constraint = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    parameter.constraint,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                // cache the mapped parameter constraint for later validation
                let parameter_symbol = parameter.symbol.into_global(module.id);
                types.set_static_parameter_constraint_type(parameter_symbol, constraint);
                let key_remap = parameter.key_remap.map(|key_remap| {
                    self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        key_remap,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let key_remap = match key_remap {
                    Some(Ok(key_remap)) => Some(key_remap),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                let value_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    value,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let parameter = TypeMappedParameter {
                    name: parameter.name,
                    symbol: parameter_symbol,
                    constraint,
                    key_remap,
                };
                Type::Mapped {
                    parameter,
                    modifiers,
                    value: value_id,
                }
            }
            Expression::TypeIndex { left, index } => {
                // resolve the left type
                let left_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;

                // treat declaration modules as index access only
                let is_index_access = if module.language_type.is_declaration() {
                    true
                } else {
                    // check whether the left type supports index access
                    let supports_index_access = self.type_supports_index_access(
                        module, profile, left_id, tree, symbols, types,
                    )?;
                    let is_primitive_literal = self.type_is_primitive_literal(left_id, types);
                    supports_index_access && !is_primitive_literal
                };

                // compute the type index result
                if !is_index_access {
                    // treat static integer literals as array sizes
                    if let Some(value) = self.evaluate_integer_static_literal(
                        module, profile, index, tree, symbols, types,
                    )? {
                        if value < 0 {
                            return Err(AnalyzeError::InvalidArraySize {
                                node: index
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            });
                        }
                        self.set_integer_literal_type(module.id, index, value, types);
                        Type::ArraySized {
                            element: left_id,
                            count: index,
                            is_readonly: false,
                        }
                    } else if self
                        .resolve_array_size_parameter_type(
                            module,
                            profile,
                            index,
                            tree,
                            symbols,
                            types,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )?
                        .is_some()
                    {
                        Type::ArraySized {
                            element: left_id,
                            count: index,
                            is_readonly: false,
                        }
                    } else {
                        let index_id = self.try_evaluate_expression_to_type(
                            module,
                            profile,
                            index,
                            tree,
                            symbols,
                            types,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )?;
                        Type::Index {
                            left: left_id,
                            index: index_id,
                        }
                    }
                } else {
                    let index_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        index,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    Type::Index {
                        left: left_id,
                        index: index_id,
                    }
                }
            }
            Expression::TypeTemplateLiteral { strings, spans } => {
                let spans = spans
                    .iter()
                    .map(|span| {
                        self.evaluate_template_literal_span_type(
                            module,
                            profile,
                            *span,
                            tree,
                            symbols,
                            types,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )
                    })
                    .collect::<AnalyzeResult<Vec<_>>>()?;
                Type::TemplateLiteral {
                    strings: strings.clone(),
                    spans,
                }
            }
            Expression::TypeImport {
                target,
                qualifier,
                static_arguments,
            } => {
                let static_arguments = self.evaluate_static_arguments(
                    module,
                    profile,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                )?;
                Type::Import {
                    target,
                    qualifier: qualifier.clone(),
                    static_arguments,
                }
            }
            Expression::TypeInfer { name, constraint } => {
                let constraint = constraint.map(|constraint| {
                    self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        constraint,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let constraint = match constraint {
                    Some(Ok(constraint)) => Some(constraint),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                Type::Infer { name, constraint }
            }
            Expression::TypePredicate {
                asserts,
                subject,
                target,
            } => {
                let target = target.map(|target| {
                    self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        target,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let target = match target {
                    Some(Ok(target)) => Some(target),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                Type::Predicate {
                    asserts,
                    subject,
                    target,
                }
            }

            // union and intersection types
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => match operator {
                BinaryOperator::ElementwiseOr => {
                    let elements = self.collect_binary_type_elements(
                        module,
                        profile,
                        left,
                        right,
                        operator,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    Type::Union { elements }
                }
                BinaryOperator::ElementwiseAnd => {
                    let elements = self.collect_binary_type_elements(
                        module,
                        profile,
                        left,
                        right,
                        operator,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    Type::Intersection { elements }
                }
                _ => return Ok(None),
            },

            // references
            Expression::LocalReference {
                target_symbol,
                static_arguments,
                ..
            }
            | Expression::ModuleReference {
                target_symbol,
                static_arguments,
                ..
            }
            | Expression::GlobalReference {
                target_symbol,
                static_arguments,
                ..
            } => {
                // follow dependency items for local imports before canonicalization
                let mut target_symbol = target_symbol;
                if target_symbol.module_id == module.id {
                    let symbol_entry = symbols.get_symbol(target_symbol.local_id);
                    if let Some(primary_declaration) = symbol_entry.primary_declaration
                        && primary_declaration.local_id.ty == NodeType::DependencyItem
                    {
                        let item_id = primary_declaration.local_id.into_typed::<DependencyItem>();
                        if let DependencyItem::Local {
                            target_symbol: dependency_target,
                            ..
                        }
                        | DependencyItem::Remote {
                            target_symbol: dependency_target,
                            ..
                        } = tree.get(item_id)
                        {
                            target_symbol = *dependency_target;
                        }
                    }
                }

                // resolve import targets without collapsing type aliases
                let target_symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    profile,
                    target_symbol,
                    CanonicalSymbolMode::PreserveAliases,
                );

                // prefer merged type symbols for namespaces
                let target_symbol =
                    self.merged_type_symbol_id(module, symbols, profile, target_symbol);
                let static_arguments = self.evaluate_static_arguments(
                    module,
                    profile,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                )?;
                if !resolve_static_arguments {
                    return Ok(Some(Type::Reference {
                        symbol: target_symbol,
                        static_arguments,
                    }));
                }
                let options = self.analyze_context_options_for_module(module.id);
                let resolved_arguments = self.resolve_type_reference_static_arguments(
                    module,
                    profile,
                    expression_id.into_any(),
                    target_symbol,
                    static_arguments.as_deref(),
                    validate_static_argument_bounds,
                    &options,
                    tree,
                    symbols,
                    types,
                )?;
                let static_arguments = resolved_arguments.or(static_arguments);
                if let Some(arguments) = static_arguments.as_deref()
                    && arguments.iter().any(|argument| match argument {
                        StaticArgument::Evaluated {
                            value: StaticExpression::Type { ty },
                            ..
                        } => matches!(types.get_type(*ty), Type::Error),
                        _ => false,
                    })
                {
                    return Ok(Some(Type::Error));
                }

                // normalize well known references into canonical structural types
                if let Some(normalized) = self.normalize_well_known_type_reference(
                    module,
                    symbols,
                    profile,
                    expression_id.into_any(),
                    target_symbol,
                    static_arguments.as_deref(),
                    types,
                ) {
                    return Ok(Some(normalized));
                }

                Type::Reference {
                    symbol: target_symbol,
                    static_arguments,
                }
            }

            // tuple (anonymous)
            Expression::ArrayExpression { elements } => {
                // evaluate element types
                let mut element_types = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let argument = tree.get(element_id);
                    let value_id = argument.value();
                    let value_ty_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        value_id,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    let mut element = TypeElement::new(value_ty_id);
                    match argument {
                        Argument::Labeled { label, .. } => {
                            element.label = Some(*label);
                        }
                        Argument::Spread { .. } => {
                            element.is_rest = true;
                        }
                        _ => {}
                    }
                    let modifiers = match argument {
                        Argument::Named { modifiers, .. }
                        | Argument::Labeled { modifiers, .. }
                        | Argument::Positional { modifiers, .. }
                        | Argument::Spread { modifiers, .. } => modifiers.as_ref(),
                    };
                    if let Some(modifiers) = modifiers {
                        if matches!(modifiers.kind, Some(BindingKind::Maybe)) {
                            element.is_optional = true;
                        }
                        if matches!(modifiers.mutability, Some(Mutability::Immutable)) {
                            element.is_readonly = true;
                        }
                    }
                    element_types.push(element);
                }

                Type::Tuple {
                    elements: element_types,
                    is_readonly: false,
                }
            }

            // tuple (anonymous)
            Expression::TupleExpression { elements } => {
                // evaluate element types
                let mut element_types = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let argument = tree.get(element_id);
                    let value_id = argument.value();
                    let value_ty_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        value_id,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    let mut element = TypeElement::new(value_ty_id);
                    let modifiers = match argument {
                        Argument::Named { modifiers, .. }
                        | Argument::Labeled { modifiers, .. }
                        | Argument::Positional { modifiers, .. }
                        | Argument::Spread { modifiers, .. } => modifiers.as_ref(),
                    };
                    if let Some(modifiers) = modifiers {
                        if matches!(modifiers.kind, Some(BindingKind::Maybe)) {
                            element.is_optional = true;
                        }
                        if matches!(modifiers.mutability, Some(Mutability::Immutable)) {
                            element.is_readonly = true;
                        }
                    }
                    element_types.push(element);
                }

                Type::Tuple {
                    elements: element_types,
                    is_readonly: false,
                }
            }
            // sequence expression (comma operator)
            Expression::SequenceExpression { .. } => {
                return Err(AnalyzeError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
            }
            // object (anonymous)
            Expression::ObjectExpression { properties } => {
                // evaluate object fields
                // #Cleanup: extract property -> type field evaluation?
                let mut fields = Vec::with_capacity(properties.len());
                let mut call_signatures = Vec::new();
                let mut construct_signatures = Vec::new();
                let mut index_signatures = Vec::new();
                for property_id in properties {
                    let property = tree.get(property_id).clone();
                    let field = match property {
                        Property::Field {
                            modifiers,
                            key,
                            value,
                            ..
                        } => {
                            // index signature
                            if let Some(DynamicKey::NamedExpression { name, key }) = key {
                                let key_type = self.try_evaluate_expression_to_type(
                                    module,
                                    profile,
                                    key,
                                    tree,
                                    symbols,
                                    types,
                                    validate_static_argument_bounds,
                                    enforce_implicit_managed,
                                )?;
                                let value_type = if let Some(value_id) = value {
                                    self.try_evaluate_expression_to_type(
                                        module,
                                        profile,
                                        value_id,
                                        tree,
                                        symbols,
                                        types,
                                        validate_static_argument_bounds,
                                        enforce_implicit_managed,
                                    )?
                                } else {
                                    let ty = Type::TypeLiteral {
                                        value: TypeLiteral::Unknown,
                                    };
                                    types.insert_type_from(ty, property_id)
                                };
                                let is_readonly = modifiers.is_some_and(|modifiers| {
                                    modifiers.mutability == Some(Mutability::Immutable)
                                });
                                index_signatures.push(TypeIndexSignature {
                                    name,
                                    key_type,
                                    value_type,
                                    is_readonly,
                                });
                                continue;
                            }

                            let Some(key) = key.and_then(|key| {
                                self.static_key_from_dynamic_key(profile, key, tree, symbols, types)
                            }) else {
                                if module.language_type.is_declaration() {
                                    continue;
                                }
                                return Err(AnalyzeError::UnsupportedConstruct {
                                    node: property_id
                                        .into_global_any(module.id)
                                        .into_anchored(Some(profile)),
                                });
                            };

                            let ty = if let Some(value_id) = value {
                                self.try_evaluate_expression_to_type(
                                    module,
                                    profile,
                                    value_id,
                                    tree,
                                    symbols,
                                    types,
                                    validate_static_argument_bounds,
                                    enforce_implicit_managed,
                                )?
                            } else {
                                let ty = Type::TypeLiteral {
                                    value: TypeLiteral::Unknown,
                                };
                                types.insert_type_from(ty, property_id)
                            };
                            let is_optional = modifiers.is_some_and(|modifiers| {
                                modifiers.kind == Some(BindingKind::Maybe)
                            });
                            let is_readonly = modifiers.is_some_and(|modifiers| {
                                modifiers.mutability == Some(Mutability::Immutable)
                            });

                            TypeField {
                                key,
                                ty,
                                is_optional,
                                is_readonly,
                            }
                        }
                        Property::Method {
                            modifiers,
                            key,
                            signature,
                            ..
                        } => {
                            // call or construct signature
                            if key.is_none()
                                && matches!(
                                    signature.mode,
                                    Some(FunctionMode::Call)
                                        | Some(FunctionMode::New)
                                        | Some(FunctionMode::Constructor)
                                )
                            {
                                let ty = self.evaluate_function_signature_to_type(
                                    module,
                                    profile,
                                    &signature,
                                    property_id.into_any(),
                                    tree,
                                    symbols,
                                    types,
                                )?;
                                let ty_id = types.insert_type_from(ty, property_id);
                                match signature.mode {
                                    Some(FunctionMode::New) | Some(FunctionMode::Constructor) => {
                                        construct_signatures.push(ty_id);
                                    }
                                    _ => {
                                        call_signatures.push(ty_id);
                                    }
                                }
                                continue;
                            }

                            let Some(key) = key.and_then(|key| {
                                self.static_key_from_dynamic_key(profile, key, tree, symbols, types)
                            }) else {
                                if module.language_type.is_declaration() {
                                    continue;
                                }
                                return Err(AnalyzeError::UnsupportedConstruct {
                                    node: property_id
                                        .into_global_any(module.id)
                                        .into_anchored(Some(profile)),
                                });
                            };

                            let ty = self.evaluate_function_signature_to_type(
                                module,
                                profile,
                                &signature,
                                property_id.into_any(),
                                tree,
                                symbols,
                                types,
                            )?;
                            let ty_id = types.insert_type_from(ty, property_id);

                            let is_optional = modifiers.is_some_and(|modifiers| {
                                modifiers.kind == Some(BindingKind::Maybe)
                            });
                            let is_readonly = modifiers.is_some_and(|modifiers| {
                                modifiers.mutability == Some(Mutability::Immutable)
                            });

                            TypeField {
                                key,
                                ty: ty_id,
                                is_optional,
                                is_readonly,
                            }
                        }
                        Property::Spread { .. } => {
                            // #Incomplete: spread properties into types
                            return Err(AnalyzeError::UnsupportedConstruct {
                                node: property_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            });
                        }
                    };

                    fields.push(field);
                }

                Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                }
            }

            // array or slice
            Expression::Index { left, right } => {
                // array with static length
                if let Some(right) = right {
                    // resolve the element type
                    let left_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        left,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;

                    // require a literal length for array types
                    let value = self
                        .evaluate_integer_static_literal(
                            module, profile, right, tree, symbols, types,
                        )?
                        .ok_or_else(|| AnalyzeError::InvalidArraySize {
                            node: right
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        })?;
                    if value < 0 {
                        return Err(AnalyzeError::InvalidArraySize {
                            node: right
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
                    self.set_integer_literal_type(module.id, right, value, types);
                    Type::ArraySized {
                        element: left_id,
                        count: right,
                        is_readonly: false,
                    }
                }
                // slice
                else {
                    // resolve the element type
                    let left_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        left,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    Type::Array {
                        element: Some(left_id),
                        is_readonly: false,
                    }
                }
            }

            _ => return Ok(None),
        };

        // enforce implicit managed restrictions for type expressions
        if is_user_module
            && enforce_implicit_managed
            && options.no_implicit_managed
            && !self.expression_has_explicit_ownership(tree, expression_id)
            && self.type_is_implicit_managed(module, profile, &ty, types)
        {
            return Err(AnalyzeError::ImplicitManagedTypeDisabled {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
        }

        Ok(Some(ty))
    }

    /// Evaluate a typeof type expression into a Type.
    fn evaluate_typeof_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // unwrap parenthesized targets
        let mut target_id = right_id;
        loop {
            let Expression::Parenthesized { expression } = tree.get(target_id) else {
                break;
            };
            target_id = *expression;
        }

        // resolve the target symbol for a typeof reference
        let Some(target_symbol) = tree.get(target_id).target_symbol() else {
            return Ok(Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            });
        };

        // resolve the value type for the target symbol
        if let Some(value_ty_id) = types.get_value_type_id(target_symbol) {
            return Ok(types.get_type(value_ty_id).clone());
        }

        if target_symbol.module_id != module.id {
            let value_ty_id = self.resolve_remote_symbol_value_type(
                module,
                profile,
                expression_id.into_any(),
                target_symbol,
                false,
                types,
            )?;
            return Ok(types.get_type(value_ty_id).clone());
        }

        // fall back to local declaration types when available
        if let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(module, target_symbol, tree, symbols)
        {
            let declared_ty_id =
                types.get_declared_type_id(declarator_id.into_global_any(module.id));
            if let Some(declared_ty_id) = declared_ty_id {
                self.evaluate_type(module, profile, declared_ty_id, tree, symbols, types)?;
                return Ok(types.get_type(declared_ty_id).clone());
            }

            let declarator = tree.get(declarator_id);
            if let Some(value_id) = declarator.value {
                let ty = self.try_evaluate_expression_to_type_value(
                    module, profile, value_id, tree, symbols, types, true, true, true, true,
                )?;
                if !matches!(ty, Type::Unevaluated(_)) {
                    return Ok(ty);
                }
            }
        }

        Ok(Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        })
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{Expression, PrimitiveType, Type, TypeLiteral};

    use crate::TestProgram;

    #[test]
    fn test_analyze_evaluate_type_on_let_expression() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "declare let x: number");
        test.analyze_module(module_id);
        test.compile_check_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let types = dir.types.read();

        let let_expr_id = dir.roots[0];
        let expression = tree.get(let_expr_id);
        let &Expression::Statement {
            statement: let_expr_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let let_expression = tree.get(let_expr_id);
        let Expression::Let { declarators, .. } = let_expression else {
            panic!("expected let expression");
        };
        let declarator_id = declarators.first().unwrap();

        let let_ty = types
            .get_declared_type(declarator_id.into_global(module.id).into())
            .unwrap();

        assert_eq!(
            *let_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        );
    }

    #[test]
    fn test_analyze_evaluate_type_on_let_expression_int() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "declare let x: int");
        test.analyze_module(module_id);
        test.compile_check_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let types = dir.types.read();

        let let_expr_id = dir.roots[0];
        let expression = tree.get(let_expr_id);
        let &Expression::Statement {
            statement: let_expr_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let let_expression = tree.get(let_expr_id);
        let Expression::Let { declarators, .. } = let_expression else {
            panic!("expected let expression");
        };
        let declarator_id = declarators.first().unwrap();

        let let_ty = types
            .get_declared_type(declarator_id.into_global(module.id).into())
            .unwrap();

        // int resolves to Arbitrary { width: 32, is_signed: true } which is semantically Int32
        assert!(matches!(
            let_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Int(int_type))
            } if int_type.width() == Some(32) && int_type.is_signed()
        ));
    }
}
