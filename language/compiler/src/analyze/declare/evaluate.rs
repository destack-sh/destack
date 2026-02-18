use crate::analyze::common::{
    AnalyzeReadStage, AssociatedProjectionSelection, CanonicalSymbolMode, RelationMode,
    StaticMemberSymbolKind, TypeRewriteCache,
};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Assignability, Compiler};
use destack_dir::{
    Argument, BinaryOperator, BindingKind, Declaration, DependencyItem, DependencyMode, DynamicKey,
    EnumFieldValue, Expression, FunctionMode, FunctionSignature, GlobalSymbolId, IfCondition,
    IfKind, IntrinsicType, LocalNodeId, LocalNodeIdAny, LocalTypeId, Member, Mutability, NodeTree,
    NodeType, NodeVisitor, NodeVisitorOptions, NormalizationMode, Parameter, Path, PrimitiveType,
    Property, Resolution, ScalarLiteral, StaticArgument, StaticExpression, StaticKey,
    StaticParameterKind, StaticProperty, SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable,
    Type, TypeBinaryOperator, TypeElement, TypeField, TypeIndexSignature, TypeLiteral,
    TypeMappedParameter, TypeTable, TypeUnaryOperator, UnaryOperator, walk_expression,
};
use destack_workspace::{Module, ModuleSource, ProfileId};
use std::collections::{HashMap, HashSet};

use super::cache::EvaluateExpressionContext;

/// Selected member target for type evaluation.
#[derive(Clone, Debug)]
enum TypeMemberSelectionForTypeEvaluation {
    /// A namespace import member.
    Namespace {
        /// The selected member symbol.
        target_symbol: GlobalSymbolId,
    },
    /// An associated projection member.
    Associated(AssociatedProjectionSelection),
}

