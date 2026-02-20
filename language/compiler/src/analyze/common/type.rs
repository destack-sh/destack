use std::collections::HashSet;

use destack_dir::{
    Block, Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalTypeId, NodeTree,
    NodeType, PrimitiveType, RuntimeCheckKind, ScalarLiteral, StaticArgument, StaticExpression,
    StaticKey, StaticParameterKind, SymbolTable, SymbolType, Type, TypeLiteral, TypeTable,
    TypeUnaryOperator, TypeVisitor, TypeVisitorOptions, walk_static_argument,
    walk_static_expression, walk_type,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::{
    AnalyzeDependencyStage, CanonicalSymbolMode, NormalizationMode, RelationMode, TypeCollector,
    TypeWalkContext, TypeWalkKey,
};
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, Compiler, ElaborateError, ElaborateResult,
};

/// Maximum number of unwrap steps when chasing type value wrappers.
const MAX_TYPE_VALUE_UNWRAP_STEPS: usize = 8;

fn base_visitor_options() -> TypeVisitorOptions {
    TypeWalkContext::new(TypeWalkKey::BASE).visitor_options()
}

/// The mode used for visited tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitedMode {
    /// Track a per-walk stack to allow revisiting nodes.
    Stack,
    /// Track visited nodes for the full traversal.
    Set,
}

/// The containment query to run while walking types.
enum TypeContainmentKind<'a> {
    /// Detect error types.
    ErrorType,
    /// Detect unevaluated static arguments or expressions.
    UnevaluatedStaticArgument,
    /// Detect unevaluated value static arguments.
    UnevaluatedValueStaticArgument {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The current module.
        module: &'a Module,
        /// The active profile.
        profile: ProfileId,
        /// The node tree for the current module.
        tree: &'a NodeTree,
        /// The symbol table for the current module.
        symbols: &'a SymbolTable,
        /// The type table for the current module.
        types: &'a TypeTable,
    },
    /// Detect static parameter usage.
    StaticParameter {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The current module.
        module: &'a Module,
        /// The active profile.
        profile: ProfileId,
        /// The symbol table for the current module.
        symbols: &'a SymbolTable,
        /// The type table for the current module.
        types: &'a TypeTable,
    },
    /// Detect free static parameters.
    FreeStaticParameter {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The current module.
        module: &'a Module,
        /// The active profile.
        profile: ProfileId,
        /// The symbol table for the current module.
        symbols: &'a SymbolTable,
        /// The type table for the current module.
        types: &'a TypeTable,
    },
    /// Detect conditional infer bindings.
    InferBinding,
    /// Detect inference variables.
    InferVar,
    /// Detect forbidden literal usage.
    ForbiddenLiteral {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The current module.
        module: &'a Module,
        /// The type table for the current module.
        types: &'a TypeTable,
        /// The literal predicate used for filtering.
        predicate: fn(&TypeLiteral) -> bool,
        /// Whether imported types should be skipped.
        skip_imported_types: bool,
    },
    /// Detect managed defaults.
    ManagedType {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The current module.
        module: &'a Module,
        /// The active profile.
        profile: ProfileId,
        /// The type table for the current module.
        types: &'a TypeTable,
    },
}

