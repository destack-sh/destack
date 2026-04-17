use std::collections::{HashMap, HashSet};

use destack_builtin::LanguageSymbol;
use destack_dir::{
    Block, Expression, Freshness, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalTypeId,
    NodeTree, NodeType, PrimitiveType, RuntimeCheckKind, ScalarLiteral, StaticArgument,
    StaticExpression, StaticKey, StaticParameterKind, SymbolType, Type, TypeLiteral, TypeTable,
    TypeVisitor, TypeVisitorOptions, are_types_equal, walk_static_argument, walk_static_expression,
    walk_type,
};
use destack_source::ModuleId;
use destack_workspace::Module;

use super::{
    CanonicalSymbolMode, InferContext, ModuleSymbolView, ModuleTypeView, NormalizationMode,
    RelationMode, SymbolTypeView, TreeSymbolView, TypeContext, TypeView, TypeWalkContext,
    TypeWalkKey,
};
use crate::{
    AnalyzeError, AnalyzeResult, Compiler, ElaborateError, ElaborateResult, InferState,
    StaticMemberSymbolKind,
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
    /// Detect unevaluated type slots, static arguments, or static expressions.
    UnevaluatedTypeState,
    /// Detect unevaluated static arguments or expressions.
    UnevaluatedStaticArgument,
    /// Detect unevaluated value static arguments.
    UnevaluatedValueStaticArgument {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The type ctx view for the current module.
        ctx: TypeView<'a>,
    },
    /// Detect static parameter usage.
    StaticParameter {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The symbol-and-type ctx view for the current module.
        ctx: SymbolTypeView<'a>,
    },
    /// Detect free static parameters.
    FreeStaticParameter {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The symbol-and-type ctx view for the current module.
        ctx: SymbolTypeView<'a>,
    },
    /// Detect conditional infer bindings.
    InferBinding,
    /// Detect inference variables.
    InferVar,
    /// Detect associated type references.
    AssociatedTypeReference {
        /// The compiler instance.
        compiler: &'a Compiler,
        /// The tree-and-symbol ctx view for the current module.
        ctx: TreeSymbolView<'a>,
    },
    /// Detect direct references to one symbol.
    ReferenceSymbol {
        /// The symbol to detect.
        symbol: GlobalSymbolId,
    },
    /// Detect `this` type references.
    ThisType,
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
        /// The module and type ctx view for this containment query.
        ctx: ModuleTypeView<'a>,
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

/// Collect type ids reachable from one root for freshness stamping.
struct TypeFreshnessCollector {
    /// The visited type ids.
    visited: HashSet<LocalTypeId>,
    /// The collected type ids in traversal order.
    type_ids: Vec<LocalTypeId>,
    /// The source node id used to scope freshness updates.
    root_source_id: LocalNodeIdAny,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl TypeFreshnessCollector {
    /// Create an empty freshness collector.
    fn new(root_source_id: LocalNodeIdAny) -> Self {
        Self {
            visited: HashSet::new(),
            type_ids: Vec::new(),
            root_source_id,
            options: base_visitor_options(),
        }
    }
}

impl TypeVisitor for TypeFreshnessCollector {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        if !self.visited.insert(id) {
            return;
        }

        // stamp only type slots owned by the same source node
        if types.get_type_source(id) == self.root_source_id {
            self.type_ids.push(id);
        }

        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        walk_type(self, types, id, ty);
    }
}

/// Collect unevaluated type ids reachable from one root.
struct TypeUnevaluatedCollector {
    /// The visited type ids.
    visited: HashSet<LocalTypeId>,
    /// The reachable unevaluated type ids.
    type_ids: Vec<LocalTypeId>,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl TypeUnevaluatedCollector {
    /// Create an empty unevaluated collector.
    fn new() -> Self {
        Self {
            visited: HashSet::new(),
            type_ids: Vec::new(),
            options: base_visitor_options(),
        }
    }
}

impl TypeVisitor for TypeUnevaluatedCollector {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        if !self.visited.insert(id) {
            return;
        }

        if matches!(types.get_type(id), Type::Unevaluated(_)) {
            self.type_ids.push(id);
        }

        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        walk_type(self, types, id, ty);
    }
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

    /// Create a visitor for unevaluated type-state detection.
    fn new_unevaluated_type_state(visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self::new(TypeContainmentKind::UnevaluatedTypeState, visited, None)
    }

    /// Create a visitor for unevaluated value static arguments.
    fn new_unevaluated_value_static(
        compiler: &'a Compiler,
        ctx: TypeView<'a>,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::UnevaluatedValueStaticArgument { compiler, ctx },
            visited,
            None,
        )
    }

    /// Create a visitor for static parameter detection.
    fn new_static_parameter(
        compiler: &'a Compiler,
        ctx: SymbolTypeView<'a>,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::StaticParameter { compiler, ctx },
            visited,
            None,
        )
    }

    /// Create a visitor for free static parameter detection.
    fn new_free_static_parameter(
        compiler: &'a Compiler,
        ctx: SymbolTypeView<'a>,
        bound: &HashSet<GlobalSymbolId>,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::FreeStaticParameter { compiler, ctx },
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

    /// Create a visitor for associated type reference containment.
    fn new_associated_type_reference(
        compiler: &'a Compiler,
        ctx: TreeSymbolView<'a>,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::AssociatedTypeReference { compiler, ctx },
            visited,
            None,
        )
    }

    /// Create a visitor for direct symbol-reference containment.
    fn new_reference_symbol(symbol: GlobalSymbolId, visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self::new(
            TypeContainmentKind::ReferenceSymbol { symbol },
            visited,
            None,
        )
    }

    /// Create a visitor for `this` type containment.
    fn new_this_type(visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self::new(TypeContainmentKind::ThisType, visited, None)
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
        ctx: ModuleTypeView<'a>,
        visited_types: &'a mut HashSet<LocalTypeId>,
        visited_symbols: &'a mut HashSet<GlobalSymbolId>,
    ) -> Self {
        Self::new(
            TypeContainmentKind::ManagedType { compiler, ctx },
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
            TypeContainmentKind::UnevaluatedTypeState => VisitedMode::Stack,
            TypeContainmentKind::UnevaluatedStaticArgument => VisitedMode::Stack,
            TypeContainmentKind::UnevaluatedValueStaticArgument { .. } => VisitedMode::Stack,
            TypeContainmentKind::StaticParameter { .. } => VisitedMode::Set,
            TypeContainmentKind::FreeStaticParameter { .. } => VisitedMode::Set,
            TypeContainmentKind::InferBinding => VisitedMode::Set,
            TypeContainmentKind::InferVar => VisitedMode::Set,
            TypeContainmentKind::AssociatedTypeReference { .. } => VisitedMode::Set,
            TypeContainmentKind::ReferenceSymbol { .. } => VisitedMode::Set,
            TypeContainmentKind::ThisType => VisitedMode::Set,
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
    fn take_free_static_bound(&mut self) -> Option<HashSet<GlobalSymbolId>> {
        self.free_static_bound.take()
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
            TypeContainmentKind::UnevaluatedTypeState => {
                if matches!(ty, Type::Unevaluated(_)) {
                    self.found = true;
                    return;
                }
            }
            TypeContainmentKind::UnevaluatedStaticArgument => {}
            TypeContainmentKind::UnevaluatedValueStaticArgument { compiler, ctx } => {
                if let Type::Reference {
                    symbol,
                    static_arguments,
                } = ty
                {
                    if compiler.reference_has_unevaluated_value_arguments(
                        *ctx,
                        *symbol,
                        static_arguments.as_deref(),
                        self.visited,
                    ) {
                        self.found = true;
                    }
                    return;
                }
            }
            TypeContainmentKind::StaticParameter { compiler, ctx } => match ty {
                Type::Reference { symbol, .. } => {
                    if compiler.symbol_is_static_parameter(*ctx, *symbol) {
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
            TypeContainmentKind::FreeStaticParameter { compiler, ctx } => match ty {
                Type::Reference {
                    symbol,
                    static_arguments,
                } => {
                    let Some(bound) = self.free_static_bound.as_ref() else {
                        self.found = true;
                        return;
                    };
                    if static_arguments.is_none()
                        && compiler.symbol_is_static_parameter(*ctx, *symbol)
                        && !bound.contains(symbol)
                    {
                        self.found = true;
                        return;
                    }
                }
                Type::Mapped {
                    parameter, value, ..
                } => {
                    let Some(mut bound) = self.take_free_static_bound() else {
                        self.found = true;
                        return;
                    };
                    let inserted = bound.insert(parameter.symbol);
                    self.restore_free_static_bound(bound);
                    self.visit_type_id(types, parameter.constraint);
                    if let Some(key_remap) = parameter.key_remap {
                        self.visit_type_id(types, key_remap);
                    }
                    self.visit_type_id(types, *value);
                    if inserted {
                        let Some(mut bound) = self.take_free_static_bound() else {
                            self.found = true;
                            return;
                        };
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
                        let Some(mut bound) = self.take_free_static_bound() else {
                            self.found = true;
                            return;
                        };
                        inserted = bound.insert(*distributive_symbol);
                        self.restore_free_static_bound(bound);
                    }

                    self.visit_type_id(types, *left);
                    self.visit_type_id(types, *right);
                    self.visit_type_id(types, *then_type);
                    self.visit_type_id(types, *else_type);

                    if inserted {
                        let Some(mut bound) = self.take_free_static_bound() else {
                            self.found = true;
                            return;
                        };
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
            TypeContainmentKind::AssociatedTypeReference { compiler, ctx } => {
                if let Type::Reference { symbol, .. } = ty
                    && compiler
                        .query_static_member_symbol_kind_for_symbol(*ctx, *symbol)
                        .ok()
                        == Some(Some(StaticMemberSymbolKind::AssociatedType))
                {
                    self.found = true;
                    return;
                }
            }
            TypeContainmentKind::ReferenceSymbol {
                symbol: target_symbol,
            } => {
                if let Type::Reference { symbol, .. } = ty
                    && symbol == target_symbol
                {
                    self.found = true;
                    return;
                }
            }
            TypeContainmentKind::ThisType => {
                if matches!(ty, Type::This) {
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
                Type::KeyOf { .. } => {
                    return;
                }
                Type::Reference { symbol, .. } => {
                    let skip_imported_types = self.skip_imported_types();
                    let Some(visited_symbols) = self.visited_symbols.as_deref_mut() else {
                        self.found = true;
                        return;
                    };
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
            TypeContainmentKind::ManagedType { compiler, ctx } => match ty {
                Type::ValueOf { .. } | Type::ReferenceOf { .. } | Type::PointerOf { .. } => {
                    return;
                }
                Type::Reference { symbol, .. } => {
                    let Some(visited_symbols) = self.visited_symbols.as_deref_mut() else {
                        self.found = true;
                        return;
                    };
                    if compiler.symbol_is_managed_inner(*ctx, *symbol, visited_symbols, false) {
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
            | TypeContainmentKind::UnevaluatedTypeState
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
            | TypeContainmentKind::UnevaluatedTypeState
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
    /// Return true when a type is still solver-owned placeholder ctx.
    pub(crate) fn type_is_solver_placeholder(&self, ty_id: LocalTypeId, types: &TypeTable) -> bool {
        types.get_type(ty_id).is_infer()
    }

    /// Return true when a type already represents a primary semantic failure.
    pub(crate) fn type_blocks_cascading_diagnostic(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        matches!(types.get_type(ty_id), Type::Error)
    }

    /// Emit one unassignable-type diagnostic unless either side already failed.
    pub(crate) fn emit_unassignable_type_for_types(
        &self,
        ctx: ModuleTypeView<'_>,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
    ) {
        let Some(error) =
            self.unassignable_type_error_for_types(ctx, node_id, expected_ty_id, actual_ty_id)
        else {
            return;
        };
        debug_assert!(error.is_cascading_semantic_diagnostic());
        self.error(error);
    }

    /// Build one unassignable-type error unless either side already failed.
    pub(crate) fn unassignable_type_error_for_types(
        &self,
        ctx: ModuleTypeView<'_>,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
    ) -> Option<AnalyzeError> {
        if self.type_blocks_cascading_diagnostic(expected_ty_id, ctx.types)
            || self.type_blocks_cascading_diagnostic(actual_ty_id, ctx.types)
        {
            return None;
        }

        Some(AnalyzeError::UnassignableType {
            node: node_id
                .into_global(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
            expected_ty: expected_ty_id.into_global(ctx.module.id),
            actual_ty: actual_ty_id.into_global(ctx.module.id),
        })
    }

    /// Report one unsatisfied-type diagnostic unless either side already failed.
    pub(crate) fn report_unsatisfied_type_for_types(
        &self,
        ctx: ModuleTypeView<'_>,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
    ) -> bool {
        let Some(error) =
            self.unsatisfied_type_error_for_types(ctx, node_id, expected_ty_id, actual_ty_id)
        else {
            return false;
        };
        debug_assert!(error.is_cascading_semantic_diagnostic());
        self.error(error);

        true
    }

    /// Build one unsatisfied-type error unless either side already failed.
    pub(crate) fn unsatisfied_type_error_for_types(
        &self,
        ctx: ModuleTypeView<'_>,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
    ) -> Option<AnalyzeError> {
        if self.type_blocks_cascading_diagnostic(expected_ty_id, ctx.types)
            || self.type_blocks_cascading_diagnostic(actual_ty_id, ctx.types)
        {
            return None;
        }

        Some(AnalyzeError::UnsatisfiedType {
            node: node_id
                .into_global(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
            expected_ty: expected_ty_id.into_global(ctx.module.id),
            actual_ty: actual_ty_id.into_global(ctx.module.id),
        })
    }

    /// Report one excess-property diagnostic unless the expected type already failed.
    pub(crate) fn report_excess_property_for_type(
        &self,
        ctx: ModuleTypeView<'_>,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        member_key: StaticKey,
    ) -> bool {
        let Some(error) =
            self.excess_property_error_for_type(ctx, node_id, expected_ty_id, member_key)
        else {
            return false;
        };
        debug_assert!(error.is_cascading_semantic_diagnostic());
        self.error(error);

        true
    }

    /// Build one excess-property error unless the expected type already failed.
    pub(crate) fn excess_property_error_for_type(
        &self,
        ctx: ModuleTypeView<'_>,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        member_key: StaticKey,
    ) -> Option<AnalyzeError> {
        if self.type_blocks_cascading_diagnostic(expected_ty_id, ctx.types) {
            return None;
        }

        Some(AnalyzeError::ExcessProperty {
            node: node_id
                .into_global(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
            expected_ty: expected_ty_id.into_global(ctx.module.id),
            member_key,
        })
    }

    /// Emit one no-overload diagnostic unless the receiver already failed.
    pub(crate) fn emit_no_overload_for_receiver_type(
        &self,
        ctx: ModuleTypeView<'_>,
        node_id: LocalNodeIdAny,
        receiver_ty_id: LocalTypeId,
    ) {
        if self.type_blocks_cascading_diagnostic(receiver_ty_id, ctx.types) {
            return;
        }

        let error = AnalyzeError::NoOverload {
            node: node_id
                .into_global(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
            receiver_ty: receiver_ty_id.into_global(ctx.module.id),
        };
        debug_assert!(error.is_cascading_semantic_diagnostic());
        self.error(error);
    }

    /// Emit one non-callable diagnostic unless the callee already failed.
    pub(crate) fn emit_non_callable_for_callee_type(
        &self,
        ctx: ModuleTypeView<'_>,
        node_id: LocalNodeIdAny,
        callee_ty_id: LocalTypeId,
    ) {
        if self.type_blocks_cascading_diagnostic(callee_ty_id, ctx.types) {
            return;
        }

        let error = AnalyzeError::NonCallable {
            node: node_id
                .into_global(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
        };
        debug_assert!(error.is_cascading_semantic_diagnostic());
        self.error(error);
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

    /// Set freshness for reachable type slots owned by one source node.
    pub(crate) fn set_type_freshness(
        &self,
        types: &mut TypeTable,
        type_id: LocalTypeId,
        freshness: Freshness,
    ) {
        let root_source_id = types.get_type_source(type_id);
        let mut collector = TypeFreshnessCollector::new(root_source_id);
        collector.visit_type_id(types, type_id);

        for id in collector.type_ids {
            types.set_type_freshness(id, freshness);
        }
    }

    /// Apply inference-state freshness to one inferred type.
    pub(crate) fn apply_infer_state_type_freshness(
        &self,
        types: &mut TypeTable,
        type_id: LocalTypeId,
        state: &InferState,
    ) {
        self.set_type_freshness(types, type_id, state.type_freshness());
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
        let block_type_id = if let Some(last_expression_id) = block.last_expression() {
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
            let Expression::Block(block) = tree.get(expression_id) else {
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
        ctx: SymbolTypeView<'_>,
        symbol: GlobalSymbolId,
    ) -> bool {
        // honor cached constraints for mapped parameters
        if ctx
            .types
            .get_static_parameter_constraint_type(symbol)
            .is_some()
        {
            return true;
        }

        // rely on the declared parameter metadata
        let Some(symbol) = self
            .with_module_symbols_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol.module_id,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
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
        ctx: TypeView<'_>,
        type_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor =
            TypeContainmentVisitor::new_static_parameter(self, ctx.symbol_type_view(), visited);
        visitor.visit_type_id(ctx.types, type_id);
        visitor.found
    }

    /// Check whether a type contains free static parameter references.
    pub(crate) fn type_contains_free_static_parameters(
        &self,
        ctx: TypeView<'_>,
        type_id: LocalTypeId,
        bound: &HashSet<GlobalSymbolId>,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_free_static_parameter(
            self,
            ctx.symbol_type_view(),
            bound,
            visited,
        );
        visitor.visit_type_id(ctx.types, type_id);
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

    /// Check whether a type contains `this`.
    pub(crate) fn type_contains_this(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_this_type(visited);
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Return true when one type satisfies one interface symbol.
    fn type_implements_interface_symbol(
        &self,
        ctx: SymbolTypeView<'_>,
        ty: &Type,
        interface_symbol: GlobalSymbolId,
    ) -> bool {
        // resolve declared nominal references directly
        if let Type::Reference { symbol, .. } = ty {
            let canonical_symbol = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                *symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            return self.is_type_lineage_assignable(ctx, canonical_symbol, interface_symbol);
        }

        // resolve well known wrappers for array-like receivers
        let well_known_symbol = match ty {
            Type::Array { .. } | Type::ArraySized { .. } | Type::Tuple { .. } => {
                self.well_known_symbol_for_type(ty, ctx.types)
            }
            _ => None,
        };
        let Some(well_known_symbol) = well_known_symbol else {
            return false;
        };
        let Some(well_known_type_symbol) =
            self.get_well_known_type_symbol(ctx.profile, well_known_symbol)
        else {
            return false;
        };

        let canonical_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            well_known_type_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        self.is_type_lineage_assignable(ctx, canonical_symbol, interface_symbol)
    }

    /// Return true when one type satisfies one language interface item.
    pub(crate) fn is_interface_implemented(
        &self,
        ctx: SymbolTypeView<'_>,
        ty: &Type,
        interface_item: LanguageSymbol,
    ) -> bool {
        let interface_symbol = self.language_symbol(ctx.profile, interface_item);
        match ty {
            Type::Value { value } => {
                let inner_ty = ctx.types.get_type(*value);
                self.is_interface_implemented(ctx, inner_ty, interface_item)
            }
            Type::Union { elements } => elements.iter().all(|element_id| {
                let element_ty = ctx.types.get_type(*element_id);
                self.is_interface_implemented(ctx, element_ty, interface_item)
            }),
            _ => self.type_implements_interface_symbol(ctx, ty, interface_symbol),
        }
    }

    /// Check whether a type requires infer convergence before stable checking.
    pub(crate) fn type_requires_infer_convergence(
        &self,
        ctx: TypeView<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        // inference variables are not stable yet
        let mut infer_var_visited = HashSet::new();
        if self.type_contains_infer_vars(type_id, ctx.types, &mut infer_var_visited) {
            return true;
        }

        // unevaluated value wrappers are not stable yet
        if self.unwrapped_value_type_is_unevaluated(type_id, ctx.types) {
            return true;
        }

        // unevaluated static arguments are not stable yet
        let mut static_argument_visited = HashSet::new();
        if self.type_has_unevaluated_static_arguments(
            type_id,
            ctx.types,
            &mut static_argument_visited,
        ) {
            return true;
        }

        false
    }

    /// Ensure a type id is evaluated when it is unevaluated.
    pub(crate) fn ensure_type_evaluated(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> AnalyzeResult<LocalTypeId> {
        if matches!(ctx.types.get_type(type_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(&mut ctx.reborrow(), type_id)?;
        }
        Ok(type_id)
    }

    /// Ensure one unwrapped value type id is evaluated before runtime and member reads.
    pub(crate) fn ensure_unwrapped_value_type_evaluated(
        &self,
        ctx: &mut InferContext<'_>,
        type_id: LocalTypeId,
    ) -> AnalyzeResult<LocalTypeId> {
        let unwrapped_type_id = ctx.types.unwrap_value_type_id(type_id);
        self.ensure_type_evaluated(&mut ctx.type_context_reborrow(), unwrapped_type_id)
    }

    /// Return true when one unwrapped value type id remains unevaluated.
    pub(crate) fn unwrapped_value_type_is_unevaluated(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let unwrapped_type_id = types.unwrap_value_type_id(type_id);
        matches!(types.get_type(unwrapped_type_id), Type::Unevaluated(_))
    }

    /// Unwrap one value type id and strip nested `as comptime` wrappers.
    pub(crate) fn unwrapped_value_without_as_comptime_type_id(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> LocalTypeId {
        let mut unwrapped_type_id = types.unwrap_value_type_id(type_id);
        while let Type::AsComptime { target_type: right } = types.get_type(unwrapped_type_id) {
            unwrapped_type_id = *right;
        }
        unwrapped_type_id
    }

    /// Return true when one unwrapped value type id is indeterminate for concrete runtime checks.
    pub(crate) fn unwrapped_value_type_is_indeterminate(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let unwrapped_type_id = types.unwrap_value_type_id(type_id);
        if matches!(types.get_type(unwrapped_type_id), Type::Unevaluated(_)) {
            return true;
        }
        if matches!(
            types.get_type(unwrapped_type_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            }
        ) {
            return true;
        }
        if matches!(types.get_type(unwrapped_type_id), Type::Error) {
            return true;
        }

        self.type_is_solver_placeholder(unwrapped_type_id, types)
    }

    /// Resolve the owning enum symbol for one enum-field symbol.
    pub(crate) fn enum_symbol_for_field_symbol(
        &self,
        ctx: SymbolTypeView<'_>,
        field_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // enum-field ownership is module-local metadata
        if field_symbol.module_id != ctx.module.id {
            let owner_dir = self
                .require_artifact_dir_declared(
                    ctx.compiler_context.revision(),
                    field_symbol.module_id,
                    ctx.profile,
                )
                .map_err(AnalyzeError::from)?;
            let field_entry = owner_dir.symbols.get_symbol(field_symbol.local_id);
            let Some(primary) = field_entry.primary_declaration else {
                return Ok(None);
            };
            if primary.local_id.ty != NodeType::EnumField {
                return Ok(None);
            }

            let scope = owner_dir.symbols.get_scope_by_symbol(field_symbol.local_id);
            let Some(owner_id) = scope.owner_id else {
                return Ok(None);
            };
            let owner_symbol = owner_id.into_global(field_symbol.module_id);
            return Ok((owner_symbol.ty() == SymbolType::Enum).then_some(owner_symbol));
        }

        let field_entry = ctx.symbols.get_symbol(field_symbol.local_id);
        let Some(primary) = field_entry.primary_declaration else {
            return Ok(None);
        };
        if primary.local_id.ty != NodeType::EnumField {
            return Ok(None);
        }

        let scope = ctx.symbols.get_scope_by_symbol(field_symbol.local_id);
        let Some(owner_id) = scope.owner_id else {
            return Ok(None);
        };
        let owner_symbol = owner_id.into_global(ctx.module.id);

        Ok((owner_symbol.ty() == SymbolType::Enum).then_some(owner_symbol))
    }

    /// Query the owning enum symbol for one enum-field symbol.
    pub(crate) fn query_enum_symbol_for_field_symbol(
        &self,
        ctx: SymbolTypeView<'_>,
        field_symbol: GlobalSymbolId,
    ) -> Option<GlobalSymbolId> {
        match self.enum_symbol_for_field_symbol(ctx, field_symbol) {
            Ok(enum_symbol) => enum_symbol,
            Err(error) => {
                self.error(error);
                None
            }
        }
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

    /// Collect static parameter symbols from one static-parameter type-id sequence.
    pub(crate) fn static_parameter_symbols_for_type_ids(
        &self,
        static_parameter_type_ids: &[LocalTypeId],
        types: &TypeTable,
    ) -> Vec<GlobalSymbolId> {
        static_parameter_type_ids
            .iter()
            .filter_map(|parameter_type_id| {
                self.unwrap_type_value_symbol(types, *parameter_type_id)
            })
            .collect()
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
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        let mut current_symbol =
            self.canonical_symbol_id(view, symbol, CanonicalSymbolMode::FollowAliases);
        let mut visited = HashSet::new();

        // chase forwarding edges with cycle protection
        loop {
            if !visited.insert(current_symbol) {
                break;
            }

            let next_symbol = self
                .with_module_symbols_or_local_for_artifact(
                    view.compiler_context,
                    view.module,
                    view.profile,
                    current_symbol.module_id,
                    view.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
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

            let next_symbol =
                self.canonical_symbol_id(view, next_symbol, CanonicalSymbolMode::FollowAliases);
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
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> Option<LocalTypeId> {
        self.require_alias_target_type_id_for_symbol(ctx, symbol, source_id)
            .unwrap_or_default()
    }

    /// Import the alias target type for a symbol when available, yielding on unmet requirements.
    pub(crate) fn require_alias_target_type_id_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let mut visited = HashSet::new();
        let mut current = symbol;

        loop {
            if !visited.insert(current) {
                let node = source_id
                    .into_global(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::RecursiveTypeInstantiation { node });

                let error_id = ctx.types.insert_type_from_any(Type::Error, source_id);
                return Ok(Some(error_id));
            }

            // load the local alias target when the symbol is local
            if current.module_id == ctx.module.id {
                let symbol_entry = ctx.symbols.get_symbol(current.local_id);
                let typed_symbol = GlobalSymbolId::new(
                    current.module_id,
                    current.local_id.with_type(symbol_entry.ty),
                );
                let declared_symbol = self
                    .declaration_symbol_id(ctx.module_symbol_view(), current)
                    .unwrap_or(typed_symbol);

                // check the incoming symbol first, then the declaration-typed symbol
                ctx.types.record_normalization_symbol_dependency(current);
                if let Some(target) = ctx.types.get_alias_target_type_id(current) {
                    return Ok(Some(target));
                }

                ctx.types
                    .record_normalization_symbol_dependency(declared_symbol);
                if let Some(target) = ctx.types.get_alias_target_type_id(declared_symbol) {
                    return Ok(Some(target));
                }

                if matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                    ctx.types
                        .record_normalization_symbol_dependency(typed_symbol);
                    if let Some(target) = ctx.types.get_alias_target_type_id(typed_symbol) {
                        return Ok(Some(target));
                    }
                }

                // follow import targets for local alias references
                let Some(target_symbol) =
                    symbol_entry.target_symbol.or(symbol_entry.canonical_symbol)
                else {
                    return Ok(None);
                };
                current = target_symbol;
                continue;
            }

            // import the declared alias target when the symbol is remote
            let (dependency_symbol, remote_alias_target, next) = self
                .remote_declared_alias_target(ctx.compiler_context, ctx.profile, current)
                .map_err(AnalyzeError::from)?;
            if let Some(dependency_symbol) = dependency_symbol {
                ctx.types
                    .record_normalization_symbol_dependency(dependency_symbol);
            }
            if let Some((typed_symbol, remote_target_ty, remote_snapshot)) = remote_alias_target {
                let local_alias_target_id = self.import_remote_type_for_node(
                    source_id,
                    &remote_target_ty,
                    &remote_snapshot,
                    ctx.types,
                );
                ctx.types
                    .set_alias_target_type_id(typed_symbol, local_alias_target_id);
                return Ok(Some(local_alias_target_id));
            }
            let Some(next) = next else {
                return Ok(None);
            };
            current = next;
        }
    }

    /// Query a committed alias target type for a symbol without triggering remote evaluation.
    pub(crate) fn committed_alias_target_type_id_for_symbol(
        &self,
        ctx: SymbolTypeView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        let mut visited = HashSet::new();
        let mut current = symbol;

        loop {
            // guard alias forwarding cycles
            if !visited.insert(current) {
                return None;
            }

            // resolve committed alias targets from the local table first
            if let Some(target) = ctx.types.get_alias_target_type_id(current) {
                return Some(target);
            }

            if current.module_id == ctx.module.id {
                let symbol_entry = ctx.symbols.get_symbol(current.local_id);
                let typed_symbol = GlobalSymbolId::new(
                    current.module_id,
                    current.local_id.with_type(symbol_entry.ty),
                );

                if matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype)
                    && let Some(target) = ctx.types.get_alias_target_type_id(typed_symbol)
                {
                    return Some(target);
                }

                let target_symbol = symbol_entry
                    .target_symbol
                    .or(symbol_entry.canonical_symbol)?;
                current = target_symbol;
                continue;
            }

            // read one remote symbol edge under declared gating
            let (typed_symbol, next) = match self.with_module_symbols_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                current.module_id,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |_owner_module, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(current.local_id);
                    let typed_symbol = GlobalSymbolId::new(
                        current.module_id,
                        current.local_id.with_type(symbol_entry.ty),
                    );
                    let next = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
                    (typed_symbol, next)
                },
            ) {
                Ok(value) => value,
                Err(_) => return None,
            };

            // read one committed declared alias target from the owner type table
            if matches!(
                typed_symbol.ty(),
                SymbolType::TypeAlias | SymbolType::Newtype
            ) {
                let resolved = self
                    .with_module_types_or_local_for_artifact(
                        ctx.compiler_context,
                        ctx.module,
                        ctx.profile,
                        current.module_id,
                        ctx.types,
                        destack_artifact::ArtifactKey::dir_declared,
                        |_owner_module, owner_types| {
                            owner_types.get_alias_target_type_id(typed_symbol)
                        },
                    )
                    .ok()
                    .flatten();
                if let Some(resolved) = resolved {
                    return Some(resolved);
                }
            }

            current = next?;
        }
    }

    /// Require an instance type for a symbol into the local type table.
    pub(crate) fn require_instance_type(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        // resolve through canonical import targets while preserving aliases
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // reuse local instance types when already available
        ctx.types.record_normalization_symbol_dependency(symbol);
        if let Some(instance_id) = ctx.types.get_instance_type_id(symbol) {
            return Some(instance_id);
        }

        // load or import the instance type through the existing resolver
        match self.resolve_instance_type_for_symbol(&mut ctx.reborrow(), source_id, symbol) {
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
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        // resolve through canonical import targets while preserving aliases
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // only aliases and newtypes expose apparent type through alias targets
        if matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype)
            && let Some(alias_target_id) =
                self.alias_target_type_id_for_symbol(&mut ctx.reborrow(), symbol, source_id)
        {
            return Some(alias_target_id);
        }

        // otherwise fall back to the instance type
        self.require_instance_type(&mut ctx.reborrow(), source_id, symbol)
    }

    /// Resolve one specialized instance surface for a reference.
    pub(crate) fn specialized_instance_type_for_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
    ) -> Option<LocalTypeId> {
        // preserve alias identity while resolving the underlying surface
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // aliases and newtypes expose their apparent target directly
        if matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype) {
            let apparent = self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)?;
            let apparent = self.materialize_reference_static_arguments_in_instance_type(
                &mut ctx.reborrow(),
                source_id,
                symbol,
                apparent,
                static_arguments,
            );
            return Some(apparent);
        }

        // start from the committed instance surface
        let instance_id = self.require_instance_type(&mut ctx.reborrow(), source_id, symbol)?;
        let instance_id = self.materialize_reference_static_arguments_in_instance_type(
            &mut ctx.reborrow(),
            source_id,
            symbol,
            instance_id,
            static_arguments,
        );

        Some(instance_id)
    }

    /// Apply explicit reference static arguments to one instance type.
    fn materialize_reference_static_arguments_in_instance_type(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        instance_id: LocalTypeId,
        static_arguments: Option<&[StaticArgument]>,
    ) -> LocalTypeId {
        let Some(static_arguments) = static_arguments else {
            return instance_id;
        };
        if static_arguments.is_empty() {
            return instance_id;
        }

        // resolve static arguments before substitution
        let resolved_arguments = if static_arguments
            .iter()
            .all(|argument| matches!(argument, StaticArgument::Evaluated { .. }))
        {
            Some(static_arguments.to_vec())
        } else {
            self.resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                source_id,
                symbol,
                Some(static_arguments),
                true,
            )
            .unwrap_or_else(|error| {
                self.error(error);
                None
            })
        };
        let Some(resolved_arguments) = resolved_arguments else {
            return instance_id;
        };
        if resolved_arguments.is_empty() {
            return instance_id;
        }

        // substitute resolved arguments into the instance surface
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.reborrow(),
            symbol,
            source_id,
            &resolved_arguments,
        );
        if substitutions.is_empty() {
            return instance_id;
        }

        let mut cache = HashMap::new();
        self.substitute_static_parameters(instance_id, &substitutions, ctx.types, &mut cache)
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

    /// Build a union type from a list of elements.
    pub(crate) fn union_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // flatten nested unions and keep structurally unique elements
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Union { elements: union } => {
                    for element_id in union {
                        if !flattened
                            .iter()
                            .any(|existing| are_types_equal(*existing, *element_id, types))
                        {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    if !flattened
                        .iter()
                        .any(|existing| are_types_equal(*existing, element_id, types))
                    {
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

        let filtered =
            self.collapse_exhaustive_simple_literal_union_members(filtered, source_type_id, types);

        // drop literal members that are already covered by one simple primitive
        let mut simplified = Vec::new();
        for element_id in filtered {
            if self.union_element_is_subsumed_by_simple_primitive_member(
                element_id,
                &simplified,
                types,
            ) {
                continue;
            }

            simplified.retain(|existing| {
                !self.union_element_is_subsumed_by_simple_primitive_member(
                    *existing,
                    &[element_id],
                    types,
                )
            });
            simplified.push(element_id);
        }

        // honor dominating any or unknown
        if let Some(any_type) = any_type {
            return any_type;
        }
        if let Some(unknown_type) = unknown_type {
            return unknown_type;
        }

        // fall back to never when the union is empty
        if simplified.is_empty() {
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
        if simplified.len() == 1 {
            return simplified[0];
        }

        // construct the union type
        let union = Type::Union {
            elements: simplified,
        };
        types.insert_type_from_any(union, types.get_type_source(source_type_id))
    }

    /// Return whether one union member is covered by one simple primitive member.
    pub(crate) fn union_element_is_subsumed_by_simple_primitive_member(
        &self,
        element_id: LocalTypeId,
        members: &[LocalTypeId],
        types: &TypeTable,
    ) -> bool {
        let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        } = types.get_type(element_id)
        else {
            return false;
        };

        members.iter().copied().any(|member_id| {
            matches!(
                types.get_type(member_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(primitive),
                } if self.scalar_literal_is_covered_by_primitive_member(literal, *primitive)
            )
        })
    }

    /// Collapse exhaustive finite literal unions to one primitive member.
    pub(crate) fn collapse_exhaustive_simple_literal_union_members(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Vec<LocalTypeId> {
        let mut has_boolean_primitive = false;
        let mut has_true_literal = false;
        let mut has_false_literal = false;

        for element_id in &elements {
            match types.get_type(*element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                } => has_boolean_primitive = true,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true)),
                } => has_true_literal = true,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(false)),
                } => has_false_literal = true,
                _ => {}
            }
        }

        if has_boolean_primitive || !has_true_literal || !has_false_literal {
            return elements;
        }

        let boolean_type_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },
            types.get_type_source(source_type_id),
        );

        let mut collapsed = Vec::with_capacity(elements.len().saturating_sub(1));
        let mut inserted_boolean = false;

        for element_id in elements {
            match types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(_)),
                } => {
                    if !inserted_boolean {
                        collapsed.push(boolean_type_id);
                        inserted_boolean = true;
                    }
                }
                _ => collapsed.push(element_id),
            }
        }

        collapsed
    }

    /// Return whether one scalar literal is covered by one primitive member.
    fn scalar_literal_is_covered_by_primitive_member(
        &self,
        literal: &ScalarLiteral,
        primitive: PrimitiveType,
    ) -> bool {
        match literal {
            ScalarLiteral::Null => false,
            // these literal families do not need module specific widening rules
            ScalarLiteral::Boolean(_) => primitive == PrimitiveType::Boolean,
            ScalarLiteral::Bigint(_) => primitive == PrimitiveType::Bigint,
            ScalarLiteral::String(_) | ScalarLiteral::RegexString { .. } => {
                primitive == PrimitiveType::String
            }

            // numeric literals depend on language and context
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) | ScalarLiteral::Character(_) => {
                false
            }
        }
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
        ctx: &mut TypeContext<'_>,
        value_type_id: LocalTypeId,
        target_type_id: LocalTypeId,
    ) -> Option<RuntimeCheckKind> {
        // normalize apparent types before relation checks
        let value_type_id = self.normalize_apparent_type(
            &mut ctx.reborrow(),
            value_type_id,
            NormalizationMode::Assign,
            RelationMode::RUNTIME_GUARD,
        );
        let target_type_id = self.normalize_apparent_type(
            &mut ctx.reborrow(),
            target_type_id,
            NormalizationMode::Assign,
            RelationMode::RUNTIME_GUARD,
        );

        // constant true when the guard is already satisfied
        let is_assignable = self
            .is_type_assignable(&mut ctx.reborrow(), target_type_id, value_type_id)
            .is_assignable();
        if is_assignable {
            return Some(RuntimeCheckKind::Constant(true));
        }

        // require a runtime checkable target type
        if !self.type_is_runtime_checkable_target(&mut ctx.reborrow(), target_type_id) {
            return None;
        }

        // decide which runtime identity the value carries
        self.runtime_check_kind_for_value_type(&mut ctx.reborrow(), value_type_id)
    }

    /// Check whether a target type can be validated at runtime.
    fn type_is_runtime_checkable_target(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        // unwrap apparent types before inspection
        let type_id = self.normalize_apparent_type(
            &mut ctx.reborrow(),
            type_id,
            NormalizationMode::Assign,
            RelationMode::RUNTIME_GUARD,
        );

        // accept unions when all members are runtime checkable
        let union_elements = match ctx.types.get_type(type_id) {
            Type::Union { elements } => Some(elements.clone()),
            _ => None,
        };
        if let Some(elements) = union_elements {
            for element_id in elements {
                if !self.type_is_runtime_checkable_target(&mut ctx.reborrow(), element_id) {
                    return false;
                }
            }
            return true;
        }

        // accept nominal reference targets
        match ctx.types.get_type(type_id) {
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
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> Option<RuntimeCheckKind> {
        // unwrap apparent types before inspection
        let type_id = self.normalize_apparent_type(
            &mut ctx.reborrow(),
            type_id,
            NormalizationMode::Assign,
            RelationMode::RUNTIME_GUARD,
        );

        let ty = ctx.types.get_type(type_id).clone();
        match ty {
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
                self.runtime_check_kind_for_value_type(&mut ctx.reborrow(), value)
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

    /// Collect the reachable unevaluated type ids from one root type.
    pub(crate) fn collect_unevaluated_type_ids(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Vec<LocalTypeId> {
        let mut collector = TypeUnevaluatedCollector::new();
        collector.visit_type_id(types, ty_id);
        collector.type_ids
    }

    pub(crate) fn type_has_unevaluated_static_arguments(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_unevaluated_static(visited);
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    /// Check whether a type contains unevaluated type slots or static state.
    pub(crate) fn type_has_unevaluated_state(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_unevaluated_type_state(visited);
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    pub(crate) fn type_contains_associated_type_reference(
        &self,
        ctx: TreeSymbolView<'_>,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_associated_type_reference(self, ctx, visited);
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    /// Check whether a type contains a direct reference to one symbol.
    pub(crate) fn type_contains_reference_symbol(
        &self,
        ty_id: LocalTypeId,
        symbol: GlobalSymbolId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_reference_symbol(symbol, visited);
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    pub(crate) fn type_has_unevaluated_value_static_arguments(
        &self,
        ctx: TypeView<'_>,
        ty_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = TypeContainmentVisitor::new_unevaluated_value_static(self, ctx, visited);
        visitor.visit_type_id(ctx.types, ty_id);
        visitor.found
    }

    fn reference_has_unevaluated_value_arguments(
        &self,
        ctx: TypeView<'_>,
        symbol: GlobalSymbolId,
        arguments: Option<&[StaticArgument]>,
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
        let Some(parameter_symbols) = self.collect_static_parameter_symbols(ctx, symbol) else {
            return false;
        };

        for (index, argument) in arguments.iter().enumerate() {
            let kind = parameter_symbols
                .get(index)
                .map(|parameter_symbol| {
                    self.with_module_tree_symbol_view_or_local_for_artifact(
                        ctx.compiler_context,
                        ctx.module,
                        ctx.profile,
                        parameter_symbol.module_id,
                        ctx.tree,
                        ctx.symbols,
                        destack_artifact::ArtifactKey::dir_declared,
                        |view| {
                            self.static_parameter_metadata_for_symbol_in_module(
                                view,
                                *parameter_symbol,
                            )
                            .0
                        },
                    )
                    .ok()
                    .unwrap_or(StaticParameterKind::Type)
                })
                .unwrap_or(StaticParameterKind::Type);

            // only value parameters require unevaluated materialization
            if kind != StaticParameterKind::Value {
                continue;
            }

            let has_unevaluated = match argument {
                StaticArgument::Unevaluated { .. } => true,
                StaticArgument::Evaluated { value, .. } => self
                    .static_expression_has_unevaluated_static_arguments(value, ctx.types, visited),
            };
            if has_unevaluated {
                return true;
            }
        }

        false
    }

    /// Check whether a static expression contains unevaluated static arguments.
    fn static_expression_has_unevaluated_static_arguments(
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