/// Classification for `TypeIndex` disambiguation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeIndexInterpretation {
    /// Interpret `T[K]` as indexed access.
    IndexAccess,
    /// Interpret `T[N]` as fixed-size array construction.
    ArraySized,
}

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

            let interpretation = match self.compiler.type_index_interpretation(
                self.module,
                self.profile,
                left_id,
                *index,
                tree,
                self.symbols,
                self.types,
            ) {
                Ok(value) => value,
                Err(error) => {
                    self.record_result(Err(error));
                    return;
                }
            };

            if interpretation == TypeIndexInterpretation::ArraySized {
                let is_array_size_candidate =
                    match self.compiler.expression_is_array_size_candidate(
                        self.module,
                        self.profile,
                        *index,
                        tree,
                        self.symbols,
                        self.types,
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            self.record_result(Err(error));
                            return;
                        }
                    };

                if is_array_size_candidate {
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
        // avoid eager static argument resolution for declaration modules
        let resolve_static_arguments = !module.language_type.is_declaration();
        let evaluated_ty = self.try_evaluate_expression_to_type_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            true,
            true,
            resolve_static_arguments,
            true,
        )?;
        types.update_type(ty_id, evaluated_ty);
        if !types.get_type(ty_id).is_unevaluated() {
            let cache_context =
                EvaluateExpressionContext::new(true, true, resolve_static_arguments);
            self.cache_expression_type_maybe(
                module.id,
                expression_id,
                cache_context,
                Some(ty_id),
                None,
                false,
                types,
            );
        }

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
        // build cache context for this evaluation
        let global_node_id = expression_id.into_global_any(module.id);
        let cache_context = EvaluateExpressionContext::new(
            validate_static_argument_bounds,
            enforce_implicit_managed,
            resolve_static_arguments,
        );
        let cache_key = cache_context.cache_key(global_node_id);
        let is_reference_expression = matches!(
            tree.get(expression_id),
            Expression::LocalReference { .. }
                | Expression::ModuleReference { .. }
                | Expression::GlobalReference { .. }
        );

        // reuse cached expression types when available
        if let Some(existing_id) = types.get_expression_type_id_cache(cache_key) {
            let ty = types.get_type(existing_id);
            if !ty.is_unevaluated() {
                return Ok(ty.clone());
            }
        }
        if let Some(existing) = types.get_expression_type_value_cache(cache_key) {
            return Ok(existing.clone());
        }

        // reuse cached declared types when requested
        if use_declared_cache
            && let Some(existing) = types.get_declared_type_id(global_node_id)
            && !types.get_type(existing).is_unevaluated()
        {
            // allow cached unknown references to resolve to their referenced symbols
            let is_unknown = types.get_type(existing).is_unknown();
            if !(is_unknown && is_reference_expression) {
                let ty = types.get_type(existing).clone();
                self.cache_expression_type_maybe(
                    module.id,
                    expression_id,
                    cache_context,
                    Some(existing),
                    Some(&ty),
                    is_reference_expression,
                    types,
                );
                return Ok(ty);
            }
        }

        // avoid recursive evaluation loops
        if types.is_expression_type_in_progress(global_node_id) {
            return Ok(Type::Unevaluated(expression_id));
        }
        types.mark_expression_type_in_progress(global_node_id);

        let result = {
            let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION);
            self.evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                validate_static_argument_bounds,
                enforce_implicit_managed,
                resolve_static_arguments,
            )
        };

        types.clear_expression_type_in_progress(global_node_id);

        // evaluate to a concrete type when possible
        let ty = result?.unwrap_or(Type::Unevaluated(expression_id));
        if !ty.is_unevaluated() {
            self.cache_expression_type_maybe(
                module.id,
                expression_id,
                cache_context,
                None,
                Some(&ty),
                is_reference_expression,
                types,
            );
        }
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
        // prefer existing type facts before re-evaluating the expression tree
        let expression_global = expression_id.into_global_any(module.id);
        if let Some(type_id) = types
            .get_inferred_type_id(expression_global)
            .or_else(|| types.get_declared_type_id(expression_global))
        {
            if let Some(value) = self.integer_literal_value_for_type_id(type_id, types) {
                return Ok(Some(value));
            }
        }

        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, tree);
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
            Some(StaticExpression::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
            }) => Some(value),
            Some(StaticExpression::Type { ty }) => match types.get_type(ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                } => Some(*value),
                _ => None,
            },
            _ => None,
        };
        Ok(literal)
    }

    /// Convert one substituted static parameter type into a static expression.
    fn static_expression_from_substitution_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> StaticExpression {
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value),
            } => StaticExpression::ScalarLiteral {
                value: value.clone(),
            },
            Type::TypeLiteral { value } => StaticExpression::TypeLiteral {
                value: value.clone(),
            },
            _ => StaticExpression::Type { ty: type_id },
        }
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

    /// Resolve one array-size count type id for an expression.
    fn array_sized_count_type_id_for_expression(
        &self,
        module_id: destack_source::ModuleId,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        if let Some(type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
        {
            return types.unwrap_value_type_id(type_id);
        }

        let unknown_type_id = types.insert_type_from(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            expression_id,
        );
        types.set_inferred_type(expression_id.into_global_any(module_id), unknown_type_id);
        unknown_type_id
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

        let (candidate_expression_id, is_explicit_comptime) =
            self.unwrap_as_comptime_expression(expression_id, tree);

        // skip expressions that are not static value references
        let kind = if let Some(kind) = self.static_parameter_reference_kind(
            module,
            profile,
            candidate_expression_id,
            tree,
            symbols,
            types,
        ) {
            kind
        } else {
            let index_ty_id = self.try_evaluate_expression_to_type(
                module,
                profile,
                candidate_expression_id,
                tree,
                symbols,
                types,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )?;
            let symbol = match types.get_type(index_ty_id) {
                Type::Reference { symbol, .. } => Some(*symbol),
                Type::Value { value } => match types.get_type(*value) {
                    Type::Reference { symbol, .. } => Some(*symbol),
                    _ => None,
                },
                _ => None,
            };
            let symbol = if symbol.is_none() {
                if let Expression::Member { left, name, .. } = tree.get(candidate_expression_id)
                    && matches!(tree.get(*left), Expression::This)
                    && let Some((owner_symbol, _)) =
                        self.owner_symbol_for_this_expression(module, profile, *left, tree, symbols)
                    && let Some(member_symbol) = self.resolve_static_member_symbol_in_tables(
                        module,
                        profile,
                        owner_symbol,
                        StaticKey::Name(*name),
                        tree,
                        symbols,
                    )
                    && matches!(
                        self.static_member_symbol_kind_for_symbol(
                            module,
                            profile,
                            member_symbol,
                            tree,
                            symbols,
                        ),
                        Some(StaticMemberSymbolKind::AssociatedComptimeConst)
                    )
                {
                    Some(member_symbol)
                } else {
                    None
                }
            } else {
                symbol
            };
            let Some(symbol) = symbol else {
                if is_explicit_comptime {
                    self.error(AnalyzeError::InvalidComptimeExpression {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
                return Ok(None);
            };
            let is_static_parameter =
                self.symbol_is_static_parameter(module, profile, symbol, symbols, types);
            let is_associated_comptime = matches!(
                self.static_member_symbol_kind_for_symbol(module, profile, symbol, tree, symbols),
                Some(StaticMemberSymbolKind::AssociatedComptimeConst)
            );
            if !is_static_parameter && !is_associated_comptime {
                if is_explicit_comptime {
                    self.error(AnalyzeError::InvalidComptimeExpression {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
                return Ok(None);
            }
            if is_associated_comptime {
                // keep associated comptime references as symbols so substitution can resolve counts
                let reference_type = Type::Reference {
                    symbol,
                    static_arguments: None,
                };
                let reference_type_id =
                    types.insert_type_from_any(reference_type, expression_id.into_any());
                types
                    .set_inferred_type(expression_id.into_global_any(module.id), reference_type_id);
                return Ok(Some(reference_type_id));
            }
            self.static_parameter_kind_for_symbol(module, profile, symbol, tree, symbols, types)
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
            candidate_expression_id,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;
        let inferred_id = if let Type::Value { value } = types.get_type(index_id) {
            *value
        } else {
            index_id
        };
        types.set_inferred_type(expression_id.into_global_any(module.id), inferred_id);
        Ok(Some(inferred_id))
    }

    /// Check whether an expression can be used as an array size candidate.
    fn expression_is_array_size_candidate(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<bool> {
        let (expression_id, is_explicit_comptime) =
            self.unwrap_as_comptime_expression(expression_id, tree);
        if is_explicit_comptime {
            return Ok(true);
        }

        if let Expression::Member { left, name, .. } = tree.get(expression_id)
            && matches!(tree.get(*left), Expression::This)
            && let Some((owner_symbol, _)) =
                self.owner_symbol_for_this_expression(module, profile, *left, tree, symbols)
            && let Some(member_symbol) = self.resolve_static_member_symbol_in_tables(
                module,
                profile,
                owner_symbol,
                StaticKey::Name(*name),
                tree,
                symbols,
            )
            && matches!(
                self.static_member_symbol_kind_for_symbol(
                    module,
                    profile,
                    member_symbol,
                    tree,
                    symbols,
                ),
                Some(StaticMemberSymbolKind::AssociatedComptimeConst)
            )
        {
            return Ok(true);
        }

        let target_symbol = if let Some(target_symbol) = tree.get(expression_id).target_symbol() {
            Some(target_symbol)
        } else {
            let index_type_id = self.try_evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                true,
                true,
            )?;
            match types.get_type(index_type_id) {
                Type::Reference { symbol, .. } => Some(*symbol),
                Type::Value { value } => match types.get_type(*value) {
                    Type::Reference { symbol, .. } => Some(*symbol),
                    _ => None,
                },
                _ => None,
            }
        };

        let Some(target_symbol) = target_symbol else {
            return Ok(false);
        };

        if self.symbol_is_static_parameter(module, profile, target_symbol, symbols, types) {
            let kind = self.static_parameter_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
                types,
            );
            return Ok(matches!(kind, StaticParameterKind::Value));
        }

        let is_associated_comptime = matches!(
            self.static_member_symbol_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
            ),
            Some(StaticMemberSymbolKind::AssociatedComptimeConst)
        );

        Ok(is_associated_comptime)
    }

    /// Classify one `TypeIndex` expression as index-access or fixed-array construction.
    pub(crate) fn type_index_interpretation(
        &self,
        module: &Module,
        profile: ProfileId,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<TypeIndexInterpretation> {
        let (_, is_explicit_comptime) =
            self.unwrap_as_comptime_expression(index_expression_id, tree);
        if module.language_type.is_declaration() {
            return Ok(if is_explicit_comptime {
                TypeIndexInterpretation::ArraySized
            } else {
                TypeIndexInterpretation::IndexAccess
            });
        }

        let index_is_array_size_candidate = self.expression_is_array_size_candidate(
            module,
            profile,
            index_expression_id,
            tree,
            symbols,
            types,
        )?;
        let left_is_array_sized = matches!(types.get_type(left_type_id), Type::ArraySized { .. });
        let supports_index_access = self.type_supports_index_access(
            module,
            profile,
            left_type_id,
            tree,
            symbols,
            types,
            true,
        )?;
        let is_primitive_literal = self.type_is_primitive_literal(left_type_id, types);
        let force_array_size_from_value_candidate = index_is_array_size_candidate;
        let force_array_size_from_nested_sized =
            left_is_array_sized && self.type_index_is_integer_literal(index_expression_id, tree);
        let force_array_size =
            force_array_size_from_value_candidate || force_array_size_from_nested_sized;

        let is_index_access = !is_explicit_comptime
            && supports_index_access
            && !is_primitive_literal
            && !force_array_size;

        Ok(if is_index_access {
            TypeIndexInterpretation::IndexAccess
        } else {
            TypeIndexInterpretation::ArraySized
        })
    }

    /// Decide whether one `TypeIndex` expression should use index-access semantics.
    pub(crate) fn type_index_uses_index_access(
        &self,
        module: &Module,
        profile: ProfileId,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<bool> {
        Ok(self.type_index_interpretation(
            module,
            profile,
            left_type_id,
            index_expression_id,
            tree,
            symbols,
            types,
        )? == TypeIndexInterpretation::IndexAccess)
    }

    /// Check whether a type-index expression is a plain integer literal.
    fn type_index_is_integer_literal(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, tree);
        let expression_id = self.unwrap_parenthesized_expression(expression_id, tree);

        if matches!(
            tree.get(expression_id),
            Expression::ScalarLiteral {
                value: ScalarLiteral::Integer(_),
            }
        ) {
            return true;
        }

        if let Expression::Unary {
            operator: UnaryOperator::Negate,
            right,
        } = tree.get(expression_id)
        {
            return matches!(
                tree.get(*right),
                Expression::ScalarLiteral {
                    value: ScalarLiteral::Integer(_),
                }
            );
        }

        false
    }

    /// Unwrap parenthesized expressions and explicit `as comptime` markers.
    fn unwrap_as_comptime_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> (LocalNodeId<Expression>, bool) {
        let mut expression_id = self.unwrap_parenthesized_expression(expression_id, tree);
        let mut is_explicit_comptime = false;

        while let Expression::TypeUnary {
            operator: TypeUnaryOperator::AsComptime,
            right,
        } = tree.get(expression_id)
        {
            is_explicit_comptime = true;
            expression_id = self.unwrap_parenthesized_expression(*right, tree);
        }

        (expression_id, is_explicit_comptime)
    }

    /// Check whether a type supports indexed access in a type expression.
    pub(crate) fn type_supports_index_access(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        treat_unknown_as_indexable: bool,
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
                    // keep unknown or any indexability configurable per disambiguation context
                    return Ok(treat_unknown_as_indexable);
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
                    if self.symbol_is_static_parameter(module, profile, symbol, symbols, types) {
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

                        // treat unconstrained static parameters as unknown when requested
                        return Ok(treat_unknown_as_indexable);
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
    pub(crate) fn type_is_primitive_literal(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
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

    /// Resolve the static parameter symbol and kind for a reference expression.
    fn static_parameter_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<(GlobalSymbolId, StaticParameterKind)> {
        // only treat references as static parameters in Destack modules
        if !module.language_type.is_destack() {
            return None;
        }

        // unwrap explicit comptime wrappers to reach the reference
        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, tree);

        // resolve the referenced symbol first
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tree.get(expression_id)
        else {
            return None;
        };

        self.with_module_tree_symbols_or_local(
            module,
            profile,
            target_symbol.module_id,
            tree,
            symbols,
            |owner_module, owner_tree, owner_symbols| {
                self.static_parameter_reference_in_symbols(
                    owner_module,
                    profile,
                    *target_symbol,
                    owner_tree,
                    owner_symbols,
                    types,
                )
            },
        )
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
        self.static_parameter_reference(module, profile, expression_id, tree, symbols, types)
            .map(|(_, kind)| kind)
    }

    /// Resolve the static parameter symbol and kind for a symbol within a symbol table.
    fn static_parameter_reference_in_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<(GlobalSymbolId, StaticParameterKind)> {
        // resolve direct static parameter references
        if self.symbol_is_static_parameter(module, profile, target_symbol, symbols, types) {
            let kind = self.static_parameter_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
                types,
            );
            return Some((target_symbol, kind));
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
                let kind = self.static_parameter_kind_for_symbol(
                    module,
                    profile,
                    candidate_global,
                    tree,
                    symbols,
                    types,
                );
                return Some((candidate_global, kind));
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
        let cache_context = EvaluateExpressionContext::new(
            validate_static_argument_bounds,
            enforce_implicit_managed,
            true,
        );
        let cache_key = cache_context.cache_key(global_node_id);
        let is_reference_expression = matches!(
            tree.get(expression_id),
            Expression::LocalReference { .. }
                | Expression::ModuleReference { .. }
                | Expression::GlobalReference { .. }
        );
        let mut cached_type_id = None;
        if let Some(existing) = types.get_expression_type_id_cache(cache_key)
            && !types.get_type(existing).is_unevaluated()
        {
            cached_type_id = Some(existing);
        } else if let Some(existing) = types.get_declared_type_id(global_node_id) {
            if types.get_type(existing).is_unevaluated() {
                // skip unevaluated declared entries and re-evaluate
            } else {
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
                    let is_unknown = types.get_type(existing).is_unknown();
                    if !(is_unknown && is_reference_expression) {
                        let ty = types.get_type(existing).clone();
                        self.cache_expression_type_maybe(
                            module.id,
                            expression_id,
                            cache_context,
                            Some(existing),
                            Some(&ty),
                            is_reference_expression,
                            types,
                        );
                        cached_type_id = Some(existing);
                    }
                }
            }
        }
        if let Some(existing) = cached_type_id {
            // materialize the type value so downstream phases do not see an untyped expression
            let type_value = Type::Value { value: existing };
            let type_value_id = types.insert_type_from(type_value, expression_id);
            types.set_inferred_type(expression_id.into_global_any(module.id), type_value_id);
            return Ok(existing);
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
        let has_static_arguments = matches!(
            &ty,
            Type::Reference {
                static_arguments: Some(arguments),
                ..
            } if !arguments.is_empty()
        );
        let should_cache =
            !ty.is_unevaluated() && (validate_static_argument_bounds || !has_static_arguments);
        if should_cache {
            types.set_declared_type(global_node_id, ty_id);
        }
        self.cache_expression_type_maybe(
            module.id,
            expression_id,
            cache_context,
            Some(ty_id),
            Some(&ty),
            is_reference_expression,
            types,
        );

        // materialize the type value so downstream phases do not see an untyped expression
        let type_value = Type::Value { value: ty_id };
        let type_value_id = types.insert_type_from(type_value, expression_id);
        types.set_inferred_type(expression_id.into_global_any(module.id), type_value_id);

        Ok(ty_id)
    }

    /// Re-evaluate an expression as a type, bypassing declared/expression caches.
    pub(crate) fn reevaluate_expression_to_type(
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
        let ty = self
            .evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                validate_static_argument_bounds,
                enforce_implicit_managed,
                true,
            )?
            .unwrap_or(Type::Unevaluated(expression_id));
        let ty_id = types.insert_type_from(ty, expression_id);
        let global_node_id = expression_id.into_global_any(module.id);
        types.set_declared_type(global_node_id, ty_id);

        let type_value_id = types.insert_type_from(Type::Value { value: ty_id }, expression_id);
        types.set_inferred_type(global_node_id, type_value_id);

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
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<Type> {
        let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_SIGNATURE);

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
            if !defer_type_evaluation {
                self.evaluate_type(module, profile, declared_type_id, tree, symbols, types)?;
            }

            // expand tuple rest parameters into positional call parameters
            if matches!(
                tree.get(*parameter_id),
                Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
            ) && let Type::Tuple { elements, .. } = types.get_type(declared_type_id)
            {
                for element in elements {
                    dynamic_parameters.push(element.ty);
                }
                continue;
            }

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
            if !defer_type_evaluation {
                self.evaluate_type(module, profile, declared_type_id, tree, symbols, types)?;
            }

            Some(declared_type_id)
        } else {
            None
        };

        // evaluate return type
        let return_type = if let Some(return_type_id) = signature.return_type {
            if defer_type_evaluation {
                Some(self.declared_type_id_for_expression_or_insert(module, return_type_id, types))
            } else {
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
            }
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

    /// Return a declared type id for an expression, inserting an unevaluated type if needed.
    fn declared_type_id_for_expression_or_insert(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let global_id = expression_id.into_global_any(module.id);
        if let Some(existing) = types.get_declared_type_id(global_id) {
            return existing;
        }

        let ty_id = types.insert_type_from(Type::Unevaluated(expression_id), expression_id);
        types.set_declared_type(global_id, ty_id);
        ty_id
    }

    /// Evaluate static arguments for a type reference.
    pub(crate) fn evaluate_static_arguments(
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
        let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_TEMPLATE);

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
            let mut best_preferred = None;

            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().rev() {
                if *candidate_key != key {
                    continue;
                }
                let candidate = symbols.get_symbol(*candidate_symbol_id);
                if !candidate.is_active {
                    continue;
                }
                let candidate_symbol = candidate_symbol_id.into_global(module.id);

                // type-value symbols satisfy all space orders
                let candidate_space = candidate.space;
                if candidate_space == SymbolSpace::TypeValue {
                    return Some(candidate_symbol);
                }

                // track the best preferred-space candidate for this scope
                if let Some(space_index) = preferred_spaces
                    .iter()
                    .position(|preferred_space| *preferred_space == candidate_space)
                {
                    let replace_best = match best_preferred {
                        Some((best_index, _)) => space_index < best_index,
                        None => true,
                    };
                    if replace_best {
                        best_preferred = Some((space_index, candidate_symbol));
                    }
                    continue;
                }

                // keep the nearest fallback only when no preferred symbol exists anywhere
                if fallback.is_none() {
                    fallback = Some(candidate_symbol);
                }
            }

            // prefer this scope's best matching symbol space before checking parent scopes
            if let Some((_, preferred_symbol)) = best_preferred {
                return Some(preferred_symbol);
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
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_EVALUATE);

        let mut visited = HashSet::new();
        self.evaluate_static_expression_value_inner(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            enum_symbol,
            None,
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
        substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
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
                    substitutions,
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
                    substitutions,
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
                    substitutions,
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
                    substitutions,
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
                    substitutions,
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
                    substitutions,
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
                if let Some((parameter_symbol, kind)) = self.static_parameter_reference(
                    module,
                    profile,
                    expression_id,
                    tree,
                    symbols,
                    types,
                ) {
                    if kind == StaticParameterKind::Value {
                        // use substitution values when available
                        if let Some(substitutions) = substitutions
                            && let Some(type_id) = substitutions.get(&parameter_symbol)
                        {
                            let value =
                                self.static_expression_from_substitution_type(*type_id, types);
                            return Ok(Some(value));
                        }

                        let reference_type = Type::Reference {
                            symbol: parameter_symbol,
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
                    substitutions,
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
                )?;
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
                left,
                name,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Ok(None);
                }
                let node_id = expression_id.into_global_any(module.id);
                let member_key = StaticKey::Name(*name);

                // resolve projected members in value space for static expressions
                if let Some(selection) = self.select_associated_projection_member_symbol(
                    module,
                    profile,
                    expression_id,
                    *left,
                    member_key,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )? {
                    let options = self.analyze_context_options_for_module(module.id);
                    let substitutions = self.associated_projection_substitutions_for_member(
                        module,
                        profile,
                        expression_id.into_any(),
                        selection.target_symbol,
                        Some(selection.receiver_symbol),
                        &selection.receiver_arguments,
                        &options,
                        Some(visited),
                        tree,
                        symbols,
                        types,
                    )?;

                    if let Some(value) = self.static_expression_from_constant_reference(
                        module,
                        profile,
                        selection.target_symbol,
                        tree,
                        symbols,
                        types,
                        Some(&substitutions),
                        visited,
                    )? {
                        return Ok(Some(value));
                    }
                }

                // fall back to resolved static candidates
                if let Some(resolution_id) = types.get_resolution_for_node(node_id) {
                    let resolution = types.get_resolution(resolution_id);
                    if let Resolution::Static { candidate, .. } = resolution
                        && let Some(value) = self.static_expression_from_constant_reference(
                            module,
                            profile,
                            candidate.target_symbol,
                            tree,
                            symbols,
                            types,
                            substitutions,
                            visited,
                        )?
                    {
                        return Ok(Some(value));
                    }
                }

                // keep enum member literals available in static contexts
                let Some(enum_symbol) = enum_symbol else {
                    return Ok(None);
                };
                let Some(target_symbol) = self.enum_field_symbol_for_name(
                    module,
                    profile,
                    enum_symbol,
                    *name,
                    tree,
                    symbols,
                )?
                else {
                    return Ok(None);
                };
                let Some(value) = self.enum_field_value_for_symbol_reference(
                    module,
                    profile,
                    enum_symbol,
                    target_symbol,
                    symbols,
                    types,
                )?
                else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: match value {
                        EnumFieldValue::Int(value) => ScalarLiteral::Integer(value),
                        EnumFieldValue::String(value) => ScalarLiteral::String(value),
                    },
                }
            }
            Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } if *kind == IfKind::Ternary => {
                let Some(else_expression) = else_expression else {
                    return Ok(None);
                };
                let IfCondition::Expression {
                    condition: condition_expression,
                } = condition
                else {
                    return Ok(None);
                };

                let condition_holds = match tree.get(*condition_expression) {
                    Expression::TypeBinary {
                        left,
                        operator: TypeBinaryOperator::Extends,
                        right,
                    } => {
                        // resolve both sides for extends checks
                        let mut evaluate_side =
                            |side_id: LocalNodeId<Expression>| -> AnalyzeResult<LocalTypeId> {
                                if let Some((parameter_symbol, _)) = self
                                    .static_parameter_reference(
                                        module, profile, side_id, tree, symbols, types,
                                    )
                                    && let Some(substitutions) = substitutions
                                    && let Some(mapped) = substitutions.get(&parameter_symbol)
                                {
                                    return Ok(*mapped);
                                }

                                self.try_evaluate_expression_to_type(
                                    module, profile, side_id, tree, symbols, types, true, true,
                                )
                            };
                        let mut left_type_id = evaluate_side(*left)?;
                        let mut right_type_id = evaluate_side(*right)?;

                        if let Some(substitutions) = substitutions
                            && !substitutions.is_empty()
                        {
                            let mut substitution_cache = HashMap::new();
                            left_type_id = self.substitute_static_parameters(
                                left_type_id,
                                substitutions,
                                types,
                                &mut substitution_cache,
                            );
                            right_type_id = self.substitute_static_parameters(
                                right_type_id,
                                substitutions,
                                types,
                                &mut substitution_cache,
                            );
                        }

                        let mut materialize_cache = TypeRewriteCache::new();
                        left_type_id = self.materialize_static_arguments_in_type(
                            module,
                            profile,
                            left_type_id,
                            tree,
                            symbols,
                            types,
                            &mut materialize_cache,
                        );
                        right_type_id = self.materialize_static_arguments_in_type(
                            module,
                            profile,
                            right_type_id,
                            tree,
                            symbols,
                            types,
                            &mut materialize_cache,
                        );
                        left_type_id = self.normalize_type_with_relation(
                            module,
                            profile,
                            left_type_id,
                            symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::TYPE_OPS,
                        );
                        right_type_id = self.normalize_type_with_relation(
                            module,
                            profile,
                            right_type_id,
                            symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::TYPE_OPS,
                        );

                        let options = self.analyze_context_options_for_module(module.id);
                        self.is_type_assignable(
                            module,
                            profile,
                            symbols,
                            right_type_id,
                            left_type_id,
                            types,
                            &options,
                        ) != Assignability::NotAssignable
                    }
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::Boolean(value),
                    } => *value,
                    _ => return Ok(None),
                };

                let selected = if condition_holds {
                    *then_expression
                } else {
                    *else_expression
                };
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    selected,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    substitutions,
                    visited,
                );
            }
            Expression::TypeConditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                // evaluate both sides as types before selecting one branch
                let mut evaluate_side =
                    |side_id: LocalNodeId<Expression>| -> AnalyzeResult<LocalTypeId> {
                        // prefer caller substitutions for static parameters
                        if let Some((parameter_symbol, _)) = self.static_parameter_reference(
                            module, profile, side_id, tree, symbols, types,
                        ) && let Some(substitutions) = substitutions
                            && let Some(mapped) = substitutions.get(&parameter_symbol)
                        {
                            return Ok(*mapped);
                        }

                        self.try_evaluate_expression_to_type(
                            module, profile, side_id, tree, symbols, types, true, true,
                        )
                    };
                let mut left_type_id = evaluate_side(*left)?;
                let mut right_type_id = evaluate_side(*right)?;

                // apply caller substitutions before relation checks
                if let Some(substitutions) = substitutions
                    && !substitutions.is_empty()
                {
                    let mut substitution_cache = HashMap::new();
                    left_type_id = self.substitute_static_parameters(
                        left_type_id,
                        substitutions,
                        types,
                        &mut substitution_cache,
                    );
                    right_type_id = self.substitute_static_parameters(
                        right_type_id,
                        substitutions,
                        types,
                        &mut substitution_cache,
                    );
                }

                // materialize and normalize both sides in type-op relation mode
                let mut materialize_cache = TypeRewriteCache::new();
                left_type_id = self.materialize_static_arguments_in_type(
                    module,
                    profile,
                    left_type_id,
                    tree,
                    symbols,
                    types,
                    &mut materialize_cache,
                );
                right_type_id = self.materialize_static_arguments_in_type(
                    module,
                    profile,
                    right_type_id,
                    tree,
                    symbols,
                    types,
                    &mut materialize_cache,
                );
                left_type_id = self.normalize_type_with_relation(
                    module,
                    profile,
                    left_type_id,
                    symbols,
                    types,
                    NormalizationMode::Assign,
                    RelationMode::TYPE_OPS,
                );
                right_type_id = self.normalize_type_with_relation(
                    module,
                    profile,
                    right_type_id,
                    symbols,
                    types,
                    NormalizationMode::Assign,
                    RelationMode::TYPE_OPS,
                );

                // choose the branch using extends assignability semantics
                let options = self.analyze_context_options_for_module(module.id);
                let is_assignable = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    right_type_id,
                    left_type_id,
                    types,
                    &options,
                );
                let selected = if is_assignable == Assignability::NotAssignable {
                    *else_type
                } else {
                    *then_type
                };

                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    selected,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    substitutions,
                    visited,
                );
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
                        substitutions,
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
                        substitutions,
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
                                substitutions,
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
                                    substitutions,
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
    pub(crate) fn static_expression_from_constant_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        // evaluate using the owning module context
        if symbol.module_id != module.id {
            return self
                .with_module_tree_symbols_for_stage(
                    module,
                    profile,
                    symbol.module_id,
                    AnalyzeReadStage::Infer,
                    |owner_module, owner_tree, owner_symbols| {
                        let mut owner_types = owner_module.dir(profile).types.write();
                        let mapped_substitutions = substitutions.map(|substitutions| {
                            let mut mapped = HashMap::new();
                            for (substitution_symbol, substitution_type_id) in substitutions {
                                let substitution_ty = types.get_type(*substitution_type_id);
                                let substitution_source =
                                    types.get_type_source(*substitution_type_id);
                                let mapped_type_id = self.import_type_from_remote_for_node(
                                    substitution_source,
                                    substitution_ty,
                                    types,
                                    *substitution_symbol,
                                    &mut owner_types,
                                );
                                mapped.insert(*substitution_symbol, mapped_type_id);
                            }
                            mapped
                        });
                        self.static_expression_from_constant_reference(
                            owner_module,
                            profile,
                            symbol,
                            owner_tree,
                            owner_symbols,
                            &mut owner_types,
                            mapped_substitutions.as_ref(),
                            visited,
                        )
                    },
                )
                .map_err(AnalyzeError::from)?;
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
                module,
                profile,
                value_id,
                tree,
                symbols,
                types,
                None,
                substitutions,
                visited,
            )?
        } else {
            let symbol_entry = symbols.get_symbol(symbol.local_id);

            // evaluate associated comptime member initializers
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::Member
            {
                let member_id = primary_declaration.local_id.into_typed::<Member>();
                if let Member::ComptimeConst {
                    value: Some(value_expression_id),
                    ..
                } = tree.get(member_id)
                {
                    // evaluate literal/static forms directly first
                    let value = self.evaluate_static_expression_value_inner(
                        module,
                        profile,
                        *value_expression_id,
                        tree,
                        symbols,
                        types,
                        None,
                        substitutions,
                        visited,
                    )?;
                    let Some(value) = value else {
                        // evaluate type-level forms through substitution and normalization
                        let mut value_type_id = self.try_evaluate_expression_to_type(
                            module,
                            profile,
                            *value_expression_id,
                            tree,
                            symbols,
                            types,
                            true,
                            true,
                        )?;

                        if let Some(substitutions) = substitutions
                            && !substitutions.is_empty()
                        {
                            let mut substitution_cache = HashMap::new();
                            value_type_id = self.substitute_static_parameters(
                                value_type_id,
                                substitutions,
                                types,
                                &mut substitution_cache,
                            );
                        }

                        let mut materialize_cache = TypeRewriteCache::new();
                        value_type_id = self.materialize_static_arguments_in_type(
                            module,
                            profile,
                            value_type_id,
                            tree,
                            symbols,
                            types,
                            &mut materialize_cache,
                        );

                        value_type_id = self.normalize_type_with_relation(
                            module,
                            profile,
                            value_type_id,
                            symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::TYPE_OPS,
                        );

                        let projected_value = match types.get_type(value_type_id) {
                            Type::TypeLiteral {
                                value: TypeLiteral::ScalarLiteral(value),
                            } => Some(StaticExpression::ScalarLiteral {
                                value: value.clone(),
                            }),
                            Type::TypeLiteral { value } => Some(StaticExpression::TypeLiteral {
                                value: value.clone(),
                            }),
                            _ => None,
                        };

                        visited.remove(&symbol);
                        return Ok(projected_value);
                    };
                    visited.remove(&symbol);
                    return Ok(Some(value));
                }
            }

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
                    substitutions,
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
                                        substitutions,
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
                    substitutions,
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
                    substitutions,
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

    /// Select a namespace-imported member for type evaluation.
    fn select_namespace_import_member_symbol_for_type_evaluation(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // require a reference expression on the left side
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tree.get(left)
        else {
            return None;
        };

        // namespace imports are local dependency items
        if target_symbol.module_id != module.id {
            return None;
        }

        // require a dependency declaration
        let symbol_entry = symbols.get_symbol(target_symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        if primary_declaration.local_id.ty != NodeType::DependencyItem {
            return None;
        }

        // select dependency mode and target module
        let dependency_id = primary_declaration.local_id.into_typed::<DependencyItem>();
        let dependency = tree.get(dependency_id);
        let (mode, target_module) = match dependency {
            DependencyItem::Remote {
                mode,
                target_module,
                ..
            } => (*mode, Some(*target_module)),
            DependencyItem::UnresolvedRemote {
                mode,
                target_module,
                ..
            } => (*mode, *target_module),
            _ => (DependencyMode::Item, None),
        };

        // only namespace imports can project members
        if mode != DependencyMode::Namespace {
            return None;
        }

        // select type space first, then value space
        let target_module = target_module?;
        let target_module = target_module.ty.or(target_module.value)?;
        self.resolve_export_symbol_for_target(
            module.id,
            expression_id.into_global_any(module.id),
            target_module,
            profile,
            SymbolSpaceOrder::TypeThenValue,
            member_key,
        )
        .ok()
        .flatten()
    }

    /// Select a type member symbol for one member expression.
    fn select_type_member_symbol_for_type_evaluation(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<TypeMemberSelectionForTypeEvaluation>> {
        // select namespace imports before projection lookups
        if let Some(namespace_symbol) = self
            .select_namespace_import_member_symbol_for_type_evaluation(
                module,
                profile,
                expression_id,
                left,
                member_key,
                tree,
                symbols,
            )
        {
            return Ok(Some(TypeMemberSelectionForTypeEvaluation::Namespace {
                target_symbol: namespace_symbol,
            }));
        }

        // select projected members through nominal receivers
        let Some(selection) = self.select_associated_projection_member_symbol(
            module,
            profile,
            expression_id,
            left,
            member_key,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?
        else {
            if let Some(enum_member_symbol) = self.select_enum_member_symbol_for_type_evaluation(
                module, profile, left, member_key, tree, symbols,
            )? {
                return Ok(Some(TypeMemberSelectionForTypeEvaluation::Namespace {
                    target_symbol: enum_member_symbol,
                }));
            }
            if let Some(projected_symbol) = tree.get(expression_id).target_symbol() {
                let is_enum_field = self
                    .with_module_tree_symbols_or_local_for_stage(
                        module,
                        profile,
                        projected_symbol.module_id,
                        tree,
                        symbols,
                        AnalyzeReadStage::Declare,
                        |_owner_module, _owner_tree, owner_symbols| {
                            let symbol_entry = owner_symbols.get_symbol(projected_symbol.local_id);
                            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                                return false;
                            };
                            primary_declaration.local_id.ty == NodeType::EnumField
                        },
                    )
                    .map_err(AnalyzeError::from)?;
                if is_enum_field {
                    return Ok(Some(TypeMemberSelectionForTypeEvaluation::Namespace {
                        target_symbol: projected_symbol,
                    }));
                }
            }

            return Ok(None);
        };

        Ok(Some(TypeMemberSelectionForTypeEvaluation::Associated(
            selection,
        )))
    }

    /// Select one enum member symbol for a type-position member expression.
    fn select_enum_member_symbol_for_type_evaluation(
        &self,
        module: &Module,
        profile: ProfileId,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let Some(left_target_symbol) = tree.get(left).target_symbol() else {
            return Ok(None);
        };
        let left_target_symbol = self.resolve_type_reference_symbol_for_evaluation(
            module,
            profile,
            left_target_symbol,
            tree,
            symbols,
        );

        let projected_symbol = self
            .with_module_tree_symbols_or_local_for_stage(
                module,
                profile,
                left_target_symbol.module_id,
                tree,
                symbols,
                AnalyzeReadStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    self.resolve_static_member_symbol_in_tables(
                        owner_module,
                        profile,
                        left_target_symbol,
                        member_key,
                        owner_tree,
                        owner_symbols,
                    )
                },
            )
            .map_err(AnalyzeError::from)?;
        let Some(projected_symbol) = projected_symbol else {
            return Ok(None);
        };

        let is_enum_field = self
            .with_module_tree_symbols_or_local_for_stage(
                module,
                profile,
                projected_symbol.module_id,
                tree,
                symbols,
                AnalyzeReadStage::Declare,
                |_owner_module, _owner_tree, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(projected_symbol.local_id);
                    let Some(primary_declaration) = symbol_entry.primary_declaration else {
                        return false;
                    };

                    primary_declaration.local_id.ty == NodeType::EnumField
                },
            )
            .map_err(AnalyzeError::from)?;
        if !is_enum_field {
            return Ok(None);
        }

        Ok(Some(projected_symbol))
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
        let module_checks = self.module_check_options_for_module(module.id);
        let is_user_module = matches!(module.source, ModuleSource::User);
        let defer_reference_resolution = module.language_type.is_declaration()
            && (module_checks.skip_lib_check || matches!(module.source, ModuleSource::Builtin(_)));

        let expression = tree.get(expression_id).clone();
        let ty = match expression {
            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE);
                let member_key = StaticKey::Name(name);
                let Some(selection) = self.select_type_member_symbol_for_type_evaluation(
                    module,
                    profile,
                    expression_id,
                    left,
                    member_key,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?
                else {
                    if is_user_module {
                        // evaluate the receiver type for a precise missing member diagnostic
                        let receiver_ty_id = self.try_evaluate_expression_to_type(
                            module,
                            profile,
                            left,
                            tree,
                            symbols,
                            types,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )?;
                        self.error(AnalyzeError::MissingMember {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                            receiver_ty: receiver_ty_id.into_global(module.id),
                            member_key,
                        });
                        return Ok(Some(Type::Error));
                    }

                    return Ok(None);
                };
                let (target_symbol, receiver_symbol, receiver_arguments) = match selection {
                    TypeMemberSelectionForTypeEvaluation::Namespace { target_symbol } => {
                        (target_symbol, None, Vec::new())
                    }
                    TypeMemberSelectionForTypeEvaluation::Associated(
                        AssociatedProjectionSelection {
                            target_symbol,
                            receiver_symbol,
                            receiver_arguments,
                        },
                    ) => (target_symbol, Some(receiver_symbol), receiver_arguments),
                };

                // require explicit arguments for generic associated type projections
                let has_explicit_static_arguments = static_arguments
                    .as_ref()
                    .is_some_and(|arguments| !arguments.is_empty());
                if !has_explicit_static_arguments
                    && self.associated_type_requires_static_arguments(
                        module,
                        profile,
                        target_symbol,
                        tree,
                        symbols,
                    )
                {
                    let node = expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile));
                    self.error(AnalyzeError::MissingStaticArgument { node });
                    return Ok(Some(Type::Error));
                }

                // evaluate static arguments for the referenced symbol
                let static_arguments = self.evaluate_static_arguments(
                    module,
                    profile,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                )?;
                let resolve_static_arguments =
                    resolve_static_arguments && !defer_reference_resolution;
                let member_ty = self.evaluate_type_reference_for_symbol(
                    module,
                    profile,
                    expression_id,
                    target_symbol,
                    static_arguments.clone(),
                    resolve_static_arguments,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                    tree,
                    symbols,
                    types,
                )?;

                self.materialize_associated_member_projection(
                    module,
                    profile,
                    expression_id.into_any(),
                    target_symbol,
                    receiver_symbol,
                    &receiver_arguments,
                    static_arguments.as_deref(),
                    member_ty,
                    tree,
                    symbols,
                    types,
                )?
            }
            Expression::Instantiation {
                left,
                static_arguments,
            } => {
                // evaluate the left side to a type reference before applying instantiation arguments
                let receiver_type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;

                // extract a nominal receiver symbol from direct references and merge intersections
                let receiver_symbol = self
                    .unwrap_type_symbol(types, receiver_type_id)
                    .map(|(symbol, _, _)| symbol)
                    .or_else(|| match types.get_type(receiver_type_id).clone() {
                        Type::Intersection { elements } | Type::Union { elements } => {
                            elements.iter().find_map(|element_id| {
                                self.unwrap_type_symbol(types, *element_id)
                                    .map(|(symbol, _, _)| symbol)
                            })
                        }
                        _ => None,
                    });
                let Some(target_symbol) = receiver_symbol else {
                    return Ok(None);
                };
                let target_symbol = self.resolve_type_reference_symbol_for_evaluation(
                    module,
                    profile,
                    target_symbol,
                    tree,
                    symbols,
                );

                // evaluate explicit instantiation static arguments
                let static_arguments = self.evaluate_static_arguments(
                    module,
                    profile,
                    Some(static_arguments.as_slice()),
                    tree,
                    symbols,
                    types,
                )?;
                let resolve_static_arguments =
                    resolve_static_arguments && !defer_reference_resolution;

                self.evaluate_type_reference_for_symbol(
                    module,
                    profile,
                    expression_id,
                    target_symbol,
                    static_arguments,
                    resolve_static_arguments,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                    tree,
                    symbols,
                    types,
                )?
            }
            Expression::LocalReference {
                target_symbol,
                static_arguments,
                path: _,
                ..
            }
            | Expression::ModuleReference {
                target_symbol,
                static_arguments,
                path: _,
                ..
            }
            | Expression::GlobalReference {
                target_symbol,
                static_arguments,
                path: _,
                ..
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE);
                let target_symbol = if let Some((parameter_symbol, _)) = self
                    .static_parameter_reference(
                        module,
                        profile,
                        expression_id,
                        tree,
                        symbols,
                        types,
                    ) {
                    parameter_symbol
                } else {
                    self.resolve_type_reference_symbol_for_evaluation(
                        module,
                        profile,
                        target_symbol,
                        tree,
                        symbols,
                    )
                };

                let static_arguments = {
                    let _timing =
                        self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE_ARGUMENTS);
                    self.evaluate_static_arguments(
                        module,
                        profile,
                        static_arguments.as_deref(),
                        tree,
                        symbols,
                        types,
                    )?
                };
                let resolve_static_arguments =
                    resolve_static_arguments && !defer_reference_resolution;
                self.evaluate_type_reference_for_symbol(
                    module,
                    profile,
                    expression_id,
                    target_symbol,
                    static_arguments,
                    resolve_static_arguments,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                    tree,
                    symbols,
                    types,
                )?
            }
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
            Expression::This | Expression::Super => Type::This,
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
                        false,
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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

                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_CONDITIONAL);

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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_MAPPED);

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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_INDEX);

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

                // disambiguate between type indexing and fixed-size arrays
                let interpretation = self.type_index_interpretation(
                    module, profile, left_id, index, tree, symbols, types,
                )?;

                // compute the type index result
                if interpretation == TypeIndexInterpretation::ArraySized {
                    let integer_array_size = self.evaluate_integer_static_literal(
                        module, profile, index, tree, symbols, types,
                    )?;

                    // treat static integer literals as array sizes
                    if let Some(value) = integer_array_size {
                        if value < 0 {
                            return Err(AnalyzeError::InvalidArraySize {
                                node: index
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            });
                        }
                        self.set_integer_literal_type(module.id, index, value, types);
                        let count_type_id =
                            self.array_sized_count_type_id_for_expression(module.id, index, types);
                        Type::ArraySized {
                            element: left_id,
                            count: count_type_id,
                            is_readonly: false,
                        }
                    } else if self.expression_is_array_size_candidate(
                        module, profile, index, tree, symbols, types,
                    )? {
                        let count_type_id = self
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
                            .unwrap_or_else(|| {
                                self.array_sized_count_type_id_for_expression(
                                    module.id, index, types,
                                )
                            });
                        Type::ArraySized {
                            element: left_id,
                            count: count_type_id,
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                arguments: _,
                qualifier,
                static_arguments,
            } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
                let static_arguments = self.evaluate_static_arguments(
                    module,
                    profile,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                )?;
                if let Expression::ScalarLiteral {
                    value: ScalarLiteral::String(target),
                } = tree.get(target)
                {
                    Type::Import {
                        target: *target,
                        qualifier: qualifier.clone(),
                        static_arguments,
                    }
                } else {
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    }
                }
            }
            Expression::TypeInfer { name, constraint } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                    let _timing =
                        self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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
                    let _timing =
                        self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_TYPE_OP);
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

            // tuple (anonymous)
            Expression::ArrayExpression { elements } => {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_LITERAL);
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
                    if matches!(
                        types.get_type(value_ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Void
                        }
                    ) {
                        return Err(AnalyzeError::VoidInTuple {
                            node: value_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_LITERAL);
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
                    if matches!(
                        types.get_type(value_ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Void
                        }
                    ) {
                        return Err(AnalyzeError::VoidInTuple {
                            node: value_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
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
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_EXPRESSION_LITERAL);
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
                                    false,
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
                                false,
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
                    if matches!(
                        types.get_type(left_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Void
                        }
                    ) {
                        return Err(AnalyzeError::VoidInArray {
                            node: left.into_global_any(module.id).into_anchored(Some(profile)),
                        });
                    }

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
                    let count_type_id =
                        self.array_sized_count_type_id_for_expression(module.id, right, types);
                    Type::ArraySized {
                        element: left_id,
                        count: count_type_id,
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
                    if matches!(
                        types.get_type(left_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Void
                        }
                    ) {
                        return Err(AnalyzeError::VoidInArray {
                            node: left.into_global_any(module.id).into_anchored(Some(profile)),
                        });
                    }
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

    /// Resolve a reference symbol for type evaluation.
    fn resolve_type_reference_symbol_for_evaluation(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> GlobalSymbolId {
        // follow import dependency items before normalization
        let mut resolved_symbol = target_symbol;
        if resolved_symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(resolved_symbol.local_id);
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
                    resolved_symbol = *dependency_target;
                }
            }
        }

        // keep simple local type-space symbols in fast path form
        if resolved_symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(resolved_symbol.local_id);
            if symbol_entry.is_static_parameter() {
                return GlobalSymbolId::new(
                    module.id,
                    resolved_symbol.local_id.with_type(symbol_entry.ty),
                );
            }

            let is_simple = symbol_entry.target_symbol.is_none()
                && symbol_entry.canonical_symbol.is_none()
                && symbol_entry.merge_group.is_none()
                && symbol_entry.kind != SymbolKind::Namespace;
            let is_type_space = matches!(
                symbol_entry.space,
                SymbolSpace::Type | SymbolSpace::TypeValue
            );
            if is_simple && is_type_space {
                return GlobalSymbolId::new(
                    module.id,
                    resolved_symbol.local_id.with_type(symbol_entry.ty),
                );
            }
        }

        // normalize and canonicalize for all non-fast-path cases
        let normalized = self.normalize_reference_symbol_id(module, profile, resolved_symbol);
        let canonical = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            normalized,
            CanonicalSymbolMode::PreserveAliases,
        );

        self.merged_type_symbol_id(module, symbols, profile, canonical)
    }

    /// Evaluate a reference to a nominal symbol into a Type.
    fn evaluate_type_reference_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        static_arguments: Option<Vec<StaticArgument>>,
        resolve_static_arguments: bool,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // check cached reference types first
        let reference_cache_key = self.type_reference_cache_key(
            target_symbol,
            static_arguments.as_deref(),
            validate_static_argument_bounds,
            enforce_implicit_managed,
            resolve_static_arguments,
        );
        let cached_reference = reference_cache_key
            .and_then(|cache_key| types.get_type_reference_cache(cache_key).cloned());
        if let Some(cached) = cached_reference {
            return Ok(cached);
        }

        // defer static argument resolution when requested
        if !resolve_static_arguments {
            let ty = Type::Reference {
                symbol: target_symbol,
                static_arguments,
            };
            self.cache_type_reference_maybe(reference_cache_key, &ty, types);
            return Ok(ty);
        }

        // resolve static arguments against declared bounds
        let options = self.analyze_context_options_for_module(module.id);
        let has_explicit_arguments = static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty());
        let parameter_symbols = self.collect_static_parameter_symbols(
            module,
            target_symbol,
            profile,
            tree,
            symbols,
            types,
        );
        let parameters_known = parameter_symbols.is_some();
        let has_parameters = parameter_symbols.is_some_and(|parameters| !parameters.is_empty());
        let resolved_arguments = if !has_explicit_arguments && parameters_known && !has_parameters {
            None
        } else {
            let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE_ARGUMENTS);
            self.resolve_type_reference_static_arguments_for_symbol(
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
            )?
        };
        let static_arguments = resolved_arguments.or(static_arguments);

        // return errors directly when static arguments failed to resolve
        let has_error_argument = static_arguments.as_deref().is_some_and(|arguments| {
            arguments.iter().any(|argument| match argument {
                StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } => types.get_type(*ty).is_error(),
                _ => false,
            })
        });
        if has_error_argument {
            return Ok(Type::Error);
        }

        // fold value-space constant references used in type positions
        let symbol_space = self.symbol_space_for_reference(module, profile, target_symbol, symbols);
        if symbol_space == Some(SymbolSpace::Value) {
            let mut visited = HashSet::new();
            if let Some(static_value) = self.static_expression_from_constant_reference(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
                types,
                None,
                &mut visited,
            )? {
                let constant_type = match static_value {
                    StaticExpression::ScalarLiteral { value } => Some(Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value),
                    }),
                    StaticExpression::TypeLiteral { value } => Some(Type::TypeLiteral { value }),
                    StaticExpression::Type { ty } => Some(types.get_type(ty).clone()),
                    _ => None,
                };

                if let Some(constant_type) = constant_type {
                    self.cache_type_reference_maybe(reference_cache_key, &constant_type, types);
                    return Ok(constant_type);
                }
            }

            return Ok(Type::Unevaluated(expression_id));
        }

        // normalize well known references into canonical structural types
        let normalized =
            if let Some(well_known) = self.well_known_array_kind(profile, target_symbol) {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE_WELL_KNOWN);
                self.normalize_well_known_type_reference(
                    module,
                    symbols,
                    profile,
                    expression_id.into_any(),
                    target_symbol,
                    well_known,
                    static_arguments.as_deref(),
                    types,
                )
            } else {
                None
            };
        if let Some(normalized) = normalized {
            self.cache_type_reference_maybe(reference_cache_key, &normalized, types);
            return Ok(normalized);
        }

        // fall back to a nominal reference
        let ty = Type::Reference {
            symbol: target_symbol,
            static_arguments,
        };
        self.cache_type_reference_maybe(reference_cache_key, &ty, types);
        Ok(ty)
    }

    /// Resolve the symbol space for a type-reference target.
    fn symbol_space_for_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> Option<SymbolSpace> {
        self.with_module_symbols_or_local(
            module,
            profile,
            target_symbol.module_id,
            symbols,
            |_owner_module, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(target_symbol.local_id);
                Some(symbol_entry.space)
            },
        )
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
        let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_TYPEOF);

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
    use destack_dir::{
        Expression, LocalScopeMark, LocalTypeId, PrimitiveType, ScalarLiteral, StaticKey,
        SymbolTable, Type, TypeLiteral, TypeTable,
    };
    use destack_source::ProfileId;

    use crate::TestProgram;

    /// Assert a fixed-array count resolves to either an integer literal or a named symbol.
    fn assert_count_matches_integer_or_symbol_name(
        count: LocalTypeId,
        expected_integer: i64,
        expected_symbol_name: StaticKey,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) {
        let mut type_id = types.unwrap_value_type_id(count);
        for _ in 0..16 {
            match types.get_type(type_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                } => {
                    assert_eq!(*value, expected_integer);
                    return;
                }
                Type::Value { value } => type_id = *value,
                Type::Reference { symbol, .. } => {
                    let symbol_key = symbols.get_symbol(symbol.local_id).key;
                    assert_eq!(symbol_key, Some(expected_symbol_name));
                    return;
                }
                other => panic!("expected integer literal or reference count type, got {other:?}"),
            }
        }

        panic!("expected integer literal or reference count type within unwrap steps")
    }

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

    #[test]
    fn test_associated_comptime_projection_uses_member_value_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