/// Walk types to detect containment queries.
pub(super) struct TypeContainmentVisitor<'a> {
    /// The visited type ids.
    visited: &'a mut HashSet<LocalTypeId>,
    /// Whether a match was found.
    found: bool,
    /// Whether traversal is inside a static argument.
    in_static_argument: bool,
    /// The bound static parameter symbols for free parameter checks.
    free_static_bound: Option<HashSet<GlobalSymbolId>>,
    /// The visited symbols for reference traversal.
    visited_symbols: Option<&'a mut HashSet<GlobalSymbolId>>,
    /// The containment kind to detect.
    kind: TypeContainmentKind<'a>,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> TypeContainmentVisitor<'a> {
    /// Create a visitor for error type detection.
    fn new_error(visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self::new(TypeContainmentKind::ErrorType, visited, None)
    }

    /// Create a visitor for unevaluated static arguments.
    fn new_unevaluated_static(visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self::new(
            TypeContainmentKind::UnevaluatedStaticArgument,
            visited,
            None,
        )
    }

    /// Create a visitor for unevaluated value static arguments.
    fn new_unevaluated_value_static(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::UnevaluatedValueStaticArgument {
                compiler,
                module,
                profile,
                tree,
                symbols,
                types,
            },
            visited,
            None,
        )
    }

    /// Create a visitor for static parameter detection.
    fn new_static_parameter(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::StaticParameter {
                compiler,
                module,
                profile,
                symbols,
                types,
            },
            visited,
            None,
        )
    }

    /// Create a visitor for free static parameter detection.
    fn new_free_static_parameter(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        bound: &HashSet<GlobalSymbolId>,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::FreeStaticParameter {
                compiler,
                module,
                profile,
                symbols,
                types,
            },
            visited,
            None,
        )
        .with_free_static_bound(bound.clone())
    }

    /// Create a visitor for conditional infer bindings.
    fn new_infer_binding(visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self::new(TypeContainmentKind::InferBinding, visited, None)
    }

    /// Create a visitor for inference variable containment.
    fn new_infer_var(visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self::new(TypeContainmentKind::InferVar, visited, None)
    }

    /// Create a visitor for forbidden literal detection.
    pub(super) fn new_forbidden_literal(
        compiler: &'a Compiler,
        module: &'a Module,
        types: &'a TypeTable,
        predicate: fn(&TypeLiteral) -> bool,
        skip_imported_types: bool,
        visited_types: &'a mut HashSet<LocalTypeId>,
        visited_symbols: &'a mut HashSet<GlobalSymbolId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::ForbiddenLiteral {
                compiler,
                module,
                types,
                predicate,
                skip_imported_types,
            },
            visited_types,
            Some(visited_symbols),
        )
    }

    /// Create a visitor for managed default detection.
    pub(super) fn new_managed_type(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        types: &'a TypeTable,
        visited_types: &'a mut HashSet<LocalTypeId>,
        visited_symbols: &'a mut HashSet<GlobalSymbolId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::ManagedType {
                compiler,
                module,
                profile,
                types,
            },
            visited_types,
            Some(visited_symbols),
        )
    }

    /// Run a containment query for the given type id.
    pub(super) fn contains(mut self, types: &TypeTable, type_id: LocalTypeId) -> bool {
        self.visit_type_id(types, type_id);
        self.found
    }

    /// Create a containment visitor for the given kind.
    fn new(
        kind: TypeContainmentKind<'a>,
        visited: &'a mut HashSet<LocalTypeId>,
        visited_symbols: Option<&'a mut HashSet<GlobalSymbolId>>,
    ) -> Self {
        Self {
            visited,
            found: false,
            in_static_argument: false,
            free_static_bound: None,
            visited_symbols,
            kind,
            options: base_visitor_options(),
        }
    }

    /// Attach a free static parameter bound set.
    fn with_free_static_bound(mut self, bound: HashSet<GlobalSymbolId>) -> Self {
        self.free_static_bound = Some(bound);
        self
    }

    /// Return the visited mode for the containment kind.
    fn visited_mode(&self) -> VisitedMode {
        match self.kind {
            TypeContainmentKind::ErrorType => VisitedMode::Stack,
            TypeContainmentKind::UnevaluatedStaticArgument => VisitedMode::Stack,
            TypeContainmentKind::UnevaluatedValueStaticArgument { .. } => VisitedMode::Stack,
            TypeContainmentKind::StaticParameter { .. } => VisitedMode::Set,
            TypeContainmentKind::FreeStaticParameter { .. } => VisitedMode::Set,
            TypeContainmentKind::InferBinding => VisitedMode::Set,
            TypeContainmentKind::InferVar => VisitedMode::Set,
            TypeContainmentKind::ForbiddenLiteral { .. } => VisitedMode::Set,
            TypeContainmentKind::ManagedType { .. } => VisitedMode::Set,
        }
    }

    /// Return true when this query should skip imported types.
    fn skip_imported_types(&self) -> bool {
        match self.kind {
            TypeContainmentKind::ForbiddenLiteral {
                skip_imported_types,
                ..
            } => skip_imported_types,
            _ => false,
        }
    }

    /// Handle static argument traversal for unevaluated checks.
    fn visit_unevaluated_static_argument(&mut self, types: &TypeTable, argument: &StaticArgument) {
        match argument {
            StaticArgument::Unevaluated { .. } => {
                self.found = true;
            }
            StaticArgument::Evaluated { value, .. } => {
                walk_static_expression(self, types, value);
            }
        }
    }

    /// Take the free static parameter bound set.
    fn take_free_static_bound(&mut self) -> HashSet<GlobalSymbolId> {
        self.free_static_bound
            .take()
            .expect("free static parameter bound missing")
    }

    /// Restore the free static parameter bound set.
    fn restore_free_static_bound(&mut self, bound: HashSet<GlobalSymbolId>) {
        self.free_static_bound = Some(bound);
    }
}