class MessagePage {
    comptime const Rows: number = 128;
}

declare const rows: MessagePage.Rows;
"#,
        );
        test.analyze_module(module_id);
        test.compile_check_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let code = module.code();
        assert!(
            !code.dirs.is_empty(),
            "expected at least one profile DIR after compile"
        );

        for (profile_index, dir) in code.dirs.iter().enumerate() {
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let types = dir.types.read();

            let class_key = StaticKey::Name(test.program.strings.intern("MessagePage"));
            let rows_key = StaticKey::Name(test.program.strings.intern("Rows"));
            let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
            let class_symbol = symbols
                .find_active_symbol_up_to(namespace_scope, class_key, LocalScopeMark::end())
                .map(|symbol| symbol.into_global(module.id))
                .expect("expected MessagePage symbol");
            let rows_symbol = test
                .compiler
                .resolve_static_member_symbol_in_tables(
                    &module,
                    ProfileId::new(profile_index as u32),
                    class_symbol,
                    rows_key,
                    &tree,
                    &symbols,
                )
                .expect("expected MessagePage.Rows symbol");
            let rows_type_id = types
                .get_value_type_id(rows_symbol)
                .expect("expected MessagePage.Rows value type");
            let rows_type = types.get_type(rows_type_id).clone();
            assert!(
                matches!(
                    rows_type,
                    Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(128))
                    }
                ),
                "expected Rows value type to be 128 in profile #{profile_index}, got {rows_type:?}"
            );

            let binding_key = StaticKey::Name(test.program.strings.intern("rows"));
            let binding_symbol = symbols
                .find_active_symbol_up_to(namespace_scope, binding_key, LocalScopeMark::end())
                .map(|symbol| symbol.into_global(module.id))
                .expect("expected rows binding symbol");
            let binding_type_id = types
                .get_value_type_id(binding_symbol)
                .expect("expected rows binding value type");
            let binding_type = types.get_type(binding_type_id);
            assert!(
                !binding_type.is_unevaluated(),
                "expected rows binding type to be evaluated in profile #{profile_index}, got {binding_type:?}"
            );
        }
    }

    /// Preserve nested fixed-size array literals in type positions.
    #[test]
    fn test_nested_fixed_array_literals_in_type_position() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
declare const grid: float32[16][16];
"#,
        );
        test.analyze_module(module_id);
        test.compile_check_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        let grid_key = StaticKey::Name(test.program.strings.intern("grid"));
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
        let grid_symbol = symbols
            .find_active_symbol_up_to(namespace_scope, grid_key, LocalScopeMark::end())
            .map(|symbol| symbol.into_global(module.id))
            .expect("expected grid symbol");
        let grid_type_id = types
            .get_value_type_id(grid_symbol)
            .expect("expected grid value type");

        let Type::ArraySized {
            element: outer_element,
            ..
        } = types.get_type(grid_type_id)
        else {
            panic!(
                "expected outer fixed array type, got {:?}",
                types.get_type(grid_type_id)
            );
        };

        let Type::ArraySized { .. } = types.get_type(*outer_element) else {
            panic!(
                "expected inner fixed array type, got {:?}",
                types.get_type(*outer_element)
            );
        };

        // ensure the type expression tree remains addressable for diagnostics
        let statement_id = dir.roots[0];
        let Expression::Statement { statement } = tree.get(statement_id) else {
            panic!("expected statement root");
        };
        let Expression::Let { declarators, .. } = tree.get(*statement) else {
            panic!("expected let declaration");
        };
        let declarator_id = declarators[0];
        let declared_type_id = types
            .get_declared_type_id(declarator_id.into_global_any(module.id))
            .expect("expected declared type id for grid");
        let declared_type = types.get_type(declared_type_id);
        assert!(
            !declared_type.is_unevaluated(),
            "expected declared type to be evaluated, got {declared_type:?}"
        );
    }

    /// Materialize interface associated comptime members in projected alias counts.
    #[test]
    fn test_interface_associated_alias_projection_materializes_comptime_counts() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