impl TypeVisitor for TypeContainmentVisitor<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        if self.found {
            return;
        }
        if self.skip_imported_types() && types.is_imported_type(id) {
            return;
        }
        if !self.visited.insert(id) {
            return;
        }
        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
        if matches!(self.visited_mode(), VisitedMode::Stack) {
            self.visited.remove(&id);
        }
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        if self.found {
            return;
        }

        match &self.kind {
            TypeContainmentKind::ErrorType => {
                if matches!(ty, Type::Error) {
                    self.found = true;
                    return;
                }
            }
            TypeContainmentKind::UnevaluatedStaticArgument => {}
            TypeContainmentKind::UnevaluatedValueStaticArgument {
                compiler,
                module,
                profile,
                tree,
                symbols,
                types: type_table,
            } => {
                if let Type::Reference {
                    symbol,
                    static_arguments,
                } = ty
                {
                    if compiler.reference_contains_unevaluated_value_arguments(
                        module,
                        *profile,
                        *symbol,
                        static_arguments.as_deref(),
                        tree,
                        symbols,
                        type_table,
                        self.visited,
                    ) {
                        self.found = true;
                    }
                    return;
                }
            }
            TypeContainmentKind::StaticParameter {
                compiler,
                module,
                profile,
                symbols,
                types: type_table,
            } => match ty {
                Type::Reference { symbol, .. } => {
                    if compiler
                        .symbol_is_static_parameter(module, *profile, *symbol, symbols, type_table)
                    {
                        self.found = true;
                        return;
                    }
                }
                Type::Infer { .. } => {
                    if self.in_static_argument {
                        self.found = true;
                        return;
                    }
                }
                _ => {}
            },
            TypeContainmentKind::FreeStaticParameter {
                compiler,
                module,
                profile,
                symbols,
                types: type_table,
            } => match ty {
                Type::Reference {
                    symbol,
                    static_arguments,
                } => {
                    let bound = self
                        .free_static_bound
                        .as_ref()
                        .expect("free static parameter bound missing");
                    if static_arguments.is_none()
                        && compiler.symbol_is_static_parameter(
                            module, *profile, *symbol, symbols, type_table,
                        )
                        && !bound.contains(symbol)
                    {
                        self.found = true;
                        return;
                    }
                }
                Type::Mapped {
                    parameter, value, ..
                } => {
                    let mut bound = self.take_free_static_bound();
                    let inserted = bound.insert(parameter.symbol);
                    self.restore_free_static_bound(bound);
                    self.visit_type_id(types, parameter.constraint);
                    if let Some(key_remap) = parameter.key_remap {
                        self.visit_type_id(types, key_remap);
                    }
                    self.visit_type_id(types, *value);
                    if inserted {
                        let mut bound = self.take_free_static_bound();
                        bound.remove(&parameter.symbol);
                        self.restore_free_static_bound(bound);
                    }
                    return;
                }
                Type::Conditional {
                    distributive_symbol,
                    left,
                    right,
                    then_type,
                    else_type,
                } => {
                    // treat distributive symbols as binders for this conditional
                    let mut inserted = false;
                    if let Some(distributive_symbol) = distributive_symbol {
                        let mut bound = self.take_free_static_bound();
                        inserted = bound.insert(*distributive_symbol);
                        self.restore_free_static_bound(bound);
                    }

                    self.visit_type_id(types, *left);
                    self.visit_type_id(types, *right);
                    self.visit_type_id(types, *then_type);
                    self.visit_type_id(types, *else_type);

                    if inserted {
                        let mut bound = self.take_free_static_bound();
                        if let Some(distributive_symbol) = distributive_symbol {
                            bound.remove(distributive_symbol);
                        }
                        self.restore_free_static_bound(bound);
                    }
                    return;
                }
                _ => {}
            },
            TypeContainmentKind::InferBinding => {
                if matches!(ty, Type::Infer { .. }) {
                    self.found = true;
                    return;
                }
            }
            TypeContainmentKind::InferVar => {
                if matches!(ty, Type::InferVar { .. }) {
                    self.found = true;
                    return;
                }
            }
            TypeContainmentKind::ForbiddenLiteral {
                compiler,
                module,
                types: type_table,
                predicate,
                ..
            } => match ty {
                Type::TypeLiteral { value } => {
                    if (predicate)(value) {
                        self.found = true;
                        return;
                    }
                }
                Type::Unary {
                    operator: TypeUnaryOperator::Keyof,
                    ..
                } => {
                    return;
                }
                Type::Reference { symbol, .. } => {
                    let skip_imported_types = self.skip_imported_types();
                    let visited_symbols = self
                        .visited_symbols
                        .as_deref_mut()
                        .expect("visited symbols missing");
                    if compiler.type_reference_contains_forbidden_literal(
                        module,
                        *symbol,
                        type_table,
                        *predicate,
                        skip_imported_types,
                        self.visited,
                        visited_symbols,
                    ) {
                        self.found = true;
                    }
                    return;
                }
                _ => {}
            },
            TypeContainmentKind::ManagedType {
                compiler,
                module,
                profile,
                types: type_table,
            } => match ty {
                Type::ValueOf { .. } | Type::ReferenceOf { .. } | Type::PointerOf { .. } => {
                    return;
                }
                Type::Reference { symbol, .. } => {
                    let visited_symbols = self
                        .visited_symbols
                        .as_deref_mut()
                        .expect("visited symbols missing");
                    if compiler.symbol_is_managed_inner(
                        module,
                        *profile,
                        *symbol,
                        type_table,
                        visited_symbols,
                        false,
                    ) {
                        self.found = true;
                    }
                    return;
                }
                Type::Object { .. } | Type::Array { .. } | Type::Function { .. } | Type::This => {
                    self.found = true;
                    return;
                }
                Type::TypeLiteral { value } => {
                    if compiler.type_literal_is_managed(value) {
                        self.found = true;
                        return;
                    }
                }
                _ => {}
            },
        }

        walk_type(self, types, id, ty);
    }

    fn visit_static_argument(&mut self, types: &TypeTable, argument: &StaticArgument) {
        if self.found {
            return;
        }

        match &self.kind {
            TypeContainmentKind::StaticParameter { .. } => match argument {
                StaticArgument::Unevaluated { .. } => {
                    self.found = true;
                }
                StaticArgument::Evaluated { value, .. } => {
                    let previous = self.in_static_argument;
                    self.in_static_argument = true;
                    walk_static_expression(self, types, value);
                    self.in_static_argument = previous;
                }
            },
            TypeContainmentKind::FreeStaticParameter { .. }
            | TypeContainmentKind::UnevaluatedStaticArgument
            | TypeContainmentKind::UnevaluatedValueStaticArgument { .. } => {
                self.visit_unevaluated_static_argument(types, argument);
            }
            _ => {
                walk_static_argument(self, types, argument);
            }
        }
    }

    fn visit_static_expression(&mut self, types: &TypeTable, expression: &StaticExpression) {
        if self.found {
            return;
        }

        match &self.kind {
            TypeContainmentKind::ErrorType => {}
            TypeContainmentKind::FreeStaticParameter { .. }
            | TypeContainmentKind::StaticParameter { .. }
            | TypeContainmentKind::UnevaluatedStaticArgument
            | TypeContainmentKind::UnevaluatedValueStaticArgument { .. } => {
                if matches!(expression, StaticExpression::Unevaluated { .. }) {
                    self.found = true;
                    return;
                }
            }
            TypeContainmentKind::ForbiddenLiteral { predicate, .. } => {
                if let StaticExpression::TypeLiteral { value } = expression {
                    if (predicate)(value) {
                        self.found = true;
                    }
                    return;
                }
            }
            _ => {}
        }

        walk_static_expression(self, types, expression);
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return true when a type already represents a primary semantic failure.
    pub(crate) fn type_blocks_follow_on_diagnostic(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        matches!(types.get_type(ty_id), Type::Error)
    }

    /// Report one unassignable-type diagnostic unless either side already failed.
    pub(crate) fn report_unassignable_type_for_types(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let Some(error) = self.unassignable_type_error_for_types(
            module,
            profile,
            node_id,
            expected_ty_id,
            actual_ty_id,
            types,
        ) else {
            return false;
        };
        debug_assert!(error.is_follow_on_semantic_diagnostic());
        self.error(error);

        true
    }

    /// Build one unassignable-type error unless either side already failed.
    pub(crate) fn unassignable_type_error_for_types(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<AnalyzeError> {
        if self.type_blocks_follow_on_diagnostic(expected_ty_id, types)
            || self.type_blocks_follow_on_diagnostic(actual_ty_id, types)
        {
            return None;
        }

        Some(AnalyzeError::UnassignableType {
            node: node_id.into_global(module.id).into_anchored(Some(profile)),
            expected_ty: expected_ty_id.into_global(module.id),
            actual_ty: actual_ty_id.into_global(module.id),
        })
    }

    /// Report one unsatisfied-type diagnostic unless either side already failed.
    pub(crate) fn report_unsatisfied_type_for_types(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let Some(error) = self.unsatisfied_type_error_for_types(
            module,
            profile,
            node_id,
            expected_ty_id,
            actual_ty_id,
            types,
        ) else {
            return false;
        };
        debug_assert!(error.is_follow_on_semantic_diagnostic());
        self.error(error);

        true
    }

    /// Build one unsatisfied-type error unless either side already failed.
    pub(crate) fn unsatisfied_type_error_for_types(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<AnalyzeError> {
        if self.type_blocks_follow_on_diagnostic(expected_ty_id, types)
            || self.type_blocks_follow_on_diagnostic(actual_ty_id, types)
        {
            return None;
        }

        Some(AnalyzeError::UnsatisfiedType {
            node: node_id.into_global(module.id).into_anchored(Some(profile)),
            expected_ty: expected_ty_id.into_global(module.id),
            actual_ty: actual_ty_id.into_global(module.id),
        })
    }

    /// Report one excess-property diagnostic unless the expected type already failed.
    pub(crate) fn report_excess_property_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        member_key: StaticKey,
        types: &TypeTable,
    ) -> bool {
        let Some(error) = self.excess_property_error_for_type(
            module,
            profile,
            node_id,
            expected_ty_id,
            member_key,
            types,
        ) else {
            return false;
        };
        debug_assert!(error.is_follow_on_semantic_diagnostic());
        self.error(error);

        true
    }

    /// Build one excess-property error unless the expected type already failed.
    pub(crate) fn excess_property_error_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        member_key: StaticKey,
        types: &TypeTable,
    ) -> Option<AnalyzeError> {
        if self.type_blocks_follow_on_diagnostic(expected_ty_id, types) {
            return None;
        }

        Some(AnalyzeError::ExcessProperty {
            node: node_id.into_global(module.id).into_anchored(Some(profile)),
            expected_ty: expected_ty_id.into_global(module.id),
            member_key,
        })
    }

    /// Report one no-overload diagnostic unless the receiver already failed.
    pub(crate) fn report_no_overload_for_receiver_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        receiver_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        if self.type_blocks_follow_on_diagnostic(receiver_ty_id, types) {
            return false;
        }

        let error = AnalyzeError::NoOverload {
            node: node_id.into_global(module.id).into_anchored(Some(profile)),
            receiver_ty: receiver_ty_id.into_global(module.id),
        };
        debug_assert!(error.is_follow_on_semantic_diagnostic());
        self.error(error);

        true
    }

    /// Report one non-callable diagnostic unless the callee already failed.
    pub(crate) fn report_non_callable_for_callee_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        callee_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        if self.type_blocks_follow_on_diagnostic(callee_ty_id, types) {
            return false;
        }

        let error = AnalyzeError::NonCallable {
            node: node_id.into_global(module.id).into_anchored(Some(profile)),
        };
        debug_assert!(error.is_follow_on_semantic_diagnostic());
        self.error(error);

        true
    }

    /// Record an inferred type for a synthesized expression.
    pub(crate) fn set_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        type_id: LocalTypeId,
    ) {
        types.set_inferred_type(expression_id.into_global_any(module_id), type_id);
    }

    /// Record a boolean type for a synthesized expression.
    pub(crate) fn set_boolean_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let bool_type = Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        };
        let bool_type_id = types.insert_type_from(bool_type, expression_id);
        self.set_expression_type(types, module_id, expression_id, bool_type_id);
    }

    /// Record a scalar literal type for a synthesized literal expression.
    pub(crate) fn set_scalar_literal_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        literal: ScalarLiteral,
    ) {
        let literal_type = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        };
        let literal_type_id = types.insert_type_from(literal_type, expression_id);
        self.set_expression_type(types, module_id, expression_id, literal_type_id);
    }

    /// Record a void type for a synthesized expression.
    pub(crate) fn set_void_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let void_type_id = self.void_type_id(types, expression_id.into_any());
        self.set_expression_type(types, module_id, expression_id, void_type_id);
    }

    /// Record a never type for a synthesized expression.
    pub(crate) fn set_never_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
    ) {
        let never_type_id = self.never_type_id(types, expression_id.into_any());
        self.set_expression_type(types, module_id, expression_id, never_type_id);
    }

    /// Resolve a symbol value type id or return an error.
    pub(crate) fn value_type_id_or_error(
        &self,
        module_id: ModuleId,
        symbol: GlobalSymbolId,
        node_id: LocalNodeIdAny,
        types: &TypeTable,
    ) -> ElaborateResult<LocalTypeId> {
        types
            .get_value_type_id(symbol)
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: node_id.into_global(module_id).into_anchored(None),
            })
    }

    /// Resolve the declared or inferred type id for an expression or return an error.
    pub(crate) fn expression_type_id_or_error(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        types: &TypeTable,
    ) -> ElaborateResult<LocalTypeId> {
        types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: expression_id.into_global_any(module_id).into_anchored(None),
            })
    }

    /// Allocate a void type id for a synthesized node.
    pub(crate) fn void_type_id(
        &self,
        types: &mut TypeTable,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        types.insert_type_from_any(ty, node_id)
    }

    /// Allocate a boolean type id for a synthesized node.
    pub(crate) fn boolean_type_id(
        &self,
        types: &mut TypeTable,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        };
        types.insert_type_from_any(ty, node_id)
    }

    /// Allocate a never type id for a synthesized node.
    pub(crate) fn never_type_id(
        &self,
        types: &mut TypeTable,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Never,
        };
        types.insert_type_from_any(ty, node_id)
    }

    /// Update the inferred type for a block after rewriting expressions.
    pub(crate) fn reinfer_block_type(
        &self,
        block_id: LocalNodeId<Block>,
        tree: &NodeTree,
        types: &mut TypeTable,
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        // resolve the last expression type or default to void
        let block = tree.get(block_id);
        let block_type_id = if let Some(last_expression_id) = block.expressions.last() {
            types
                .get_declared_or_inferred_type_id(last_expression_id.into_global_any(module_id))
                .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                    node: last_expression_id
                        .into_global_any(module_id)
                        .into_anchored(None),
                })?
        } else {
            self.void_type_id(types, block_id.into_any())
        };

        // update the block node type
        types.set_inferred_type(block_id.into_global_any(module_id), block_type_id);

        // update any expression wrappers for the block
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            let Expression::Block { block } = tree.get(expression_id) else {
                continue;
            };
            if *block != block_id {
                continue;
            }
            self.set_expression_type(types, module_id, expression_id, block_type_id);
        }

        Ok(())
    }

    /// Record a void type for a block expression wrapper.
    pub(crate) fn set_void_block_expression_type(
        &self,
        types: &mut TypeTable,
        module_id: ModuleId,
        block_id: LocalNodeId<Block>,
        block_expression_id: LocalNodeId<Expression>,
    ) {
        let void_type_id = self.void_type_id(types, block_id.into_any());
        types.set_inferred_type(block_id.into_global_any(module_id), void_type_id);
        self.set_expression_type(types, module_id, block_expression_id, void_type_id);
    }

    /// Return true when a type is wrapped in explicit ownership modifiers.
    pub(crate) fn type_is_explicit_ownership_wrapper(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> bool {
        match types.get_type(type_id) {
            Type::ValueOf { .. } | Type::ReferenceOf { .. } | Type::PointerOf { .. } => true,
            Type::Value { value } => self.type_is_explicit_ownership_wrapper(types, *value),
            _ => false,
        }
    }

    /// Check whether a symbol is a static parameter.
    pub(crate) fn symbol_is_static_parameter(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // honor cached constraints for mapped parameters
        if types.get_static_parameter_constraint_type(symbol).is_some() {
            return true;
        }

        // rely on the declared parameter metadata
        let Some(symbol) = self
            .with_module_symbols_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                symbols,
                AnalyzeDependencyStage::Declare,
                |_, owner_symbols| owner_symbols.get_symbol(symbol.local_id).clone(),
            )
            .ok()
        else {
            return false;
        };
        if symbol.is_static_parameter() {
            return true;
        }

        symbol
            .primary_declaration
            .is_some_and(|declaration| declaration.local_id.ty == NodeType::Parameter)
    }

    /// Check whether a type contains a static parameter reference.
    pub(crate) fn type_contains_static_parameters(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_static_parameter(
            self, module, profile, symbols, types, visited,
        );
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Check whether a type contains free static parameter references.
    pub(crate) fn type_contains_free_static_parameters(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        bound: &HashSet<GlobalSymbolId>,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_free_static_parameter(
            self, module, profile, symbols, types, bound, visited,
        );
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Check whether a type contains conditional infer bindings.
    pub(crate) fn type_contains_infer(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_infer_binding(visited);
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Check whether a type contains inference variables.
    pub(crate) fn type_contains_infer_vars(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_infer_var(visited);
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Check whether a type needs instantiation before evaluation.
    pub(crate) fn type_needs_instantiation(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // TODO #Cleanup: centralize this gate in the evaluation boundary once we split structural and evaluative normalization
        // NOTE #Suspicious: type_needs_instantiation does not treat `this` as instantiation dependent yet
        // check for free static parameter references
        let mut static_visited = HashSet::new();
        let bound = HashSet::new();
        if self.type_contains_free_static_parameters(
            module,
            profile,
            type_id,
            &bound,
            symbols,
            types,
            &mut static_visited,
        ) {
            return true;
        }

        // check for inference variables
        let mut infer_visited = HashSet::new();
        if self.type_contains_infer_vars(type_id, types, &mut infer_visited) {
            return true;
        }

        // check for conditional infer bindings
        let mut binding_visited = HashSet::new();
        if self.type_contains_infer(type_id, types, &mut binding_visited) {
            return true;
        }

        false
    }

    /// Ensure a type id is evaluated when it is unevaluated.
    pub(crate) fn ensure_type_evaluated(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        if matches!(types.get_type(type_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(module, profile, type_id, tree, symbols, types)?;
        }
        Ok(type_id)
    }

    /// Materialize an imported type by evaluating unevaluated components when needed.
    pub(crate) fn materialize_imported_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        if !module.language_type.is_declaration() {
            self.ensure_type_evaluated(module, profile, type_id, tree, symbols, types)?;
            return Ok(());
        }

        // walk the type graph and evaluate unevaluated nodes before import
        let mut pending = Vec::new();
        let mut visited = HashSet::new();
        let mut discovered = Vec::new();
        pending.push(type_id);
        while let Some(current_id) = pending.pop() {
            if !visited.insert(current_id) {
                continue;
            }

            // evaluate unevaluated types before walking children
            if matches!(types.get_type(current_id), Type::Unevaluated(_)) {
                self.resolve_declared_type(module, profile, current_id, tree, symbols, types)?;
            }

            // keep walking the type graph to discover unevaluated types
            let ty = types.get_type(current_id).clone();
            {
                let mut visitor = TypeCollector::new(&mut discovered, base_visitor_options());
                walk_type(&mut visitor, types, current_id, &ty);
            }
            if !discovered.is_empty() {
                pending.append(&mut discovered);
            }
        }

        Ok(())
    }

    /// Resolve a type symbol from a type reference or type-as-value.
    pub(crate) fn unwrap_type_value_symbol(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // unwrap direct references
        if let Type::Reference { symbol, .. } = types.get_type(type_id) {
            return Some(*symbol);
        }

        // unwrap references stored in type-as-value wrappers
        if let Type::Value { value } = types.get_type(type_id)
            && let Type::Reference { symbol, .. } = types.get_type(*value)
        {
            return Some(*symbol);
        }

        None
    }

    /// Resolve an integer literal value from one type id when possible.
    pub(crate) fn integer_literal_value_for_type_id(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<i64> {
        let mut current_id = type_id;

        // peel wrappers and value symbols until we hit a concrete integer literal
        for _ in 0..MAX_TYPE_VALUE_UNWRAP_STEPS {
            match types.get_type(current_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                } => return Some(*value),
                Type::Value { value } => current_id = *value,
                Type::Reference { symbol, .. } => {
                    current_id = types.get_value_type_id(*symbol)?;
                }
                _ => return None,
            }
        }

        None
    }

    /// Follow symbol forwarding edges until a stable symbol is reached.
    pub(crate) fn forwarded_symbol_id(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> GlobalSymbolId {
        let mut current_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let mut visited = HashSet::new();

        // chase forwarding edges with cycle protection
        loop {
            if !visited.insert(current_symbol) {
                break;
            }

            let next_symbol = self
                .with_module_symbols_or_local_at_stage(
                    module,
                    profile,
                    current_symbol.module_id,
                    symbols,
                    AnalyzeDependencyStage::Declare,
                    |_owner_module, owner_symbols| {
                        let symbol_entry = owner_symbols.get_symbol(current_symbol.local_id);
                        symbol_entry.target_symbol.or(symbol_entry.canonical_symbol)
                    },
                )
                .ok()
                .flatten();
            let Some(next_symbol) = next_symbol else {
                break;
            };

            let next_symbol = self.canonical_symbol_id(
                module,
                symbols,
                profile,
                next_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            if next_symbol == current_symbol {
                break;
            }

            current_symbol = next_symbol;
        }

        current_symbol
    }

    /// Import the alias target type for a symbol when available.
    pub(crate) fn alias_target_type_id_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let mut visited = HashSet::new();
        let mut current = symbol;

        loop {
            if !visited.insert(current) {
                return None;
            }

            // load the local alias target when the symbol is local
            if current.module_id == module.id {
                let symbol_entry = symbols.get_symbol(current.local_id);
                let typed_symbol = GlobalSymbolId::new(
                    current.module_id,
                    current.local_id.with_type(symbol_entry.ty),
                );

                // check the incoming symbol first, then the declaration-typed symbol
                types.record_normalization_symbol_dependency(current);
                if let Some(target) = types.get_alias_target_type_id(current) {
                    return Some(target);
                }

                if matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                    types.record_normalization_symbol_dependency(typed_symbol);
                    if let Some(target) = types.get_alias_target_type_id(typed_symbol) {
                        return Some(target);
                    }
                }

                // follow import targets for local alias references
                let target_symbol = symbol_entry
                    .target_symbol
                    .or(symbol_entry.canonical_symbol)?;
                current = target_symbol;
                continue;
            }

            // import the alias target when the symbol is remote
            let (resolved, next) = match self.with_module_tree_symbols_at_stage(
                module,
                profile,
                current.module_id,
                AnalyzeDependencyStage::Declare,
                |owner_module, owner_tree, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(current.local_id);
                    if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                        let target_symbol =
                            symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
                        return (None, target_symbol);
                    }

                    let typed_symbol = GlobalSymbolId::new(
                        current.module_id,
                        current.local_id.with_type(symbol_entry.ty),
                    );
                    types.record_normalization_symbol_dependency(typed_symbol);

                    // resolve the remote alias target id without holding a write lock
                    let remote_target_id = {
                        let owner_types = owner_module.dir(profile).types.read();
                        match owner_types.get_alias_target_type_id(typed_symbol) {
                            Some(id) => id,
                            None => return (None, None),
                        }
                    };

                    // evaluate the remote alias target when needed
                    let needs_evaluation = {
                        let owner_types = owner_module.dir(profile).types.read();
                        matches!(owner_types.get_type(remote_target_id), Type::Unevaluated(_))
                    };
                    if needs_evaluation {
                        let mut owner_types = owner_module.dir(profile).types.write();
                        if matches!(owner_types.get_type(remote_target_id), Type::Unevaluated(_))
                            && let Err(error) = self.resolve_declared_type(
                                owner_module,
                                profile,
                                remote_target_id,
                                owner_tree,
                                owner_symbols,
                                &mut owner_types,
                            )
                        {
                            self.error(error);
                            return (None, None);
                        }
                    }

                    // read the evaluated remote alias target and import it locally
                    let owner_types = owner_module.dir(profile).types.read();
                    let needs_materialization = self
                        .type_contains_unevaluated_value_static_arguments(
                            owner_module,
                            profile,
                            remote_target_id,
                            owner_tree,
                            owner_symbols,
                            &owner_types,
                            &mut HashSet::new(),
                        );
                    if needs_materialization {
                        return (None, None);
                    }

                    let remote_target_ty = owner_types.get_type(remote_target_id);
                    let local_alias_target_id = self.import_type_from_remote_for_node(
                        source_id,
                        remote_target_ty,
                        &owner_types,
                        typed_symbol,
                        types,
                    );
                    types.set_alias_target_type_id(typed_symbol, local_alias_target_id);
                    (Some(local_alias_target_id), None)
                },
            ) {
                Ok(value) => value,
                Err(error) => {
                    let _ = error;
                    return None;
                }
            };
            if let Some(resolved) = resolved {
                return Some(resolved);
            }
            current = next?;
        }
    }

    /// Require an instance type for a symbol into the local type table.
    pub(crate) fn require_instance_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // resolve through canonical import targets while preserving aliases
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                module,
                symbols,
                profile,
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // reuse local instance types when already available
        types.record_normalization_symbol_dependency(symbol);
        if let Some(instance_id) = types.get_instance_type_id(symbol) {
            return Some(instance_id);
        }

        // load or import the instance type through the existing resolver
        match self.resolve_instance_type_for_symbol(module, profile, source_id, symbol, types) {
            Ok(instance_id) => instance_id,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Resolve the apparent instance type for shape queries like `keyof`.
    pub(crate) fn apparent_instance_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // NOTE #Suspicious: apparent type resolution prefers alias targets over instance types without checking instantiation
        // resolve through canonical import targets while preserving aliases
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                module,
                symbols,
                profile,
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // prefer alias targets as the apparent type when available
        if let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(module, profile, symbol, source_id, symbols, types)
        {
            return Some(alias_target_id);
        }

        // otherwise fall back to the instance type
        self.require_instance_type(module, profile, source_id, symbol, symbols, types)
    }

    /// Unwrap a type-as-value wrapper to the underlying type id.
    pub(crate) fn unwrap_type_value(&self, type_id: LocalTypeId, types: &TypeTable) -> LocalTypeId {
        match types.get_type(type_id) {
            Type::Value { value } => *value,
            _ => type_id,
        }
    }

    /// Resolve an enum symbol from a type when possible.
    pub(crate) fn enum_symbol_for_type(
        &self,
        ty: &Type,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        match ty {
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Enum => Some(*symbol),
            Type::Value { value } => {
                let inner = types.get_type(*value);
                self.enum_symbol_for_type(inner, types)
            }
            Type::Intersection { elements } => elements.iter().find_map(|element_id| {
                let element_ty = types.get_type(*element_id);
                self.enum_symbol_for_type(element_ty, types)
            }),
            _ => None,
        }
    }

    /// Build a union type from two type ids.
    pub(crate) fn union_type(
        &self,
        left: LocalTypeId,
        right: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // reuse the left source for the combined union
        self.union_type_from_list(vec![left, right], left, types)
    }

    /// Build a union type from a list of elements.
    pub(crate) fn union_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // flatten nested unions and keep elements unique
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Union { elements: union } => {
                    for element_id in union {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    if !flattened.contains(&element_id) {
                        flattened.push(element_id);
                    }
                }
            }
        }

        // collapse any or unknown and remove never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        for element_id in flattened {
            match types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        // honor dominating any or unknown
        if let Some(any_type) = any_type {
            return any_type;
        }
        if let Some(unknown_type) = unknown_type {
            return unknown_type;
        }

        // fall back to never when the union is empty
        if filtered.is_empty() {
            return never_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    types.get_type_source(source_type_id),
                )
            });
        }

        // avoid rebuilding when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // construct the union type
        let union = Type::Union { elements: filtered };
        types.insert_type_from_any(union, types.get_type_source(source_type_id))
    }

    /// Build an intersection type from a list of elements.
    pub(crate) fn intersection_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // flatten nested intersections and keep elements unique
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Intersection { elements } => {
                    for element_id in elements {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    if !flattened.contains(&element_id) {
                        flattened.push(element_id);
                    }
                }
            }
        }

        // collapse any or unknown and handle never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        for element_id in flattened {
            match types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        // honor dominating never or any
        if let Some(never_type) = never_type {
            return never_type;
        }
        if let Some(any_type) = any_type {
            return any_type;
        }

        // fall back to unknown when the intersection is empty
        if filtered.is_empty() {
            return unknown_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    types.get_type_source(source_type_id),
                )
            });
        }

        // avoid rebuilding when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // construct the intersection type
        let intersection = Type::Intersection { elements: filtered };
        types.insert_type_from_any(intersection, types.get_type_source(source_type_id))
    }

    /// Determine the runtime check kind for a type guard relation.
    pub(crate) fn runtime_check_kind_for_relation(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        value_type_id: LocalTypeId,
        target_type_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<RuntimeCheckKind> {
        // normalize apparent types before relation checks
        let value_type_id = self.normalize_apparent_type(
            module,
            profile,
            value_type_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::RUNTIME_GUARD,
        );
        let target_type_id = self.normalize_apparent_type(
            module,
            profile,
            target_type_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::RUNTIME_GUARD,
        );

        // constant true when the guard is already satisfied
        let is_assignable = self
            .is_type_assignable(
                module,
                profile,
                symbols,
                target_type_id,
                value_type_id,
                types,
                options,
            )
            .is_assignable();
        if is_assignable {
            return Some(RuntimeCheckKind::Constant(true));
        }

        // require a runtime checkable target type
        if !self.type_is_runtime_checkable_target(module, profile, symbols, target_type_id, types) {
            return None;
        }

        // decide which runtime identity the value carries
        self.runtime_check_kind_for_value_type(module, profile, symbols, value_type_id, types)
    }

    /// Check whether a target type can be validated at runtime.
    fn type_is_runtime_checkable_target(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> bool {
        // unwrap apparent types before inspection
        let type_id = self.normalize_apparent_type(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::RUNTIME_GUARD,
        );

        // accept unions when all members are runtime checkable
        let union_elements = match types.get_type(type_id) {
            Type::Union { elements } => Some(elements.clone()),
            _ => None,
        };
        if let Some(elements) = union_elements {
            for element_id in elements {
                if !self
                    .type_is_runtime_checkable_target(module, profile, symbols, element_id, types)
                {
                    return false;
                }
            }
            return true;
        }

        // accept nominal reference targets
        match types.get_type(type_id) {
            Type::Reference { symbol, .. } => matches!(
                symbol.local_id.ty,
                SymbolType::Class | SymbolType::Struct | SymbolType::Enum | SymbolType::Newtype
            ),
            _ => false,
        }
    }

    /// Determine the runtime identity carried by a value type.
    fn runtime_check_kind_for_value_type(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Option<RuntimeCheckKind> {
        // unwrap apparent types before inspection
        let type_id = self.normalize_apparent_type(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::RUNTIME_GUARD,
        );

        match types.get_type(type_id) {
            Type::Union { .. } => Some(RuntimeCheckKind::UnionTag),
            Type::Reference { symbol, .. } => match symbol.local_id.ty {
                SymbolType::Class | SymbolType::Struct | SymbolType::Enum | SymbolType::Newtype => {
                    Some(RuntimeCheckKind::TypeDescriptor)
                }
                _ => None,
            },
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            } => Some(RuntimeCheckKind::TypeDescriptor),
            Type::Value { value } => {
                self.runtime_check_kind_for_value_type(module, profile, symbols, *value, types)
            }
            _ => None,
        }
    }

    /// Check whether a type contains an error type.
    pub(crate) fn type_contains_error(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_error(visited);
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    pub(crate) fn type_contains_unevaluated_static_arguments(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_unevaluated_static(visited);
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    pub(crate) fn type_contains_unevaluated_value_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_unevaluated_value_static(
            self, module, profile, tree, symbols, types, visited,
        );
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    fn reference_contains_unevaluated_value_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        arguments: Option<&[StaticArgument]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(arguments) = arguments else {
            return false;
        };

        // no arguments means no value materialization is needed
        if arguments.is_empty() {
            return false;
        }

        // resolve parameter kinds for the referenced declaration
        let Some(parameter_symbols) =
            self.collect_static_parameter_symbols(module, symbol, profile, tree, symbols, types)
        else {
            return false;
        };

        for (index, argument) in arguments.iter().enumerate() {
            let kind = parameter_symbols
                .get(index)
                .map(|parameter_symbol| {
                    self.with_module_tree_symbols_or_local_at_stage(
                        module,
                        profile,
                        parameter_symbol.module_id,
                        tree,
                        symbols,
                        AnalyzeDependencyStage::Declare,
                        |_, owner_tree, owner_symbols| {
                            self.static_parameter_kind_for_symbol_in_module(
                                *parameter_symbol,
                                owner_tree,
                                owner_symbols,
                            )
                        },
                    )
                    .unwrap_or_else(|error| {
                        let _ = error;
                        StaticParameterKind::Type
                    })
                })
                .unwrap_or(StaticParameterKind::Type);

            // only value parameters require unevaluated materialization
            if kind != StaticParameterKind::Value {
                continue;
            }

            let has_unevaluated = match argument {
                StaticArgument::Unevaluated { .. } => true,
                StaticArgument::Evaluated { value, .. } => self
                    .static_expression_contains_unevaluated_static_arguments(value, types, visited),
            };
            if has_unevaluated {
                return true;
            }
        }

        false
    }

    /// Check whether a static expression contains unevaluated static arguments.
    fn static_expression_contains_unevaluated_static_arguments(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_unevaluated_static(visited);
        visitor.visit_static_expression(types, expression);
        visitor.found
    }
}