interface PartitionedStore<Row> {
    comptime const SegmentBytes: number;
    type Segment = Row[this.SegmentBytes];
}

class AuditStore implements PartitionedStore<string> {
    comptime const SegmentBytes: number = 1024;
}

declare const segment: AuditStore.Segment;
"#,
        );
        test.analyze_module(module_id);
        test.compile_check_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();

        let segment_key = StaticKey::Name(test.program.strings.intern("segment"));
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
        let segment_symbol = symbols
            .find_active_symbol_up_to(namespace_scope, segment_key, LocalScopeMark::end())
            .map(|symbol| symbol.into_global(module.id))
            .expect("expected segment symbol");

        let audit_key = StaticKey::Name(test.program.strings.intern("AuditStore"));
        let audit_symbol = symbols
            .find_active_symbol_up_to(namespace_scope, audit_key, LocalScopeMark::end())
            .map(|symbol| symbol.into_global(module.id))
            .expect("expected AuditStore symbol");
        let segment_member_key = StaticKey::Name(test.program.strings.intern("Segment"));
        let segment_member_symbol = test
            .compiler
            .resolve_static_member_symbol_in_tables(
                &module,
                profile,
                audit_symbol,
                segment_member_key,
                &tree,
                &symbols,
            )
            .expect("expected Segment member symbol");
        let alias_target_id = test
            .compiler
            .alias_target_type_id_for_symbol(
                &module,
                profile,
                segment_member_symbol,
                dir.roots[0].into_any(),
                &symbols,
                &mut types,
            )
            .expect("expected alias target for Segment member");
        let alias_count = match types.get_type(alias_target_id) {
            Type::ArraySized { count, .. } => *count,
            other => panic!("expected fixed-size alias target, got {other:?}"),
        };
        assert_count_matches_integer_or_symbol_name(
            alias_count,
            1024,
            StaticKey::Name(test.program.strings.intern("SegmentBytes")),
            &symbols,
            &types,
        );

        let segment_type_id = types
            .get_value_type_id(segment_symbol)
            .expect("expected segment value type");

        let count = match types.get_type(segment_type_id) {
            Type::ArraySized { count, .. } => *count,
            other => panic!("expected fixed-size segment projection, got {other:?}"),
        };
        assert_count_matches_integer_or_symbol_name(
            count,
            1024,
            StaticKey::Name(test.program.strings.intern("SegmentBytes")),
            &symbols,
            &types,
        );
    }
}
