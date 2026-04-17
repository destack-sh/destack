use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::declaration::DeclaratorConstraint;

use crate::analyze::StaticSubstitutionEnvironment;
use crate::analyze::common::{
    CanonicalSymbolMode, ConstContext, ContextualTypingMode, FreshnessMode, InferContext,
    ModuleSymbolView, RelationMode, TreeSymbolView, TypeContext, TypeRewriteCache, WideningMode,
};
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, AnalyzeWarning, Assignability, BreakTargetKind,
    Compiler, FlowContext, InferState,
};
use destack_artifact::ModuleEdgeRelation;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    Addressability, Argument, Asynchrony, Block, Constraint, Declaration, DependencyItem,
    DependencyKind, DependencyMode, Expression, FlowGraphBuilder, ForEachBinding, ForEachKind,
    Freshness, FunctionCardinality, FunctionKind, FunctionMode, GenericArgument, GlobalNodeIdAny,
    GlobalSymbolId, IfCondition, ImportSource, ImportTarget, InferOrigin, InferScope, LocalNodeId,
    LocalNodeIdAny, LocalSymbolId, LocalTypeId, LoopKind, MatchCase, MatchKind, MatchSelector,
    MatchSource, Member, NodeTree, NodeType, NormalizationMode, Pattern, PrimitiveType, Property,
    Resolution, ResolvedSignature, ScalarLiteral, StaticKey, StringId, SymbolDecorators,
    SymbolSpace, Type, TypeElement, TypeExpression, TypeField, TypeLiteral,
    TypeRelationObligationDiagnostic, TypeTable, WellKnownSymbol, YieldCardinality,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId, Revision};

/// Object literal field metadata for excess property checks.
#[derive(Debug, Clone)]
pub(crate) struct ObjectLiteralField {
    /// The field of the object literal.
    field: TypeField,
    /// The corresponding property of the object literal.
    property_id: LocalNodeId<Property>,
}

impl ObjectLiteralField {
    /// Return the object literal field type information.
    pub(crate) fn field(&self) -> &TypeField {
        &self.field
    }
}

/// Optional chain receiver metadata.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OptionalChainReceiver {
    /// The receiver expression id.
    pub(crate) receiver_id: LocalNodeId<Expression>,
    /// The non-nullish receiver type id, when available.
    pub(crate) receiver_ty_id: Option<LocalTypeId>,
    /// Whether the receiver included nullish types.
    pub(crate) has_nullish: bool,
}

/// Lexical home-object kinds that allow super access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuperHomeObjectKind {
    /// A class method home object.
    ClassMethod { is_constructor: bool },
    /// A class field home object.
    ClassField,
    /// A class static block home object.
    ClassStaticBlock,
    /// An object method home object.
    ObjectMethod,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Report pre-infer expression form diagnostics.
    pub(crate) fn report_pre_infer_expression_form_diagnostics(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // cache strict mode once per expression
        let is_strict = ctx.module.source_type.is_module() || ctx.options.always_strict;

        match expression {
            Expression::Assign { left, .. } | Expression::AssignBinary { left, .. } => {
                self.validate_assignment_target(&ctx.reborrow(), *left, is_strict);
            }
            Expression::Super => {
                self.validate_super_reference_expression(&ctx.reborrow(), expression_id);
            }
            Expression::Call { left, .. } => {
                self.validate_super_call_expression(&ctx.reborrow(), expression_id, *left);
                self.validate_super_property_expression(&ctx.reborrow(), expression_id);
            }
            Expression::New { left, .. } => {
                self.validate_new_optional_chain_expression(&ctx.reborrow(), expression_id, *left);
                self.validate_super_property_expression(&ctx.reborrow(), expression_id);
            }
            Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. } => {
                self.validate_instantiation_access(&ctx.reborrow(), expression_id);
                self.validate_super_property_expression(&ctx.reborrow(), expression_id);
            }
            Expression::Maybe { left } => {
                self.validate_super_optional_chain(&ctx.reborrow(), expression_id, *left);
            }
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => {
                self.validate_strict_reserved_identifier_reference(
                    &ctx.reborrow(),
                    expression_id,
                    path,
                    is_strict,
                );
            }
            _ => {}
        }
    }

    /// Return true when `super.x` is valid in the current lexical context.
    pub(crate) fn super_property_is_valid_context(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        self.super_home_object_for_context(tree, expression_id, true)
            .is_some()
    }

    /// Return true when `super(...)` is valid in the current lexical context.
    pub(crate) fn super_call_is_valid_context(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // super calls require a constructor method home object
        let Some(home_object) = self.super_home_object_for_context(tree, expression_id, false)
        else {
            return false;
        };
        if !matches!(
            home_object,
            SuperHomeObjectKind::ClassMethod {
                is_constructor: true
            }
        ) {
            return false;
        }

        // super calls require an enclosing derived class
        self.enclosing_class_is_derived(tree, expression_id)
    }

    /// Return the nearest lexical home object that can bind `super`.
    fn super_home_object_for_context(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        allow_lambda_boundaries: bool,
    ) -> Option<SuperHomeObjectKind> {
        let mut current = Some(expression_id.into_any());

        // walk outward through parent scopes
        while let Some(node_id) = current {
            let Some(parent) = tree.get_parent(node_id.id) else {
                break;
            };

            // non-lambda functions stop lexical super inheritance
            if parent.ty == NodeType::Declaration {
                let declaration = tree.get(parent.into_typed::<Declaration>());
                if let Declaration::Function(declaration) = declaration {
                    if allow_lambda_boundaries && declaration.signature.kind == FunctionKind::Lambda
                    {
                        current = Some(parent);
                        continue;
                    }
                    return None;
                }
            }

            // class member contexts provide a home object
            if parent.ty == NodeType::Member {
                let member = tree.get(parent.into_typed::<Member>());
                match member {
                    Member::Method { signature, .. } => {
                        let is_constructor = signature.mode == Some(FunctionMode::Constructor);
                        return Some(SuperHomeObjectKind::ClassMethod { is_constructor });
                    }
                    Member::Field { .. } => return Some(SuperHomeObjectKind::ClassField),
                    Member::StaticBlock { .. } => {
                        return Some(SuperHomeObjectKind::ClassStaticBlock);
                    }
                    Member::AssociatedType { .. }
                    | Member::AssociatedConst { .. }
                    | Member::Embed { .. }
                    | Member::ComptimeBlock { .. }
                    | Member::Error { .. } => {
                        return None;
                    }
                }
            }

            // object method contexts provide a home object
            if parent.ty == NodeType::Property {
                let property = tree.get(parent.into_typed::<Property>());
                match property {
                    Property::Method { .. } => return Some(SuperHomeObjectKind::ObjectMethod),
                    Property::Field { .. } | Property::Spread { .. } | Property::Error { .. } => {
                        return None;
                    }
                }
            }

            current = Some(parent);
        }

        None
    }

    /// Return true when the nearest enclosing class declaration has an extends clause.
    fn enclosing_class_is_derived(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut current = Some(expression_id.into_any());

        // walk outward to the first class declaration
        while let Some(node_id) = current {
            let Some(parent) = tree.get_parent(node_id.id) else {
                break;
            };

            if parent.ty == NodeType::Declaration {
                let declaration = tree.get(parent.into_typed::<Declaration>());
                if let Declaration::Class(declaration) = declaration {
                    return declaration.extends_expression.is_some();
                }
            }

            current = Some(parent);
        }

        false
    }

    /// Infer optional chain receiver metadata when a maybe wrapper is present.
    pub(crate) fn infer_optional_chain_receiver(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<Option<OptionalChainReceiver>> {
        let Expression::Maybe { left } = ctx.tree.get(receiver_id) else {
            return Ok(None);
        };

        // infer the underlying receiver expression
        let receiver_ty_id = self.infer_expression(&mut ctx.reborrow(), *left, state)?;
        let (non_nullish, has_nullish) = self.strip_nullish_from_union(receiver_ty_id, ctx.types);

        Ok(Some(OptionalChainReceiver {
            receiver_id: *left,
            receiver_ty_id: non_nullish,
            has_nullish,
        }))
    }

    /// Add undefined to an optional chain result when needed.
    pub(crate) fn optional_chain_result_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        base_type_id: LocalTypeId,
        has_nullish: bool,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        if !has_nullish {
            return base_type_id;
        }

        let undefined_type_id = types.insert_type_from(
            Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            },
            expression_id,
        );
        let combined = vec![base_type_id, undefined_type_id];
        self.union_type_from_list(combined, base_type_id, types)
    }

    /// Decide whether a scalar literal should be widened in this context.
    pub(crate) fn should_widen_scalar_literal(&self, state: &InferState) -> bool {
        if !matches!(state.widening_mode, WideningMode::Widen) {
            return false;
        }
        if !matches!(state.freshness_mode, FreshnessMode::Regularized) {
            return false;
        }
        matches!(state.const_context, ConstContext::None)
    }

    /// Widen a scalar literal type when the context requires it.
    pub(crate) fn widen_scalar_literal_type_if_needed(
        &self,
        module: &Module,
        types: &mut TypeTable,
        type_id: LocalTypeId,
        state: &InferState,
        source_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        if !self.should_widen_scalar_literal(state) {
            return type_id;
        }

        let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        } = types.get_type(type_id)
        else {
            return type_id;
        };

        let widened = Type::TypeLiteral {
            value: self.widen_scalar_literal_for_module(module, literal),
        };
        types.insert_type_from_any(widened, source_id)
    }

    /// Resolve the element type produced by spreading a value in an array literal.
    pub(crate) fn array_spread_element_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        // apparent types determine array literal spread shapes
        let type_id = {
            let mut visited = Vec::new();
            self.normalize_type_inner(
                &mut ctx.reborrow(),
                type_id,
                NormalizationMode::Assign,
                RelationMode::OBJECT_SHAPE,
                &mut visited,
            )
        };
        match ctx.types.get_type(type_id) {
            Type::Array { element, .. } => *element,
            Type::ArraySized { element, .. } => Some(*element),
            Type::Tuple { elements, .. } => {
                let elements = elements.clone();
                let mut element_type_ids = Vec::with_capacity(elements.len());
                for element in elements {
                    let mut element_ty_id = element.ty;
                    if element.is_rest
                        && let Some(rest_element_ty_id) =
                            self.array_spread_element_type(&mut ctx.reborrow(), element_ty_id)
                    {
                        element_ty_id = rest_element_ty_id;
                    }
                    element_type_ids.push(element_ty_id);
                }
                if element_type_ids.is_empty() {
                    None
                } else {
                    let source_type_id = element_type_ids[0];
                    Some(self.union_type_from_list(element_type_ids, source_type_id, ctx.types))
                }
            }
            Type::Union { elements } => {
                let elements = elements.clone();
                let mut element_type_ids = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let element_type_id =
                        self.array_spread_element_type(&mut ctx.reborrow(), element_id)?;
                    element_type_ids.push(element_type_id);
                }
                if element_type_ids.is_empty() {
                    None
                } else {
                    let source_type_id = element_type_ids[0];
                    Some(self.union_type_from_list(element_type_ids, source_type_id, ctx.types))
                }
            }
            _ => None,
        }
    }

    /// Select the best common type for a pair of branch results.
    fn best_common_type_for_pair(
        &self,
        ctx: &mut TypeContext<'_>,
        state: &InferState,
        source_id: LocalNodeIdAny,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        expected_type: Option<LocalTypeId>,
        allow_widening: bool,
    ) -> LocalTypeId {
        let mut left_ty_id = left_ty_id;
        let mut right_ty_id = right_ty_id;

        // widen scalar literals when no contextual type is forcing a shape
        if allow_widening {
            left_ty_id = self.widen_scalar_literal_type_if_needed(
                ctx.module, ctx.types, left_ty_id, state, source_id,
            );
            right_ty_id = self.widen_scalar_literal_type_if_needed(
                ctx.module,
                ctx.types,
                right_ty_id,
                state,
                source_id,
            );
        }

        // prefer the contextual type when both branches satisfy it
        if let Some(expected_ty_id) = expected_type {
            let left_assignable =
                self.is_type_assignable(&mut ctx.reborrow(), expected_ty_id, left_ty_id);
            let right_assignable =
                self.is_type_assignable(&mut ctx.reborrow(), expected_ty_id, right_ty_id);
            if left_assignable.is_assignable() && right_assignable.is_assignable() {
                return expected_ty_id;
            }
        }

        // prefer a common supertype when one branch subsumes the other
        let left_to_right = self.is_type_assignable(&mut ctx.reborrow(), right_ty_id, left_ty_id);
        if left_to_right.is_assignable() {
            return right_ty_id;
        }
        let right_to_left = self.is_type_assignable(&mut ctx.reborrow(), left_ty_id, right_ty_id);
        if right_to_left.is_assignable() {
            return left_ty_id;
        }

        // widen numeric branches to a shared numeric type when allowed
        let left_ty = ctx.types.get_type(left_ty_id).clone();
        let right_ty = ctx.types.get_type(right_ty_id).clone();
        let left_is_numeric = self.is_numeric_like_type(&left_ty, ctx.types);
        let right_is_numeric = self.is_numeric_like_type(&right_ty, ctx.types);
        if left_is_numeric && right_is_numeric {
            let left_is_literal = matches!(
                left_ty,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                            | ScalarLiteral::Float(_)
                            | ScalarLiteral::Bigint(_),
                    ),
                }
            );
            let right_is_literal = matches!(
                right_ty,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                            | ScalarLiteral::Float(_)
                            | ScalarLiteral::Bigint(_),
                    ),
                }
            );
            let allow_numeric_widening = allow_widening
                && (!left_is_literal
                    || !right_is_literal
                    || self.should_widen_scalar_literal(state));
            if allow_numeric_widening {
                let widened = self.widen_numeric_types(&left_ty, &right_ty);
                return ctx.types.insert_type_from_any(widened, source_id);
            }
        }

        self.union_type_from_list(vec![left_ty_id, right_ty_id], left_ty_id, ctx.types)
    }

    /// Select the best common type for a list of branch results.
    fn best_common_type_for_list(
        &self,
        ctx: &mut TypeContext<'_>,
        state: &InferState,
        source_id: LocalNodeIdAny,
        candidates: &[LocalTypeId],
    ) -> LocalTypeId {
        let Some((&first, rest)) = candidates.split_first() else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Never,
            };
            return ctx.types.insert_type_from_any(ty, source_id);
        };

        // allow the contextual type only when all candidates satisfy it
        let contextual_expected_type = self.expected_value_type(state.expected_type, ctx.types);
        let expected_type = contextual_expected_type.filter(|expected_ty_id| {
            candidates.iter().all(|candidate| {
                self.is_type_assignable(&mut ctx.reborrow(), *expected_ty_id, *candidate)
                    .is_assignable()
            })
        });
        let allow_widening = expected_type.is_none();

        rest.iter().fold(first, |current, next| {
            self.best_common_type_for_pair(
                &mut ctx.reborrow(),
                state,
                source_id,
                current,
                *next,
                expected_type,
                allow_widening,
            )
        })
    }

    /// Resolve the result type for a loop expression from collected break values.
    fn loop_break_result_type(
        &self,
        ctx: &mut TypeContext<'_>,
        state: &InferState,
        source_id: LocalNodeIdAny,
        break_values: &[LocalTypeId],
        expected_type: Option<LocalTypeId>,
    ) -> LocalTypeId {
        let Some((&first, rest)) = break_values.split_first() else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return ctx.types.insert_type_from_any(ty, source_id);
        };

        // use contextual expected types only when they are concrete
        let expected_type = self.expected_value_type(expected_type, ctx.types);
        let allow_widening = expected_type.is_none();
        rest.iter().fold(first, |current, next| {
            self.best_common_type_for_pair(
                &mut ctx.reborrow(),
                state,
                source_id,
                current,
                *next,
                expected_type,
                allow_widening,
            )
        })
    }

    /// Resolve the yield and return types for a yield* delegate value.
    fn yield_star_delegate_types(
        &self,
        ctx: &mut TypeContext<'_>,
        value_ty_id: LocalTypeId,
    ) -> Option<(LocalTypeId, Option<LocalTypeId>)> {
        if let Some(element_ty_id) =
            self.array_spread_element_type(&mut ctx.reborrow(), value_ty_id)
        {
            return Some((element_ty_id, None));
        }

        if let Some((yield_ty_id, return_ty_id, _next_ty_id)) =
            self.generator_type_arguments(&mut ctx.reborrow(), value_ty_id)
        {
            return Some((yield_ty_id, Some(return_ty_id)));
        }

        let (symbol, generic_arguments) = {
            let Type::Reference {
                symbol,
                generic_arguments,
            } = ctx.types.get_type(value_ty_id)
            else {
                return None;
            };

            (*symbol, generic_arguments.clone())
        };
        let source_id = ctx.types.get_type_source(value_ty_id);

        let canonical_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let iterable_symbol =
            self.get_well_known_type_symbol(ctx.profile, WellKnownSymbol::Iterable)?;
        if canonical_symbol != iterable_symbol {
            return None;
        }

        let unknown_ty_id = ctx.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );
        let yield_ty_id = generic_arguments
            .as_ref()
            .and_then(|arguments| arguments.first())
            .map(|argument| self.convert_static_argument_type(argument, source_id, ctx.types))
            .unwrap_or(unknown_ty_id);

        Some((yield_ty_id, None))
    }

    /// Resolve the value type observed by a for each binding.
    fn for_each_binding_type(
        &self,
        ctx: &mut TypeContext<'_>,
        iterator_ty_id: LocalTypeId,
        asynchrony: Asynchrony,
        kind: ForEachKind,
    ) -> LocalTypeId {
        let source_id = ctx.types.get_type_source(iterator_ty_id);

        // for in bindings are still intentionally loose here
        if kind == ForEachKind::In {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };

            return ctx.types.insert_type_from_any(ty, source_id);
        }

        // arrays and tuples expose their element types directly
        if let Some(element_ty_id) =
            self.array_spread_element_type(&mut ctx.reborrow(), iterator_ty_id)
        {
            if asynchrony == Asynchrony::Async {
                return self.unwrap_awaited_type(&mut ctx.reborrow(), element_ty_id);
            }

            return element_ty_id;
        }

        // iterators and generators expose their yielded type
        if let Some((yield_ty_id, _, _)) =
            self.generator_type_arguments(&mut ctx.reborrow(), iterator_ty_id)
        {
            if asynchrony == Asynchrony::Async {
                return self.unwrap_awaited_type(&mut ctx.reborrow(), yield_ty_id);
            }

            return yield_ty_id;
        }

        // iterable references expose their first type argument
        let reference = match ctx.types.get_type(iterator_ty_id).clone() {
            Type::Reference {
                symbol,
                generic_arguments,
            } => Some((symbol, generic_arguments)),
            _ => None,
        };
        if let Some((symbol, generic_arguments)) = reference {
            let canonical_symbol = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            let iterable_symbol =
                self.get_well_known_type_symbol(ctx.profile, WellKnownSymbol::Iterable);
            let async_iterable_symbol =
                self.get_well_known_type_symbol(ctx.profile, WellKnownSymbol::AsyncIterable);
            let async_iterator_symbol =
                self.get_well_known_type_symbol(ctx.profile, WellKnownSymbol::AsyncIterator);
            let is_iterable = iterable_symbol
                .is_some_and(|iterable_symbol| iterable_symbol == canonical_symbol)
                || async_iterable_symbol
                    .is_some_and(|async_iterable_symbol| async_iterable_symbol == canonical_symbol)
                || async_iterator_symbol
                    .is_some_and(|async_iterator_symbol| async_iterator_symbol == canonical_symbol);

            if is_iterable {
                let resolved_arguments = self
                    .resolve_type_reference_static_arguments(
                        &mut ctx.reborrow(),
                        source_id,
                        symbol,
                        generic_arguments.as_deref(),
                        true,
                    )
                    .ok()
                    .flatten();
                let arguments = resolved_arguments
                    .as_ref()
                    .or(generic_arguments.as_ref())
                    .map(|arguments| arguments.as_slice())
                    .unwrap_or(&[]);
                if let Some(argument) = arguments.first() {
                    let element_ty_id =
                        self.convert_static_argument_type(argument, source_id, ctx.types);
                    if asynchrony == Asynchrony::Async {
                        return self.unwrap_awaited_type(&mut ctx.reborrow(), element_ty_id);
                    }

                    return element_ty_id;
                }
            }
        }

        // unresolved iterator shapes should not recurse through the source type
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        };

        ctx.types.insert_type_from_any(ty, source_id)
    }

    /// Resolve a type expression or fall back to inference when unevaluated.
    fn resolve_type_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        _state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // evaluate the type expression directly from type syntax
        let ty_id = self.resolve_declared_type_expression(
            &mut ctx.type_context_reborrow(),
            expression_id,
            true,
            true,
        )?;

        Ok(ty_id)
    }

    /// Infer a direct binding value type when none is cached yet.
    pub(crate) fn infer_direct_binding_value_type(
        &self,
        ctx: &mut InferContext<'_>,
        symbol: GlobalSymbolId,
        state: &InferState,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // only infer local bindings
        if symbol.module_id != ctx.module.id {
            return Ok(None);
        }

        // keep concrete cached value types and only refine infer placeholders
        let existing_value_type_id = ctx.types.get_value_type_id(symbol);
        if existing_value_type_id.is_some_and(|value_type_id| {
            !self.is_infer_var_type(value_type_id, ctx.types)
                && !self.unwrapped_value_type_is_unevaluated(value_type_id, ctx.types)
        }) {
            return Ok(None);
        }

        // resolve the declarator for the binding
        let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(ctx.tree_symbol_view(), symbol)
        else {
            return Ok(None);
        };

        // reuse declared types when present
        let declared_ty_id = ctx
            .types
            .get_declared_type_id(declarator_id.into_global_any(ctx.module.id));
        if let Some(declared_ty_id) = declared_ty_id {
            self.resolve_declared_type(&mut ctx.type_context_reborrow(), declared_ty_id)?;
            ctx.types.set_value_type(symbol, declared_ty_id);
            return Ok(Some(declared_ty_id));
        }

        // infer from the initializer when available
        let declarator = ctx.tree.get(declarator_id);
        let Some(value_id) = declarator.value else {
            return Ok(None);
        };

        // seed a placeholder to avoid recursion through self references
        let scope = InferScope {
            owner: symbol,
            function_id: state
                .in_function
                .map(|function_id| function_id.into_global(ctx.module.id)),
        };
        let placeholder_ty_id = if let Some(existing_value_type_id) = existing_value_type_id
            && self.is_infer_var_type(existing_value_type_id, ctx.types)
        {
            existing_value_type_id
        } else {
            self.infer_var_type_for_symbol(
                ctx.infer,
                ctx.types,
                symbol,
                value_id.into_any(),
                InferOrigin::Expression(value_id.into_global_any(ctx.module.id)),
                scope,
            )
        };
        ctx.types.set_value_type(symbol, placeholder_ty_id);

        // infer the initializer with binding defaults
        let mut value_ctx = state
            .reset()
            .with_expected_type(None)
            .with_contextual_typing_mode(ContextualTypingMode::Default);
        let binding_mutability = ctx.symbols.get_symbol(symbol.local_id).binding_mutability;
        value_ctx = self.binding_initializer_context(&value_ctx, binding_mutability);
        let inferred_ty_id =
            self.infer_expression(&mut ctx.reborrow(), value_id, &mut value_ctx)?;

        // commit the binding type before caching it
        let committed_ty_id = self.materialize_declarator_initializer_type(
            &mut ctx.type_context_reborrow(),
            declarator_id,
            inferred_ty_id,
            &value_ctx,
        );
        ctx.types.set_value_type(symbol, committed_ty_id);
        ctx.infer
            .upsert_direct_binding_value_commit_intent(symbol, declarator_id, value_id);

        Ok(Some(committed_ty_id))
    }

    /// Resolve a flow-narrowed symbol type when it is concrete enough for value inference.
    fn resolved_narrowed_type_for_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        symbol: GlobalSymbolId,
        state: &InferState,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip when no narrowing is available
        let Some(narrowed_ty_id) = state.get_narrowed(symbol) else {
            return Ok(None);
        };

        // resolve the narrowed unwrapped value type before using it
        let narrowed_unwrapped_ty_id =
            self.ensure_unwrapped_value_type_evaluated(&mut ctx.reborrow(), narrowed_ty_id)?;

        // ignore stale or indeterminate narrowings and fall back to stable symbol commitments
        if self.unwrapped_value_type_is_indeterminate(narrowed_unwrapped_ty_id, ctx.types) {
            return Ok(None);
        }

        Ok(Some(narrowed_ty_id))
    }

    /// Resolve one cached inferred type id when it is ready for runtime use.
    fn inferred_type_id_for_node_if_ready(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: GlobalNodeIdAny,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(inferred_type_id) = ctx.infer.inferred_type_for_node(node_id) else {
            return Ok(None);
        };

        let inferred_unwrapped_ty_id =
            self.ensure_unwrapped_value_type_evaluated(&mut ctx.reborrow(), inferred_type_id)?;
        if self.unwrapped_value_type_is_unevaluated(inferred_unwrapped_ty_id, ctx.types) {
            return Ok(None);
        }

        Ok(Some(inferred_type_id))
    }

    /// Instantiate one inferred expression type from infer-local instance obligations.
    fn instantiate_type_from_node_instance_obligation(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        let node_id = expression_id.into_global_any(ctx.module.id);
        let Some((instance_symbol, instance_arguments)) =
            self.query_instance_symbol_arguments_for_node_infer(node_id, ctx.infer, ctx.types)
        else {
            return type_id;
        };

        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.type_context_reborrow(),
            instance_symbol,
            expression_id.into_any(),
            &instance_arguments,
        );
        if substitutions.is_empty() {
            return type_id;
        }

        let mut materialize_cache = TypeRewriteCache::new();
        let mut substitution_cache = HashMap::new();
        self.instantiate_type_with_substitutions(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            None,
            type_id,
            &substitutions,
            &mut materialize_cache,
            &mut substitution_cache,
        )
    }

    /// Infer an (expression) body with flow aware typing.
    pub(crate) fn infer_body(
        &self,
        ctx: &mut InferContext<'_>,
        body_id: LocalNodeId<Expression>,
        context: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // build a flow graph and flow table for the function body when needed
        let previous_flow = context.flow.clone();
        if self.expression_requires_flow(ctx.tree, body_id) {
            let graph = FlowGraphBuilder::new(ctx.module.id, ctx.tree).build(body_id);
            let flow = self.compute_flow_table_for_graph(
                &mut ctx.type_context_reborrow(),
                &graph,
                context,
            )?;

            // seed the inference context with flow information
            context.flow = Some(FlowContext {
                module_id: ctx.module.id,
                graph: Arc::new(graph),
                table: Arc::new(flow),
            });
        } else {
            context.flow = None;
        }

        // infer the expression using the flow context
        let result = self.infer_expression(&mut ctx.reborrow(), body_id, context)?;

        // restore the previous flow context
        context.flow = previous_flow;

        Ok(result)
    }

    /// Infer statement-like expressions that produce `void` or delegated statement results.
    fn infer_statement_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let expression = ctx.tree.get(expression_id);
        let ty_id = match expression {
            // declaration: analyze the declaration
            Expression::Declaration(declaration) => {
                if !self.declaration_requires_infer(ctx.module, *declaration, ctx.tree) {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    return Ok(ctx.types.insert_type_from(ty, expression_id));
                }

                self.infer_declaration(&mut ctx.reborrow(), *declaration, state)?;
                let declaration = ctx.tree.get(*declaration);

                // function declarations used as expressions evaluate to function values
                if let Declaration::Function(declaration) = declaration {
                    if let Some(value_ty_id) = ctx
                        .types
                        .get_value_type_id(declaration.symbol.into_global(ctx.module.id))
                    {
                        value_ty_id
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Void,
                        };
                        ctx.types.insert_type_from(ty, expression_id)
                    }
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    ctx.types.insert_type_from(ty, expression_id)
                }
            }

            // block: analyze the block
            Expression::Block(block) => self.infer_block(&mut ctx.reborrow(), *block, state)?,

            // statement: analyze the statement
            // labelled statement: analyze the body with label in context
            Expression::Labelled {
                label: _,
                body: body_id,
                symbol: _,
            } => {
                self.infer_expression(&mut ctx.reborrow(), *body_id, state)?;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }

            // import / exports
            Expression::Import {
                kind: _,
                source,
                target: _,
                target_module: _,
                items,
                attributes,
                arguments,
            } => {
                // reject dynamic imports when configured
                if state.options.no_dynamic_import
                    && matches!(ctx.module.source, ModuleSource::User)
                    && matches!(source, ImportSource::ImportCall | ImportSource::RequireCall)
                {
                    self.error(AnalyzeError::DynamicImportDisabled {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                for item_id in items.as_deref().unwrap_or(&[]) {
                    self.infer_dependency_item(&mut ctx.reborrow(), *item_id, state)?;
                }
                let _attributes = attributes;
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.infer_argument(&mut ctx.reborrow(), *argument_id, None, state)?;
                    }
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }
            Expression::UnresolvedImport {
                kind: _,
                source,
                target,
                items,
                attributes,
                arguments,
                ..
            } => {
                // reject dynamic imports when configured
                if state.options.no_dynamic_import
                    && matches!(ctx.module.source, ModuleSource::User)
                    && matches!(source, ImportSource::ImportCall | ImportSource::RequireCall)
                {
                    self.error(AnalyzeError::DynamicImportDisabled {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                if let ImportTarget::Expression { target } = target {
                    self.infer_expression(&mut ctx.reborrow(), *target, state)?;
                }

                for item_id in items.as_deref().unwrap_or(&[]) {
                    self.infer_dependency_item(&mut ctx.reborrow(), *item_id, state)?;
                }
                let _attributes = attributes;
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.infer_argument(&mut ctx.reborrow(), *argument_id, None, state)?;
                    }
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }
            Expression::ReExport {
                target: _,
                target_module: _,
                kind: _,
                items,
                attributes,
            }
            | Expression::UnresolvedReExport {
                target: _,
                kind: _,
                items,
                attributes,
            } => {
                for item_id in items {
                    self.infer_dependency_item(&mut ctx.reborrow(), *item_id, state)?;
                }
                let _attributes = attributes;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }
            Expression::Export {
                kind: _,
                items,
                attributes,
            } => {
                for item_id in items {
                    self.infer_dependency_item(&mut ctx.reborrow(), *item_id, state)?;
                }
                let _attributes = attributes;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }
            Expression::ExportNamespace { name: _ } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }

            // let
            Expression::Let {
                mutability,
                declarators,
                ..
            } => {
                for decl_id in declarators {
                    let mut decl_ctx = state.fork().with_binding_mutability(*mutability);
                    self.infer_declarator(
                        &mut ctx.reborrow(),
                        *decl_id,
                        expression_id,
                        DeclaratorConstraint::Assignable,
                        &mut decl_ctx,
                    )?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }
            // using
            Expression::Using {
                asynchrony: _,
                declarators,
                ..
            } => {
                for decl_id in declarators {
                    let mut decl_ctx = state.fork().with_using_binding();
                    self.infer_declarator(
                        &mut ctx.reborrow(),
                        *decl_id,
                        expression_id,
                        DeclaratorConstraint::Assignable,
                        &mut decl_ctx,
                    )?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }

            _ => unreachable!("statement helper called with non-statement expression"),
        };

        Ok(ty_id)
    }

    /// Infer cast-like expressions.
    fn infer_cast_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let ty_id = match expression {
            // `value as T`: preserve the asserted target type
            Expression::As {
                operator: _,
                source: _,
                expression: value,
                target_type,
            } => {
                // reject unsafe explicit casts when configured
                if state.options.no_unsafe_type_assertions
                    && matches!(ctx.module.source, ModuleSource::User)
                {
                    self.error(AnalyzeError::UnsafeTypeAssertionDisabled {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                // infer the source and resolve the target type
                self.infer_expression(&mut ctx.reborrow(), *value, state)?;
                self.resolve_type_expression(&mut ctx.reborrow(), *target_type, state)?
            }

            // `value satisfies T`: keep the source type, but enforce relation to `T`
            Expression::Satisfies {
                expression: value,
                target_type,
            } => {
                let target_ty_id =
                    self.resolve_type_expression(&mut ctx.reborrow(), *target_type, state)?;
                ctx.infer.set_inferred_type_for_node(
                    target_type.into_global_any(ctx.module.id),
                    target_ty_id,
                );

                let mut value_ctx = state
                    .fork()
                    .with_expected_type(Some(target_ty_id))
                    .with_contextual_typing_mode(ContextualTypingMode::Satisfies);
                let value_ty_id =
                    self.infer_expression(&mut ctx.reborrow(), *value, &mut value_ctx)?;
                let value_ty_id = self.instantiate_type_from_node_instance_obligation(
                    &mut ctx.reborrow(),
                    *value,
                    value_ty_id,
                );

                // defer the final relation until substitutions converge
                self.push_relation_obligation_for_target_type_and_source_expression(
                    ctx.module,
                    expression_id.into_any(),
                    target_ty_id,
                    *value,
                    TypeRelationObligationDiagnostic::UnsatisfiedType,
                    ctx.infer,
                );

                value_ty_id
            }
            _ => unreachable!("cast helper called with non-cast expression"),
        };

        Ok(ty_id)
    }

    /// Infer value, reference, and pointer ownership operation expressions.
    fn infer_value_reference_operation_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let ty_id = match expression {
            // value of operation: value of type
            Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mut ownership_ctx = state.fork().with_explicit_ownership();
                let right_ty_id =
                    self.infer_expression(&mut ctx.reborrow(), *right, &mut ownership_ctx)?;

                // reject ownership conversions on explicit ownership types
                if self.type_is_explicit_ownership_wrapper(ctx.types, right_ty_id) {
                    self.error(AnalyzeError::InvalidOwnershipOperand {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                        actual_ty: right_ty_id.into_global(ctx.module.id),
                    });
                }

                let ty = self.infer_value_of_operation(*mutability, *variance, right_ty_id);
                ctx.types.insert_type_from(ty, expression_id)
            }

            // reference of operation: reference of type
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mut ownership_ctx = state.fork().with_explicit_ownership();
                let right_ty_id =
                    self.infer_expression(&mut ctx.reborrow(), *right, &mut ownership_ctx)?;

                let ty = self.infer_reference_of_operation(*mutability, *variance, right_ty_id);
                ctx.types.insert_type_from(ty, expression_id)
            }
            // pointer of operation: raw pointer to type
            Expression::PointerOf { mutability, right } => {
                let mut ownership_ctx = state.fork().with_explicit_ownership();
                let right_ty_id =
                    self.infer_expression(&mut ctx.reborrow(), *right, &mut ownership_ctx)?;

                let ty = Type::PointerOf {
                    mutability: *mutability,
                    right: right_ty_id,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }
            _ => unreachable!("value/reference helper called with non ownership operation"),
        };

        Ok(ty_id)
    }

    /// Infer delete expressions.
    fn infer_delete_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // enforce strict mode delete restrictions on bindings
        let enforce_strict_mode = ctx.module.source_type.is_module() || state.options.always_strict;
        if enforce_strict_mode && matches!(ctx.module.source, ModuleSource::User) {
            let target_id = self.unwrap_parenthesized_expression(value, ctx.tree);

            // forbid delete on binding references in strict mode
            if matches!(
                ctx.tree.get(target_id),
                Expression::LocalReference { .. }
                    | Expression::ModuleReference { .. }
                    | Expression::GlobalReference { .. }
            ) {
                self.error(AnalyzeError::InvalidStrictDelete {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // reject delete in dynamic shape restricted mode
        if state.options.no_dynamic_shapes && matches!(ctx.module.source, ModuleSource::User) {
            self.error(AnalyzeError::DynamicShapesDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }
        let _value_ty_id = self.infer_expression(&mut ctx.reborrow(), value, state)?;

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer special-reference and pseudo-reference expressions.
    fn infer_special_reference_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let ty_id = match expression {
            // unresolved references use error recovery
            Expression::UnresolvedPath {
                path: _,
                generic_arguments: _,
                space_order: _,
            } => ctx.types.insert_type_from(Type::Error, expression_id),

            // private identifiers only appear in brand checks
            Expression::PrivateIdentifier { name: _ } => {
                ctx.types.insert_type_from(Type::Error, expression_id)
            }

            // import meta: statically known type
            Expression::ImportMeta => {
                // resolve the import.meta interface symbol
                let Some(import_meta_symbol) =
                    self.get_language_symbol(ctx.profile, LanguageSymbol::ImportMeta)
                else {
                    self.error(AnalyzeError::Internal {
                        message: format!(
                            "missing language symbol for import.meta: module={}, profile={:?}",
                            ctx.module.id, state.profile,
                        ),
                    });
                    return Ok(ctx.types.insert_type_from(Type::Error, expression_id));
                };

                // prefer one apparent or imported instance type so member lookup sees object fields eagerly
                if let Some(import_meta_instance_ty_id) = self
                    .require_instance_type(
                        &mut ctx.type_context_reborrow(),
                        expression_id.into_any(),
                        import_meta_symbol,
                    )
                    .or(self.import_instance_type_for_symbol(
                        &ctx.index,
                        ctx.compiler_context.revision(),
                        ctx.profile,
                        expression_id.into_any(),
                        import_meta_symbol,
                        ctx.types,
                    )?)
                {
                    import_meta_instance_ty_id
                } else if import_meta_symbol.module_id != ctx.module.id {
                    // resolve import.meta through the remote value boundary before using a raw reference
                    self.resolve_remote_symbol_value_type_for_context(
                        &mut ctx.reborrow(),
                        state,
                        expression_id.into_any(),
                        import_meta_symbol,
                    )?
                } else {
                    // fall back to a direct reference when no instance record is published yet
                    ctx.types.insert_type_from(
                        Type::Reference {
                            symbol: import_meta_symbol,
                            generic_arguments: None,
                        },
                        expression_id,
                    )
                }
            }

            // new target: preserve syntax, but keep the type lane conservative
            Expression::NewTarget => ctx.types.insert_type_from(Type::Error, expression_id),

            // this: reference to the current instance item context
            Expression::This => {
                let this_symbol = self.resolve_this_symbol(ctx.tree_symbol_view(), expression_id);
                if let Some(this_symbol) = this_symbol
                    && let Some(ty_id) = ctx.types.get_value_type_id(this_symbol)
                {
                    ty_id
                } else if let Some(this_ty_id) =
                    self.contextual_this_type(ctx.module, state, ctx.types)
                {
                    this_ty_id
                } else {
                    // report implicit this in functions and scripts
                    if state.options.no_implicit_this
                        && !matches!(ctx.module.source, ModuleSource::Builtin(_))
                    {
                        let is_script = ctx.module.source_type.is_script();
                        let in_function = state.in_function.is_some();
                        if is_script || in_function {
                            self.error(AnalyzeError::ImplicitThis {
                                node: expression_id
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            });
                        }
                    }

                    // default to undefined in modules, unknown in scripts
                    if ctx.module.source_type.is_module() {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Undefined,
                        };
                        ctx.types.insert_type_from(ty, expression_id)
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        ctx.types.insert_type_from(ty, expression_id)
                    }
                }
            }

            // super: reference to the base nominal type when available
            Expression::Super => {
                if let Some(super_ty_id) =
                    self.super_type_for_context(&mut ctx.reborrow(), expression_id, state)
                {
                    super_ty_id
                } else {
                    ctx.types.insert_type_from(Type::Error, expression_id)
                }
            }

            _ => unreachable!("special-reference helper called with non special-reference"),
        };

        Ok(ty_id)
    }

    /// Infer instantiation expressions.
    fn infer_instantiation_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        generic_arguments: &[LocalNodeId<GenericArgument>],
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let left_ty_id = self.infer_expression(&mut ctx.reborrow(), left, state)?;
        if generic_arguments.is_empty() {
            return Ok(left_ty_id);
        }

        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            left_ty_id,
        )?;

        let mut owner_symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), left);

        if owner_symbol.is_none() {
            let node_id = left.into_global_any(ctx.module.id);
            if let Some(resolution) =
                self.query_resolution_for_node_infer(node_id, ctx.infer, ctx.types)
                && let Resolution::Static { candidate, .. } = resolution
            {
                owner_symbol = Some(candidate.target_symbol);
            }
        }

        self.instantiate_callable_type_with_static_arguments(
            ctx,
            expression_id,
            left_ty_id,
            owner_symbol,
            generic_arguments,
        )
    }

    /// Instantiate one callable type with static arguments and record the resulting instance.
    fn instantiate_callable_type_with_static_arguments(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        base_ty_id: LocalTypeId,
        owner_symbol: Option<GlobalSymbolId>,
        generic_argument_ids: &[LocalNodeId<GenericArgument>],
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve one callable signature for generic instantiation
        let Some(signature_ty_id) = self
            .call_signatures_for_type(base_ty_id, &*ctx.types)
            .first()
            .copied()
        else {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                message: "static arguments require a callable generic target".to_string(),
            });
            return Ok(base_ty_id);
        };

        let Type::Function {
            asynchrony,
            cardinality,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
        } = ctx.types.get_type(signature_ty_id).clone()
        else {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                message: "static arguments require a callable generic target".to_string(),
            });
            return Ok(base_ty_id);
        };

        // ensure owner generic metadata is consistent when no signature static params are present
        if generic_parameters.is_empty()
            && let Some(owner_symbol) = owner_symbol
        {
            let owner_parameter_symbols = self
                .collect_static_parameter_symbols(ctx.type_view(), owner_symbol)
                .unwrap_or_default();
            if !owner_parameter_symbols.is_empty() {
                return Err(AnalyzeError::Internal {
                    message: format!(
                        "missing signature static parameters for generic owner {owner_symbol:?}"
                    ),
                });
            }
        }

        let resolved = self
            .resolve_function_static_arguments(
                &mut ctx.reborrow(),
                super::call::SignatureStaticResolutionContext {
                    node_id: expression_id.into_any(),
                    owner_symbol,
                    generic_argument_ids: Some(generic_argument_ids),
                    prefilled_static_arguments: None,
                    bound_substitutions: None,
                    argument_ids: None,
                    generic_parameter_type_ids: &generic_parameters,
                    parameter_type_ids: &parameters,
                    return_type,
                    expected_return_type: None,
                    mode: super::SignatureResolutionMode::Check,
                    allow_missing_value_arguments: false,
                },
            )?
            .unwrap_or(ResolvedSignature {
                parameters,
                return_type,
                generic_arguments: Vec::new(),
            });

        let instantiated_fn = Type::Function {
            asynchrony,
            cardinality,
            generic_parameters: Vec::new(),
            this_parameter,
            parameters: resolved.parameters,
            return_type: resolved.return_type,
        };
        let instantiated_ty_id = ctx.types.insert_type_from(instantiated_fn, expression_id);

        // record one provisional instance for static argument substitutions
        if let Some(owner_symbol) = owner_symbol {
            let signature_parameter_symbols =
                self.query_signature_static_parameter_symbols(signature_ty_id, ctx.types);
            let environment = StaticSubstitutionEnvironment::from_parameter_symbols(
                resolved.generic_arguments.clone(),
                signature_parameter_symbols,
                0,
            )
            .or_else(|| {
                self.instance_environment_for_symbol_arguments(
                    &ctx.reborrow(),
                    owner_symbol,
                    resolved.generic_arguments.clone(),
                    0,
                )
            });
            if let Some(environment) = environment {
                self.record_node_provisional_instance(
                    expression_id.into_global_any(ctx.module.id),
                    owner_symbol,
                    environment,
                    &mut *ctx.infer,
                    ctx.types,
                )?;
            }
        }

        Ok(instantiated_ty_id)
    }

    /// Infer scalar literals and type-as-value expressions.
    fn infer_scalar_or_type_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        types: &mut TypeTable,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let ty_id = match expression {
            // scalar literal: derive type from value
            Expression::ScalarLiteral { value } => {
                // reject managed literals when runtime-managed values are disabled
                if state.options.no_managed
                    && !state.is_explicit_ownership
                    && matches!(module.source, ModuleSource::User)
                    && self.scalar_literal_is_managed(value)
                {
                    self.error(AnalyzeError::ManagedMemoryDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(state.profile)),
                    });
                }

                // apply contextual typing when a matching expected type is available
                let allow_contextual_literal =
                    !matches!(state.contextual_typing, ContextualTypingMode::Satisfies);
                let should_preserve_scalar_literal =
                    matches!(state.contextual_typing, ContextualTypingMode::Satisfies)
                        && self.expected_type_is_scalar_literal_union(state.expected_type, types);
                if allow_contextual_literal
                    && let Some(expected_ty_id) = self.expected_type_for_scalar_literal(
                        value,
                        state.expected_type,
                        types,
                        &state.options,
                    )
                {
                    expected_ty_id
                } else {
                    let literal = if self.should_widen_scalar_literal(state) {
                        self.widen_scalar_literal_for_module(module, value)
                    } else {
                        self.infer_scalar_literal(value)
                    };
                    let ty = Type::TypeLiteral { value: literal };
                    let ty_id = types.insert_type_from(ty, expression_id);
                    if should_preserve_scalar_literal {
                        self.set_type_freshness(types, ty_id, Freshness::Regular);
                    } else {
                        self.apply_infer_state_type_freshness(types, ty_id, state);
                    }
                    ty_id
                }
            }

            // type literal expressions are value-level carriers of type-literal payloads
            Expression::TypeLiteral { value } => {
                let ty = Type::TypeLiteral {
                    value: value.clone(),
                };
                types.insert_type_from(ty, expression_id)
            }

            // type as a value: type
            Expression::Type { resolved_type, .. } => {
                let ty = Type::Value {
                    value: *resolved_type,
                };
                types.insert_type_from(ty, expression_id)
            }

            _ => unreachable!("scalar/type helper called with non scalar-or-type expression"),
        };

        Ok(ty_id)
    }

    /// Infer grouped expressions that forward inner expression typing.
    fn infer_grouping_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let ty_id = match expression {
            // sequence expression (comma operator): type of last expression
            Expression::SequenceExpression { expressions } => {
                let mut last_ty = None;
                for (index, expr_id) in expressions.iter().enumerate() {
                    let is_last = index + 1 == expressions.len();
                    if is_last {
                        let mut expr_ctx = state.fork().with_expected_type(state.expected_type);
                        last_ty = Some(self.infer_expression(
                            &mut ctx.reborrow(),
                            *expr_id,
                            &mut expr_ctx,
                        )?);
                    } else {
                        let mut expr_ctx = state.fork().with_expected_type(None);
                        last_ty = Some(self.infer_expression(
                            &mut ctx.reborrow(),
                            *expr_id,
                            &mut expr_ctx,
                        )?);
                    }
                }

                // return the type of the last expression, or void if empty (should not be empty)
                last_ty.unwrap_or_else(|| {
                    ctx.types.insert_type_from(
                        Type::TypeLiteral {
                            value: TypeLiteral::Void,
                        },
                        expression_id,
                    )
                })
            }

            // parenthesized: same type as inner
            Expression::Parenthesized {
                expression: inner_id,
            } => self.infer_expression(&mut ctx.reborrow(), *inner_id, state)?,

            _ => unreachable!("grouping helper called with non grouping expression"),
        };

        Ok(ty_id)
    }

    /// Infer tagged newtype construction expressions.
    fn infer_tagged_literal_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_LITERAL);
        let ty_id = match expression {
            Expression::TaggedScalarExpression { ty, value } => {
                // resolve the tag type
                let ty_id = self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    *ty,
                    true,
                    true,
                )?;
                let ty_id = self.expected_tag_reference_type_from_context(
                    *ty,
                    state.expected_type,
                    ty_id,
                    ctx.tree,
                    ctx.types,
                );

                // infer the value expression
                self.infer_expression(&mut ctx.reborrow(), *value, state)?;

                ty_id
            }

            Expression::TaggedTupleExpression { ty, elements } => {
                // resolve the tag type
                let ty_id = self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    *ty,
                    true,
                    true,
                )?;
                let ty_id = self.expected_tag_reference_type_from_context(
                    *ty,
                    state.expected_type,
                    ty_id,
                    ctx.tree,
                    ctx.types,
                );

                // collect expected element types
                let expected_element_types =
                    self.expected_element_types(Some(ty_id), elements.len(), ctx.types);

                // infer each element using contextual types
                for (index, elem) in elements.iter().enumerate() {
                    let expected_element_ty_id =
                        expected_element_types.get(index).copied().flatten();
                    self.infer_argument(&mut ctx.reborrow(), *elem, expected_element_ty_id, state)?;
                }

                ty_id
            }

            Expression::TaggedObjectExpression { ty, properties } => {
                // resolve the tag type
                let ty_id = self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    *ty,
                    true,
                    true,
                )?;
                let ty_id = self.expected_tag_reference_type_from_context(
                    *ty,
                    state.expected_type,
                    ty_id,
                    ctx.tree,
                    ctx.types,
                );

                // derive an expected object type from the tag
                let expected_object_ty_id =
                    self.expected_object_type(&mut ctx.reborrow(), Some(ty_id))?;
                let expected_object_ty_id = self.expected_tagged_object_type(
                    &mut ctx.reborrow(),
                    expression_id,
                    ty_id,
                    expected_object_ty_id,
                )?;

                // record the contextual object type for query consumers
                if let Some(expected_object_ty_id) = expected_object_ty_id {
                    ctx.types.set_contextual_object_type_for_node(
                        expression_id.into_global_any(ctx.module.id),
                        expected_object_ty_id,
                    );
                }

                // infer object literal shapes and fields
                let (literal_fields, shapes, spread_override) = self.infer_object_literal_shapes(
                    &mut ctx.reborrow(),
                    properties,
                    expected_object_ty_id,
                    state,
                )?;
                self.check_excess_object_literal_properties(
                    &mut ctx.reborrow(),
                    expression_id.into_any(),
                    expected_object_ty_id,
                    &literal_fields,
                )?;

                // validate shapes against the expected type
                if let Some(expected_object_ty_id) = expected_object_ty_id
                    && spread_override.is_none()
                {
                    // validate spread shapes against the explicit type
                    for shape in shapes {
                        let shape_ty_id = ctx
                            .types
                            .insert_type_from(shape.into_object_type(), expression_id);
                        if self.is_type_assignable(
                            &mut ctx.type_context_reborrow(),
                            expected_object_ty_id,
                            shape_ty_id,
                        ) == Assignability::NotAssignable
                        {
                            self.emit_unassignable_type_for_types(
                                ctx.module_type_view(),
                                expression_id.into_any(),
                                expected_object_ty_id,
                                shape_ty_id,
                            );
                            break;
                        }
                    }
                }

                ty_id
            }

            _ => unreachable!("tagged helper called with non tagged expression"),
        };

        Ok(ty_id)
    }

    /// Infer tree expressions.
    fn infer_tree_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: Option<LocalNodeId<Expression>>,
        arguments: Option<&[LocalNodeId<Argument>]>,
        elements: Option<&[LocalNodeId<Argument>]>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(left) = left {
            self.infer_expression(&mut ctx.reborrow(), left, state)?;
        }
        if let Some(arguments) = arguments {
            for argument in arguments {
                self.infer_argument(&mut ctx.reborrow(), *argument, None, state)?;
            }
        }
        if let Some(elements) = elements {
            for element in elements {
                self.infer_argument(&mut ctx.reborrow(), *element, None, state)?;
            }
        }

        // #Incomplete: JSX element type (see Elaborate/reify)
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer aggregate literal expressions.
    fn infer_aggregate_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_LITERAL);
        let ty_id = match expression {
            // array expression: infer element types and build array type
            Expression::ArrayExpression { elements } => self.infer_array_literal_expression(
                &mut ctx.reborrow(),
                expression_id,
                elements,
                state,
            )?,

            // tuple expression: preserve positional element types
            Expression::TupleExpression { elements } => self.infer_tuple_literal_expression(
                &mut ctx.reborrow(),
                expression_id,
                elements,
                state,
            )?,

            // object expression: object type
            Expression::ObjectExpression { properties, .. } => self
                .infer_object_literal_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    properties,
                    state,
                )?,

            _ => unreachable!("aggregate helper called with non aggregate literal"),
        };

        // stamp freshness only for the aggregate type synthesized at this node
        if ctx.types.get_type_source(ty_id) == expression_id.into_any() {
            self.apply_infer_state_type_freshness(ctx.types, ty_id, state);
        }

        Ok(ty_id)
    }

    /// Infer an array literal expression.
    fn infer_array_literal_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<Argument>],
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve contextual type for array literals
        let expected_ty_id = self
            .expected_value_type(state.expected_type, ctx.types)
            .map(|expected_ty_id| {
                self.normalize_type_with_relation(
                    &mut ctx.type_context_reborrow(),
                    expected_ty_id,
                    NormalizationMode::Assign,
                    RelationMode::EXPECTED_TYPE,
                )
            });
        let expected_is_tuple = expected_ty_id.is_some_and(|expected_ty_id| {
            matches!(
                ctx.types.get_type(expected_ty_id),
                Type::Tuple { .. } | Type::ArraySized { .. }
            )
        });
        let is_as_const = matches!(state.const_context, ConstContext::AsConst);
        let infer_tuple = expected_is_tuple || is_as_const;

        // infer element types using any contextual type
        let mut expected_element_types =
            self.expected_element_types(expected_ty_id, elements.len(), ctx.types);
        let mut expected_array_element_type =
            self.expected_array_element_type(expected_ty_id, ctx.types);

        // allow well known array references to supply element types
        if let Some(expected_ty_id) = expected_ty_id
            && let Type::Reference {
                symbol,
                generic_arguments,
            } = ctx.types.get_type(expected_ty_id).clone()
            && let Some(well_known) = self.well_known_array_kind(ctx.profile, symbol)
            && let Some(Type::Array { element, .. }) = self.normalize_well_known_type_reference(
                &mut ctx.type_context_reborrow(),
                expression_id.into_any(),
                well_known,
                generic_arguments.as_deref(),
            )
        {
            if expected_array_element_type.is_none() {
                expected_array_element_type = element;
            }
            if expected_element_types.iter().all(|ty| ty.is_none()) {
                expected_element_types.fill(element);
            }
        }

        // reject sparse array holes
        for element_id in elements {
            let element = ctx.tree.get(*element_id);
            let value_id = element.value();
            if matches!(ctx.tree.get(value_id), Expression::Stub) {
                self.error(AnalyzeError::ArrayLiteralHole {
                    node: value_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // infer element types and collect their contextualized types
        let mut element_type_ids = Vec::with_capacity(elements.len());
        for (index, element_id) in elements.iter().enumerate() {
            let expected_element_ty_id = expected_element_types.get(index).copied().flatten();
            let mut element_ctx = state
                .nested_literal_context()
                .with_expected_type(expected_element_ty_id);
            self.infer_argument(
                &mut ctx.reborrow(),
                *element_id,
                expected_element_ty_id,
                &mut element_ctx,
            )?;
            let argument = ctx.tree.get(*element_id);
            let value_id = argument.value();
            let ty_id = if let Some(ty_id) = ctx
                .infer
                .inferred_type_for_node(value_id.into_global_any(ctx.module.id))
            {
                ty_id
            } else {
                self.infer_expression(&mut ctx.reborrow(), value_id, &mut element_ctx)?
            };
            if matches!(argument, Argument::Spread { .. })
                && let Some(spread_element_type_id) =
                    self.array_spread_element_type(&mut ctx.type_context_reborrow(), ty_id)
            {
                element_type_ids.push(spread_element_type_id);
                continue;
            }
            element_type_ids.push(ty_id);
        }

        // use tuple types when an expected tuple type exists
        let ty = if infer_tuple {
            let mut tuple_elements = Vec::with_capacity(element_type_ids.len());
            for (index, element_id) in elements.iter().enumerate() {
                let argument = ctx.tree.get(*element_id);
                let mut element = TypeElement::new(element_type_ids[index]);
                if is_as_const {
                    element.is_readonly = true;
                }
                match argument {
                    Argument::Labeled { label, .. } => {
                        element.label = Some(*label);
                    }
                    Argument::Spread { .. } => {
                        element.is_rest = true;
                    }
                    _ => {}
                }
                tuple_elements.push(element);
            }

            Type::Tuple {
                elements: tuple_elements,
                is_readonly: is_as_const,
            }
        } else {
            // resolve the array element type
            let element_ty_id = if element_type_ids.is_empty() {
                expected_array_element_type
            } else {
                let source_type_id = element_type_ids[0];
                Some(self.union_type_from_list(element_type_ids, source_type_id, ctx.types))
            };

            Type::Array {
                element: element_ty_id,
                is_readonly: is_as_const,
            }
        };

        let ty_id = ctx.types.insert_type_from(ty, expression_id);

        // reject managed array types when managed memory is disabled
        if state.options.no_managed
            && !state.is_explicit_ownership
            && matches!(ctx.module.source, ModuleSource::User)
            && self.type_contains_managed(ctx.module_type_view(), ty_id)
        {
            self.error(AnalyzeError::ManagedMemoryDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        Ok(ty_id)
    }

    /// Infer a tuple literal expression.
    fn infer_tuple_literal_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<Argument>],
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer element types using any contextual type
        let expected_ty_id = self
            .expected_value_type(state.expected_type, ctx.types)
            .map(|expected_ty_id| {
                self.normalize_type_with_relation(
                    &mut ctx.type_context_reborrow(),
                    expected_ty_id,
                    NormalizationMode::Assign,
                    RelationMode::EXPECTED_TYPE,
                )
            });
        let expected_element_types =
            self.expected_element_types(expected_ty_id, elements.len(), ctx.types);

        // infer element types and collect their contextualized types
        let mut element_tys = Vec::with_capacity(elements.len());
        let is_as_const = matches!(state.const_context, ConstContext::AsConst);
        for (index, element_id) in elements.iter().enumerate() {
            let expected_element_ty_id = expected_element_types.get(index).copied().flatten();
            let mut element_ctx = state
                .nested_literal_context()
                .with_expected_type(expected_element_ty_id);
            self.infer_argument(
                &mut ctx.reborrow(),
                *element_id,
                expected_element_ty_id,
                &mut element_ctx,
            )?;
            let element = ctx.tree.get(*element_id);
            let value_id = element.value();
            let ty_id = if let Some(ty_id) = ctx
                .infer
                .inferred_type_for_node(value_id.into_global_any(ctx.module.id))
            {
                ty_id
            } else {
                self.infer_expression(&mut ctx.reborrow(), value_id, &mut element_ctx)?
            };
            let contextual_ty_id = if let Some(expected_element_ty_id) = expected_element_ty_id {
                if self
                    .is_type_assignable(
                        &mut ctx.type_context_reborrow(),
                        expected_element_ty_id,
                        ty_id,
                    )
                    .is_assignable()
                {
                    expected_element_ty_id
                } else {
                    ty_id
                }
            } else if state.expected_type.is_some() || is_as_const {
                ty_id
            } else {
                let materialize_ctx = state.for_widening_commit();
                self.widen_scalar_literal_type_if_needed(
                    ctx.module,
                    ctx.types,
                    ty_id,
                    &materialize_ctx,
                    value_id.into_any(),
                )
            };
            let mut element = TypeElement::new(contextual_ty_id);
            if is_as_const {
                element.is_readonly = true;
            }
            element_tys.push(element);
        }

        let ty = Type::Tuple {
            elements: element_tys,
            is_readonly: is_as_const,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer an object literal expression.
    fn infer_object_literal_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        properties: &[LocalNodeId<Property>],
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // apply contextual object type when available
        let expected_object_ty_id =
            self.expected_object_type(&mut ctx.reborrow(), state.expected_type)?;
        let expected_object_ty_id = expected_object_ty_id.filter(|expected_ty_id| {
            matches!(ctx.types.get_type(*expected_ty_id), Type::Object { .. })
        });
        let expected_object_ty_id = if expected_object_ty_id.is_some() {
            expected_object_ty_id
        } else {
            let options = state.options;
            self.expected_object_type_for_literal_union(
                &mut ctx.reborrow(),
                state.expected_type,
                properties,
                &options,
            )?
        };

        // record the contextual object type for query consumers
        if let Some(expected_object_ty_id) = expected_object_ty_id {
            ctx.types.set_contextual_object_type_for_node(
                expression_id.into_global_any(ctx.module.id),
                expected_object_ty_id,
            );
        }

        let (literal_fields, shapes, spread_override) = self.infer_object_literal_shapes(
            &mut ctx.reborrow(),
            properties,
            expected_object_ty_id,
            state,
        )?;

        // fall back to the contextual expected type for union excess checks
        let excess_check_ty_id = expected_object_ty_id.or(state.expected_type);

        self.check_excess_object_literal_properties(
            &mut ctx.reborrow(),
            expression_id.into_any(),
            excess_check_ty_id,
            &literal_fields,
        )?;
        if let Some(spread_override) = spread_override {
            return Ok(spread_override);
        }

        // synthesize the final object type from collected shapes
        let mut shape_ids = Vec::with_capacity(shapes.len());
        for shape in shapes {
            shape_ids.push(
                ctx.types
                    .insert_type_from(shape.into_object_type(), expression_id),
            );
        }

        let ty_id = match shape_ids.len() {
            0 => ctx.types.insert_type_from(
                Type::Object {
                    fields: Vec::new(),
                    call_signatures: Vec::new(),
                    construct_signatures: Vec::new(),
                    index_signatures: Vec::new(),
                },
                expression_id,
            ),
            1 => shape_ids[0],
            _ => ctx.types.insert_type_from(
                Type::Union {
                    elements: shape_ids,
                },
                expression_id,
            ),
        };

        // reject managed object types when managed memory is disabled
        if state.options.no_managed
            && !state.is_explicit_ownership
            && matches!(ctx.module.source, ModuleSource::User)
            && self.type_contains_managed(ctx.module_type_view(), ty_id)
        {
            self.error(AnalyzeError::ManagedMemoryDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        Ok(ty_id)
    }

    /// Infer expression variants that require infer ctx context.
    fn infer_expression_with_ctx(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        match expression {
            Expression::Unary { operator, right } => self.infer_unary_expression(
                &mut ctx.reborrow(),
                expression_id,
                operator,
                *right,
                state,
            ),
            Expression::Is { value, target_type } => self.infer_is_expression(
                &mut ctx.reborrow(),
                expression_id,
                *value,
                *target_type,
                state,
            ),
            Expression::InstanceOf { value, target } => self.infer_instanceof_expression(
                &mut ctx.reborrow(),
                expression_id,
                *value,
                *target,
                state,
            ),
            Expression::Binary {
                left,
                operator,
                right,
            } => self.infer_binary_expression(
                &mut ctx.reborrow(),
                expression_id,
                operator,
                *left,
                *right,
                state,
            ),
            Expression::Assign { left, right } => self.infer_assign_expression(
                &mut ctx.reborrow(),
                expression_id,
                *left,
                *right,
                state,
            ),
            Expression::AssignBinary {
                left,
                operator,
                right,
            } => self.infer_assign_binary_expression(
                &mut ctx.reborrow(),
                expression_id,
                operator,
                *left,
                *right,
                state,
            ),
            Expression::Call {
                left,
                generic_arguments,
                arguments,
            } => {
                let return_ty_id = self.infer_call_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    *left,
                    Some(generic_arguments.as_slice()),
                    arguments,
                    state,
                )?;

                let expects_unique_symbol = state.expected_type.is_some_and(|expected_ty_id| {
                    matches!(
                        ctx.types.get_type(expected_ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                        }
                    )
                });
                if expects_unique_symbol
                    && let Some(callee_symbol) =
                        self.reference_symbol_for_expression(ctx.tree_symbol_view(), *left)
                    && self.is_well_known_symbol(
                        ctx.profile,
                        callee_symbol,
                        WellKnownSymbol::Symbol,
                    )
                    && matches!(
                        ctx.types.get_type(return_ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::Symbol),
                        }
                    )
                {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                    };
                    Ok(ctx.types.insert_type_from(ty, expression_id))
                } else {
                    Ok(return_ty_id)
                }
            }
            Expression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let Some(name) = *name else {
                    return Ok(ctx
                        .types
                        .insert_type_from_any(Type::Error, expression_id.into_any()));
                };
                self.infer_member_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    *left,
                    name,
                    Some(generic_arguments.as_slice()),
                    state,
                )
            }
            Expression::PrivateMember {
                left,
                name,
                generic_arguments,
            } => {
                let Some(name) = *name else {
                    return Ok(ctx
                        .types
                        .insert_type_from_any(Type::Error, expression_id.into_any()));
                };
                let private_name = self.private_key_string_id(name);
                self.infer_member_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    *left,
                    private_name,
                    Some(generic_arguments.as_slice()),
                    state,
                )
            }
            Expression::Index { left, right } => self.infer_index_access_expression(
                &mut ctx.reborrow(),
                expression_id,
                *left,
                *right,
                state,
            ),
            Expression::New {
                left,
                generic_arguments,
                arguments,
            } => self.infer_new_expression(
                &mut ctx.reborrow(),
                expression_id,
                *left,
                Some(generic_arguments.as_slice()),
                arguments,
                state,
            ),
            Expression::TaggedTemplateExpression { tag, value } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_TEMPLATE);

                self.validate_call_expression(ctx, expression_id, *tag, false);
                self.infer_tagged_template_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    *tag,
                    value,
                    state,
                )
            }
            Expression::ArrayExpression { .. }
            | Expression::TupleExpression { .. }
            | Expression::ObjectExpression { .. } => self.infer_aggregate_expression(
                &mut ctx.reborrow(),
                expression_id,
                expression,
                state,
            ),
            Expression::TaggedScalarExpression { .. }
            | Expression::TaggedTupleExpression { .. }
            | Expression::TaggedObjectExpression { .. } => self.infer_tagged_literal_expression(
                &mut ctx.reborrow(),
                expression_id,
                expression,
                state,
            ),
            Expression::TreeExpression {
                left,
                arguments,
                elements,
            } => self.infer_tree_expression(
                &mut ctx.reborrow(),
                expression_id,
                *left,
                arguments.as_deref(),
                elements.as_deref(),
                state,
            ),
            Expression::Instantiation {
                left,
                generic_arguments,
            } => self.infer_instantiation_expression(
                &mut ctx.reborrow(),
                expression_id,
                *left,
                generic_arguments.as_slice(),
                state,
            ),
            Expression::SequenceExpression { .. } | Expression::Parenthesized { .. } => self
                .infer_grouping_expression(&mut ctx.reborrow(), expression_id, expression, state),
            _ => unreachable!("infer_expression_with_ctx called with unsupported expression"),
        }
    }

    destack_core::ensure_sufficient_stack! {
    /// Infer the type of an expression.
    pub(crate) fn infer_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // reuse an inferred result when caching is enabled
        let has_flow = state.flow.is_some();
        let node_id = expression_id.into_global_any(ctx.module.id);
        let expression = ctx.tree.get(expression_id);

        let cache_eligible_expression = !matches!(
            expression,
            Expression::Member { .. } | Expression::PrivateMember { .. }
        );
        if cache_eligible_expression
            && !state.is_surface_inference
            && !has_flow
            && let Some(ty_id) = self.inferred_type_id_for_node_if_ready(
                &mut ctx.reborrow(),
                node_id,
            )?
            && ctx.infer.addressability_for_node(node_id).is_some()
        {
            self.ensure_expression_addressability(&mut ctx.reborrow(), expression_id);
            return Ok(ty_id);
        }

        // resolve the flow environment for the node when flow typing is active
        let flow_environment = state.flow.as_ref().and_then(|flow_context| {
            if flow_context.module_id != ctx.module.id {
                // flow environments are module local: ignore foreign module flow contexts
                return None;
            }
            if let Some(environment_id) = flow_context.table.environment_by_node.get(&node_id) {
                return flow_context.table.environment(*environment_id).cloned();
            }

            let block_id = flow_context
                .graph
                .block_by_node
                .get(&expression_id.into_any())
                .copied()?;
            let environment_id = flow_context.table.entry_environment_for_block(block_id)?;
            flow_context.table.environment(environment_id).cloned()
        });
        if let Some(environment) = flow_environment {
            self.apply_flow_environment_to_context(&environment, state);
        }

        let ty_id: LocalTypeId = match expression {
            Expression::Declaration(..)
            | Expression::Block(..)
            | Expression::Labelled { .. }
            | Expression::Import { .. }
            | Expression::UnresolvedImport { .. }
            | Expression::ReExport { .. }
            | Expression::Export { .. }
            | Expression::UnresolvedReExport { .. }
            | Expression::ExportNamespace { .. }
            | Expression::Let { .. }
            | Expression::Using { .. } => self
                .infer_statement_expression(&mut ctx.reborrow(), expression_id, state)?,

            Expression::As { .. } | Expression::Satisfies { .. } => self
                .infer_cast_expression(&mut ctx.reborrow(), expression_id, expression, state)?,

            // expression variants that require table context
            Expression::Unary { .. }
            | Expression::Is { .. }
            | Expression::InstanceOf { .. }
            | Expression::Binary { .. }
            | Expression::Assign { .. }
            | Expression::AssignBinary { .. }
            | Expression::Call { .. }
            | Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::New { .. }
            | Expression::TaggedTemplateExpression { .. }
            | Expression::ArrayExpression { .. }
            | Expression::TupleExpression { .. }
            | Expression::ObjectExpression { .. }
            | Expression::TaggedScalarExpression { .. }
            | Expression::TaggedTupleExpression { .. }
            | Expression::TaggedObjectExpression { .. }
            | Expression::TreeExpression { .. }
            | Expression::Instantiation { .. }
            | Expression::SequenceExpression { .. }
            | Expression::Parenthesized { .. } => self
                .infer_expression_with_ctx(&mut ctx.reborrow(), expression_id, expression, state)?,

            Expression::ValueOf { .. } | Expression::ReferenceOf { .. } | Expression::PointerOf { .. } => self
                .infer_value_reference_operation_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    expression,
                    state,                )?,

            // delete operation: void
            Expression::Delete { value } => {
                self.infer_delete_expression(&mut ctx.reborrow(), expression_id, *value, state)?
            }

            // references: look up symbol type
            Expression::UnresolvedPath {
                path: _,
                generic_arguments: _,
                space_order: _,
            }
            | Expression::PrivateIdentifier { .. }
            | Expression::ImportMeta
            | Expression::This
            | Expression::Super => self.infer_special_reference_expression(
                &mut ctx.reborrow(),
                expression_id,
                expression,
                state,            )?,

            // new target: preserve syntax, but keep the type lane conservative
            Expression::NewTarget => ctx.types.insert_type_from(Type::Error, expression_id),

            // reference: symbol type
            Expression::LocalReference {
                path: _,
                target_symbol,
                generic_arguments,
            }
            | Expression::ModuleReference {
                path: _,
                target_symbol,
                generic_arguments,
            }
            | Expression::GlobalReference {
                path: _,
                target_symbol,
                generic_arguments,
            } => self.infer_reference_expression(
                &mut ctx.reborrow(),
                expression_id,
                *target_symbol,
                Some(generic_arguments.as_slice()),
                state,            )?,

            Expression::ScalarLiteral { .. }
            | Expression::TypeLiteral { .. }
            | Expression::Type { .. } => self.infer_scalar_or_type_expression(
                ctx.module,
                expression_id,
                expression,
                ctx.types,
                state,            )?,

            // control expressions
            Expression::If { .. }
            | Expression::Loop { .. }
            | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Match { .. }
            | Expression::Try { .. }
            | Expression::Return { .. }
            | Expression::Break { .. }
            | Expression::UnresolvedBreak { .. }
            | Expression::Continue { .. }
            | Expression::UnresolvedContinue { .. }
            | Expression::Throw { .. }
            | Expression::Await { .. }
            | Expression::AwaitMaybe { .. }
            | Expression::Comptime { .. }
            | Expression::Yield { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. } => self.infer_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                expression,
                state,            )?,

            // template expressions: string
            Expression::TemplateExpression { value: _ } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_TEMPLATE);

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                };
                ctx.types.insert_type_from(ty, expression_id)
            }

            // missing and error expressions: error type
            Expression::Missing | Expression::Error => {
                let ty = Type::Error;
                ctx.types.insert_type_from(ty, expression_id)
            }

            // debugger: void
            Expression::Debugger => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }

            // stub: nothing to do
            Expression::Stub => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                ctx.types.insert_type_from(ty, expression_id)
            }
        };

        // determine addressability for the expression
        self.ensure_expression_addressability(&mut ctx.reborrow(), expression_id);

        if !state.is_surface_inference {
            ctx.infer.set_inferred_type_for_node(node_id, ty_id);
        }

        Ok(ty_id)
    }
    }

    /// Infer control-flow expressions and their result types.
    fn infer_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let ty_id = match expression {
            // if expression: union of branches or common type
            Expression::If {
                kind: _,
                condition,
                then_expression,
                else_expression,
            } => self.infer_if_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                condition,
                *then_expression,
                *else_expression,
                state,
            )?,

            // loop: loop body type or never
            Expression::Loop {
                kind,
                condition,
                body,
                scope: _,
                symbol,
            } => self.infer_loop_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *kind,
                *condition,
                *body,
                *symbol,
                state,
            )?,

            // for each: void
            Expression::ForEach {
                asynchrony,
                kind,
                binding,
                iterator,
                body,
                scope: _,
                symbol,
            } => self.infer_for_each_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *asynchrony,
                *kind,
                binding,
                *iterator,
                *body,
                *symbol,
                state,
            )?,

            // for: void
            Expression::For {
                initialization,
                condition,
                increment,
                body,
                scope: _,
                symbol,
            } => self.infer_for_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *initialization,
                *condition,
                *increment,
                *body,
                *symbol,
                state,
            )?,

            // match/switch: ctx.infer case result types
            Expression::Match {
                kind,
                value,
                cases,
                source,
                scope: _,
                symbol: _,
            } => self.infer_match_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *kind,
                *value,
                cases,
                *source,
                state,
            )?,

            // try: result type
            Expression::Try {
                try_expression,
                catch_pattern,
                catch_ty,
                catch_expression,
                finally_expression,
                scope: _,
                symbol: _,
            } => self.infer_try_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *try_expression,
                *catch_pattern,
                *catch_ty,
                *catch_expression,
                *finally_expression,
                state,
            )?,

            // return: never (control flow)
            Expression::Return { value } => {
                self.infer_return_expression(&mut ctx.reborrow(), expression_id, *value, state)?
            }

            // break: never
            Expression::Break {
                target,
                target_symbol,
                value,
            } => self.infer_break_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *target,
                *target_symbol,
                *value,
                state,
            )?,
            Expression::UnresolvedBreak { target: _, value } => self
                .infer_unresolved_break_control_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    *value,
                    state,
                )?,

            // continue: never
            Expression::Continue {
                target,
                target_symbol: _,
            } => self.infer_continue_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *target,
                state,
            )?,
            Expression::UnresolvedContinue { target } => self.infer_continue_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                Some(*target),
                state,
            )?,

            // throw: never (control flow)
            Expression::Throw { value } => self.infer_throw_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *value,
                state,
            )?,

            // await: awaited type
            Expression::Await { expression } => self.infer_await_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *expression,
                state,
            )?,

            // await? should be desugared in Bind
            Expression::AwaitMaybe { .. } => {
                unreachable!("AwaitMaybe should be desugared before analysis")
            }

            // comptime: type of body (evaluated at compile time)
            Expression::Comptime { body } => {
                self.infer_expression(&mut ctx.reborrow(), *body, state)?
            }

            // yield: yielded type
            Expression::Yield { cardinality, value } => self.infer_yield_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *cardinality,
                *value,
                state,
            )?,

            // maybe unwrap: try operator
            Expression::Maybe { left } => {
                self.infer_try_unwrap_expression(&mut ctx.reborrow(), expression_id, *left, state)?
            }

            // must unwrap: must assertion
            Expression::Must { left } => self.infer_must_control_expression(
                &mut ctx.reborrow(),
                expression_id,
                *left,
                state,
            )?,

            _ => unreachable!("infer_control_expression called for non-control expression"),
        };

        Ok(ty_id)
    }

    /// Infer one `if` control expression.
    fn infer_if_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        condition: &IfCondition,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the condition
        match condition {
            IfCondition::Expression { condition } => {
                self.infer_expression(&mut ctx.reborrow(), *condition, state)?;
            }
            IfCondition::Let { declarator, .. } => {
                self.infer_declarator(
                    &mut ctx.reborrow(),
                    *declarator,
                    expression_id,
                    DeclaratorConstraint::Satisfies,
                    state,
                )?;
            }
        }

        // infer the then branch
        let mut then_ctx = state.fork().with_expected_type(state.expected_type);
        let then_ty_id =
            self.infer_expression(&mut ctx.reborrow(), then_expression, &mut then_ctx)?;

        // infer the else branch
        let else_ty_id = if let Some(else_expr) = else_expression {
            let mut else_ctx = state.fork().with_expected_type(state.expected_type);
            Some(self.infer_expression(&mut ctx.reborrow(), else_expr, &mut else_ctx)?)
        } else {
            None
        };

        // compute the result type from the branches
        if let Some(else_ty_id) = else_ty_id {
            Ok(self.best_common_type_for_list(
                &mut ctx.type_context_reborrow(),
                state,
                expression_id.into_any(),
                &[then_ty_id, else_ty_id],
            ))
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            Ok(ctx.types.insert_type_from(ty, expression_id))
        }
    }

    /// Infer one `loop` control expression.
    fn infer_loop_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        kind: LoopKind,
        condition: Option<LocalNodeId<Expression>>,
        body: LocalNodeId<Block>,
        symbol: LocalSymbolId,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(cond) = condition {
            self.infer_expression(&mut ctx.reborrow(), cond, state)?;
        }

        let loop_expected_type = state.expected_type;
        let loop_symbol = symbol.into_global(ctx.module.id);
        let mut loop_ctx = state.fork().with_expected_type(None).in_loop_with_symbol(
            expression_id.into_any(),
            loop_symbol,
            loop_expected_type,
        );
        self.infer_block(&mut ctx.reborrow(), body, &mut loop_ctx)?;

        let loop_context = loop_ctx.pop_loop_context();
        let (break_values, expected_type) = loop_context
            .map(|context| (context.break_values, context.expected_type))
            .unwrap_or_else(|| (Vec::new(), None));

        if kind == LoopKind::NoTest {
            Ok(self.loop_break_result_type(
                &mut ctx.type_context_reborrow(),
                &loop_ctx,
                expression_id.into_any(),
                &break_values,
                expected_type,
            ))
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            Ok(ctx.types.insert_type_from(ty, expression_id))
        }
    }

    /// Infer one `for each` control expression.
    fn infer_for_each_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        asynchrony: Asynchrony,
        kind: ForEachKind,
        binding: &ForEachBinding,
        iterator: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
        symbol: LocalSymbolId,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // iterator and binding
        let iterator_ty_id = self.infer_expression(&mut ctx.reborrow(), iterator, state)?;
        let binding_ty_id = self.for_each_binding_type(
            &mut ctx.type_context_reborrow(),
            iterator_ty_id,
            asynchrony,
            kind,
        );
        match binding {
            ForEachBinding::Pattern { pattern, .. } | ForEachBinding::Using { pattern, .. } => {
                self.infer_pattern(&mut ctx.reborrow(), *pattern, Some(binding_ty_id), state)?;
            }
        }

        // loop body
        let loop_expected_type = state.expected_type;
        let loop_symbol = symbol.into_global(ctx.module.id);
        let mut loop_ctx = state.fork().with_expected_type(None).in_loop_with_symbol(
            expression_id.into_any(),
            loop_symbol,
            loop_expected_type,
        );
        self.infer_block(&mut ctx.reborrow(), body, &mut loop_ctx)?;

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer one `for` control expression.
    fn infer_for_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        initialization: Option<LocalNodeId<Expression>>,
        condition: Option<LocalNodeId<Expression>>,
        increment: Option<LocalNodeId<Expression>>,
        body: LocalNodeId<Block>,
        symbol: LocalSymbolId,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(initialization) = initialization {
            self.infer_expression(&mut ctx.reborrow(), initialization, state)?;
        }
        if let Some(condition) = condition {
            self.infer_expression(&mut ctx.reborrow(), condition, state)?;
        }
        if let Some(increment) = increment {
            self.infer_expression(&mut ctx.reborrow(), increment, state)?;
        }

        let loop_expected_type = state.expected_type;
        let loop_symbol = symbol.into_global(ctx.module.id);
        let mut loop_ctx = state.fork().with_expected_type(None).in_loop_with_symbol(
            expression_id.into_any(),
            loop_symbol,
            loop_expected_type,
        );
        self.infer_block(&mut ctx.reborrow(), body, &mut loop_ctx)?;

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer one `match` or `switch` control expression.
    fn infer_match_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        kind: MatchKind,
        value: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        source: MatchSource,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let value_ty_id = self.infer_expression(&mut ctx.reborrow(), value, state)?;

        // match and switch set different break contexts
        let mut base_ctx = state.fork();
        if source == MatchSource::Match {
            if kind == MatchKind::Match {
                base_ctx = base_ctx.in_match(expression_id.into_any());
            } else {
                base_ctx = base_ctx.in_switch(expression_id.into_any());
            }
        }

        let is_switch = kind == MatchKind::Switch;
        let mut case_type_ids = Vec::new();
        for case_id in cases {
            let mut case_ctx = base_ctx.fork();
            let case = ctx.tree.get(*case_id);
            let (selector, body_expr, block_body) = match case {
                MatchCase::Expression {
                    selector,
                    body,
                    scope: _,
                } => (selector, Some(*body), None),
                MatchCase::Block {
                    selector,
                    body,
                    scope: _,
                } => (selector, None, Some(*body)),
            };

            // infer pattern and guard from selector
            if let MatchSelector::Pattern { pattern, guard } = selector {
                let mut allow_pattern_infer = true;
                if is_switch {
                    match ctx.tree.get(*pattern) {
                        Pattern::Expression { value } => {
                            if self.is_invalid_switch_case_expression(ctx.tree, *value) {
                                allow_pattern_infer = false;
                            }
                        }
                        _ => allow_pattern_infer = false,
                    }
                }
                if allow_pattern_infer {
                    let pattern_binding_ty_id = self.narrow_match_pattern_binding_type(
                        &mut ctx.type_context_reborrow(),
                        value_ty_id,
                        *pattern,
                    );
                    self.infer_pattern(
                        &mut ctx.reborrow(),
                        *pattern,
                        Some(pattern_binding_ty_id),
                        &mut case_ctx,
                    )?;
                }
                if let Some(guard_expr) = guard
                    && !is_switch
                {
                    self.infer_expression(&mut ctx.reborrow(), *guard_expr, &mut case_ctx)?;
                }
            }

            // apply contextual typing to the case body
            let expected_type = if kind == MatchKind::Match {
                base_ctx.expected_type
            } else {
                None
            };
            let mut case_ctx = case_ctx.with_expected_type(expected_type);

            // infer the case body and collect types for matches
            let case_ty_id = if let Some(expr) = body_expr {
                Some(self.infer_expression(&mut ctx.reborrow(), expr, &mut case_ctx)?)
            } else if let Some(body) = block_body {
                Some(self.infer_block(&mut ctx.reborrow(), body, &mut case_ctx)?)
            } else {
                None
            };
            if let Some(case_ty_id) = case_ty_id
                && !is_switch
            {
                case_type_ids.push(case_ty_id);
            }
        }

        if kind == MatchKind::Switch {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            Ok(ctx.types.insert_type_from(ty, expression_id))
        } else {
            Ok(match case_type_ids.len() {
                0 => {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    };
                    ctx.types.insert_type_from(ty, expression_id)
                }
                1 => case_type_ids[0],
                _ => self.best_common_type_for_list(
                    &mut ctx.type_context_reborrow(),
                    state,
                    expression_id.into_any(),
                    &case_type_ids,
                ),
            })
        }
    }

    /// Infer one `try` control expression.
    fn infer_try_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        try_expression: LocalNodeId<Expression>,
        catch_pattern: Option<LocalNodeId<Pattern>>,
        catch_ty: Option<LocalNodeId<TypeExpression>>,
        catch_expression: Option<LocalNodeId<Expression>>,
        finally_expression: Option<LocalNodeId<Expression>>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // reject try/catch forms when exceptions are disabled
        if state.options.no_exceptions {
            self.error(AnalyzeError::ExceptionsDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Never,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // validate try shape
        if catch_expression.is_none() && finally_expression.is_none() {
            self.error(AnalyzeError::IncompleteTry {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // collect try errors for catch typing
        let has_catch = catch_expression.is_some();

        // infer the try body
        let mut try_ctx = state.fork().with_expected_type(state.expected_type);
        try_ctx.push_try_frame(has_catch);
        let try_ty_id = self.infer_expression(&mut ctx.reborrow(), try_expression, &mut try_ctx)?;

        // capture try errors before entering catch
        let try_error_types = try_ctx
            .pop_try_frame()
            .map(|frame| frame.error_types)
            .unwrap_or_default();

        // infer the catch pattern and expression
        let mut catch_ty_id = None;
        if let Some(catch_expr) = catch_expression {
            let catch_binding_ty_id = if let Some(catch_ty) = catch_ty {
                Some(self.resolve_declared_type_expression(
                    &mut ctx.type_context_reborrow(),
                    catch_ty,
                    true,
                    true,
                )?)
            } else {
                None
            };

            if let Some(catch_pat) = catch_pattern {
                // infer the catch error type from try branches
                let catch_error_type_id = if let Some(catch_binding_ty_id) = catch_binding_ty_id {
                    catch_binding_ty_id
                } else if try_error_types.is_empty() {
                    let value = if state.options.use_unknown_in_catch_variables {
                        TypeLiteral::Unknown
                    } else {
                        TypeLiteral::Any
                    };
                    ctx.types
                        .insert_type_from_any(Type::TypeLiteral { value }, expression_id.into_any())
                } else {
                    let source_type_id = try_error_types[0];
                    self.union_type_from_list(try_error_types, source_type_id, ctx.types)
                };

                // bind the catch pattern to the error type
                self.infer_pattern(
                    &mut ctx.reborrow(),
                    catch_pat,
                    Some(catch_error_type_id),
                    state,
                )?;
            }

            // infer the catch expression with contextual typing
            let mut catch_ctx = state.fork().with_expected_type(state.expected_type);
            catch_ty_id =
                Some(self.infer_expression(&mut ctx.reborrow(), catch_expr, &mut catch_ctx)?);
        }

        // infer the finally expression
        if let Some(finally_expr) = finally_expression {
            let mut finally_ctx = state.fork().with_expected_type(None);
            self.infer_expression(&mut ctx.reborrow(), finally_expr, &mut finally_ctx)?;
        }

        // combine try and catch result types
        if let Some(catch_ty_id) = catch_ty_id {
            Ok(self.best_common_type_for_list(
                &mut ctx.type_context_reborrow(),
                state,
                expression_id.into_any(),
                &[try_ty_id, catch_ty_id],
            ))
        } else {
            Ok(try_ty_id)
        }
    }

    /// Infer one `break` control expression.
    fn infer_break_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        target: Option<StringId>,
        target_symbol: Option<GlobalSymbolId>,
        value: Option<LocalNodeId<Expression>>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let is_labelled = target.is_some();
        if !is_labelled && !state.can_break() {
            self.error(AnalyzeError::InvalidBreak {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                label: target,
            });
        }

        // determine the innermost break target
        let break_target = state.break_stack.last().copied();
        let is_switch_break = !is_labelled && matches!(break_target, Some(BreakTargetKind::Switch));
        if is_switch_break && value.is_some() {
            self.error(AnalyzeError::InvalidSwitchBreakValue {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        if let Some(value) = value {
            let value_ty_id = self.infer_expression(&mut ctx.reborrow(), value, state)?;
            if !is_switch_break {
                state.record_break_value(target_symbol, value_ty_id);
            }
        } else if !is_switch_break {
            let void_ty_id = ctx.types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Void,
                },
                expression_id.into_any(),
            );
            state.record_break_value(target_symbol, void_ty_id);
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Never,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer one unresolved `break` control expression.
    fn infer_unresolved_break_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        value: Option<LocalNodeId<Expression>>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(value) = value {
            self.infer_expression(&mut ctx.reborrow(), value, state)?;
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Never,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer one `continue` control expression.
    fn infer_continue_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        target: Option<StringId>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        if !state.can_continue() {
            self.error(AnalyzeError::InvalidContinue {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                label: target,
            });
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Never,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer one `throw` control expression.
    fn infer_throw_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // enforce no-exceptions mode
        if state.options.no_exceptions {
            self.error(AnalyzeError::ExceptionsDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Never,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        self.infer_expression(&mut ctx.reborrow(), value, state)?;

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Never,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer one `await` control expression.
    fn infer_await_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        inner_expression: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // reject await when runtime is disabled
        if state.options.no_runtime && matches!(ctx.module.source, ModuleSource::User) {
            self.error(AnalyzeError::RuntimeDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        if !state.can_await() {
            self.error(AnalyzeError::InvalidAwait {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        let inner_ty_id = self.infer_expression(&mut ctx.reborrow(), inner_expression, state)?;
        let awaited_ty_id = self.unwrap_awaited_type(&mut ctx.type_context_reborrow(), inner_ty_id);
        if awaited_ty_id == inner_ty_id
            && !self.type_is_semantic_top_like(inner_ty_id, ctx.types)
            && let Some(promise_ty_id) =
                self.promise_type(ctx.profile, None, expression_id.into_any(), ctx.types)
        {
            self.emit_unassignable_type_for_types(
                ctx.module_type_view(),
                expression_id.into_any(),
                promise_ty_id,
                inner_ty_id,
            );
        }

        Ok(awaited_ty_id)
    }

    /// Infer one `yield` control expression.
    fn infer_yield_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        cardinality: YieldCardinality,
        value: Option<LocalNodeId<Expression>>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // reject yield when runtime is disabled
        if state.options.no_runtime && matches!(ctx.module.source, ModuleSource::User) {
            self.error(AnalyzeError::RuntimeDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        if !state.can_yield() {
            self.error(AnalyzeError::InvalidYield {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        let mut delegate_return_type = None;
        if let Some(value_id) = value {
            let value_ty_id = self.infer_expression(&mut ctx.reborrow(), value_id, state)?;
            if let Some(expected_yield_ty_id) = state.generator_yield_type {
                if cardinality == YieldCardinality::Generator {
                    if let Some((yield_ty_id, return_ty_id)) = self
                        .yield_star_delegate_types(&mut ctx.type_context_reborrow(), value_ty_id)
                    {
                        delegate_return_type = return_ty_id;
                        if self.is_type_assignable(
                            &mut ctx.type_context_reborrow(),
                            expected_yield_ty_id,
                            yield_ty_id,
                        ) == Assignability::NotAssignable
                        {
                            self.emit_unassignable_type_for_types(
                                ctx.module_type_view(),
                                value_id.into_any(),
                                expected_yield_ty_id,
                                yield_ty_id,
                            );
                        }
                    }
                } else if self.is_type_assignable(
                    &mut ctx.type_context_reborrow(),
                    expected_yield_ty_id,
                    value_ty_id,
                ) == Assignability::NotAssignable
                {
                    self.emit_unassignable_type_for_types(
                        ctx.module_type_view(),
                        value_id.into_any(),
                        expected_yield_ty_id,
                        value_ty_id,
                    );
                }
            }
        }

        if let Some(return_ty_id) = delegate_return_type {
            Ok(return_ty_id)
        } else if let Some(next_ty_id) = state.generator_next_type {
            Ok(next_ty_id)
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            Ok(ctx.types.insert_type_from(ty, expression_id))
        }
    }

    /// Infer one `must` control expression.
    fn infer_must_control_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let left_ty_id = self.infer_expression(&mut ctx.reborrow(), left, state)?;
        let (non_nullish_ty_id, has_nullish) = self.strip_nullish_from_union(left_ty_id, ctx.types);
        if has_nullish {
            Ok(non_nullish_ty_id.unwrap_or_else(|| {
                ctx.types.insert_type_from(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    expression_id,
                )
            }))
        } else {
            Ok(left_ty_id)
        }
    }

    /// Ensure addressability is cached for an expression.
    fn ensure_expression_addressability(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> Addressability {
        let node_id = expression_id.into_global_any(ctx.module.id);
        if let Some(addressability) = ctx.infer.addressability_for_node(node_id) {
            return addressability;
        }

        // walk the expression to compute addressability
        let expression = ctx.tree.get(expression_id);
        let addressability = match expression {
            Expression::Parenthesized { expression } => {
                self.ensure_expression_addressability(&mut ctx.reborrow(), *expression)
            }
            Expression::LocalReference { .. }
            | Expression::ModuleReference { .. }
            | Expression::GlobalReference { .. }
            | Expression::This => Addressability::Place,
            Expression::Super => Addressability::Value,
            Expression::Member {
                generic_arguments, ..
            }
            | Expression::PrivateMember {
                generic_arguments, ..
            } => {
                if !generic_arguments.is_empty() {
                    Addressability::Value
                } else {
                    Addressability::Place
                }
            }
            Expression::Index { .. } => Addressability::Place,
            _ => Addressability::Value,
        };

        ctx.infer
            .set_addressability_for_node(node_id, addressability);
        addressability
    }

    /// Infer a block.
    pub(crate) fn infer_block(
        &self,
        ctx: &mut InferContext<'_>,
        block_id: LocalNodeId<Block>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let block_node_id = block_id.into_global_any(ctx.module.id);
        if !state.is_surface_inference
            && let Some(ty_id) =
                self.inferred_type_id_for_node_if_ready(&mut ctx.reborrow(), block_node_id)?
        {
            return Ok(ty_id);
        }

        let block = ctx.tree.get(block_id);

        // infer all but the last expression without contextual typing
        for expression_id in &block.leading_expressions {
            let mut expr_ctx = state.fork().with_expected_type(None);
            self.infer_expression(&mut ctx.reborrow(), *expression_id, &mut expr_ctx)?;
            self.warn_ignored_return_value(&ctx.reborrow(), *expression_id);
            state.merge_try_error_types_from(&expr_ctx);
            state.merge_break_values_from(&expr_ctx);
        }

        // infer the tail expression with contextual typing
        let ty_id = if let Some(last_expression_id) = block.tail_expression {
            let mut last_ctx = state.fork().with_expected_type(state.expected_type);
            let ty_id =
                self.infer_expression(&mut ctx.reborrow(), last_expression_id, &mut last_ctx)?;
            state.merge_try_error_types_from(&last_ctx);
            state.merge_break_values_from(&last_ctx);
            ty_id
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            ctx.types.insert_type_from(ty, block_id)
        };

        if !state.is_surface_inference {
            ctx.infer
                .set_inferred_type_for_node(block_id.into_global_any(ctx.module.id), ty_id);
        }

        Ok(ty_id)
    }

    /// Resolve contextual `this` from the current function signature.
    pub(crate) fn contextual_this_type(
        &self,
        module: &Module,
        state: &InferState,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let function_id = state.in_function?;
        let signature_ty_id =
            types.get_signature_type_for_node(function_id.into_global(module.id))?;
        let signature_ty = types.get_type(signature_ty_id);
        match signature_ty {
            Type::Function { this_parameter, .. } => *this_parameter,
            _ => None,
        }
    }

    /// Resolve the super reference type from the enclosing nominal context.
    pub(crate) fn super_type_for_context(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        state: &InferState,
    ) -> Option<LocalTypeId> {
        // super references require a valid lexical home object
        self.super_home_object_for_context(ctx.tree, expression_id, true)?;

        // resolve the enclosing class declaration
        let mut current = Some(expression_id.into_any());
        while let Some(node_id) = current {
            let Some(parent) = ctx.tree.get_parent(node_id.id) else {
                break;
            };

            if parent.ty == NodeType::Declaration {
                let declaration_id = parent.into_typed::<Declaration>();
                let declaration = ctx.tree.get(declaration_id);
                let Declaration::Class(declaration) = declaration else {
                    break;
                };

                // resolve the extends expression for the class
                let extends_expression_id = declaration.extends_expression?;

                // prefer the declared or inferred base type
                let extends_global_id = extends_expression_id.into_global_any(ctx.module.id);
                if let Some(extends_ty_id) = ctx
                    .infer
                    .inferred_type_for_node(extends_global_id)
                    .or_else(|| {
                        ctx.types
                            .get_declared_or_inferred_type_id(extends_global_id)
                    })
                {
                    return Some(extends_ty_id);
                }

                // fall back to the syntactic target symbol
                let extends_expression = ctx.tree.get(extends_expression_id);
                if let Some(target_symbol) = extends_expression.target_symbol() {
                    let super_type = Type::Reference {
                        symbol: target_symbol,
                        generic_arguments: None,
                    };
                    let super_ty_id = ctx.types.insert_type_from(super_type, expression_id);

                    return Some(super_ty_id);
                }

                break;
            }

            current = Some(parent);
        }

        // require a nominal context for super references
        let enclosing_symbol = state.in_nominal_symbol?;

        // resolve the immediate base symbol from the nominal lineage
        let base_symbol = ctx
            .types
            .get_lineage_for_symbol(enclosing_symbol)
            .and_then(|lineage| lineage.extends)?;

        // build a nominal reference to the base type
        let super_type = Type::Reference {
            symbol: base_symbol,
            generic_arguments: None,
        };
        let super_ty_id = ctx.types.insert_type_from(super_type, expression_id);

        Some(super_ty_id)
    }

    /// Infer a property and return its TypeField if it has a static key.
    pub(crate) fn infer_property(
        &self,
        ctx: &mut InferContext<'_>,
        property_id: LocalNodeId<Property>,
        expected_object_ty_id: Option<LocalTypeId>,
        state: &mut InferState,
    ) -> AnalyzeResult<Option<ObjectLiteralField>> {
        let property = ctx.tree.get(property_id);
        match property {
            Property::Field {
                key,
                value,
                symbol: _,
            } => {
                // extract the static key from the property key
                let static_key = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    *key,
                );

                // derive an expected field type from the contextual object type
                let expected_field_ty_id = static_key.as_ref().and_then(|key| {
                    self.expected_field_type(expected_object_ty_id, key, ctx.types)
                });

                // infer the value type
                let mut value_ctx = state
                    .nested_literal_context()
                    .with_expected_type(expected_field_ty_id);
                let value_ty_id =
                    self.infer_expression(&mut ctx.reborrow(), *value, &mut value_ctx)?;

                // object literal fields are only readonly under const assertions
                let is_optional = false;
                let is_readonly = matches!(state.const_context, ConstContext::AsConst);

                // static key
                if let Some(key) = static_key {
                    Ok(Some(ObjectLiteralField {
                        field: TypeField {
                            key,
                            ty: value_ty_id,
                            is_optional,
                            is_readonly,
                        },
                        property_id,
                    }))
                } else {
                    Ok(None)
                }
            }
            Property::Method {
                key,
                signature,
                body,
                symbol,
                ..
            } => {
                let expected_method_ty_id = key
                    .and_then(|key| {
                        self.static_key_from_key(
                            ctx.compiler_context.revision(),
                            ctx.profile,
                            ctx.tree,
                            ctx.symbols,
                            ctx.types,
                            key,
                        )
                    })
                    .and_then(|key| {
                        self.expected_field_type(expected_object_ty_id, &key, ctx.types)
                    });

                // enforce runtime constraints up front
                self.check_signature_runtime_constraints(
                    ctx,
                    property_id.into_any(),
                    signature,
                    state.options,
                );

                // infer the method signature with contextual typing
                let declared_signature_ty_id = ctx
                    .types
                    .get_signature_type_for_node(property_id.into_global_any(ctx.module.id));
                let method_ty_id = if self.should_use_declared_signature(
                    ctx.type_view(),
                    signature,
                    declared_signature_ty_id,
                    expected_method_ty_id,
                ) {
                    let declared_signature_ty_id = self
                        .require_declared_signature_type_for_skipped_inference(
                            declared_signature_ty_id,
                        )?;
                    self.bind_declared_signature(
                        &mut ctx.reborrow(),
                        property_id.into_any(),
                        signature,
                        declared_signature_ty_id,
                        state,
                    )?
                } else {
                    let owner_symbol = symbol.into_global(ctx.module.id);
                    self.infer_signature(
                        &mut ctx.reborrow(),
                        property_id.into_any(),
                        owner_symbol,
                        signature,
                        expected_method_ty_id,
                        declared_signature_ty_id,
                        state,
                    )?
                };

                // body
                if let Some(body) = body {
                    let return_type = self.function_return_type(method_ty_id, ctx.types);
                    let state = state
                        .reset()
                        .without_const_context()
                        .in_function_with_signature(property_id.into_any(), signature);
                    let mut context_return_type = return_type;
                    let mut state = if signature.cardinality == FunctionCardinality::Generator {
                        let (yield_ty_id, return_ty_id, next_ty_id) = self.generator_context_types(
                            &mut ctx.type_context_reborrow(),
                            property_id.into_any(),
                            return_type,
                        );
                        context_return_type = Some(return_ty_id);
                        state
                            .with_return_type(Some(return_ty_id))
                            .with_generator_types(Some(yield_ty_id), Some(next_ty_id))
                    } else {
                        state.with_return_type(return_type)
                    };
                    state = state.with_expected_type(context_return_type);

                    // infer the method body with implicit return typing
                    let body_ty_id = self.infer_body(&mut ctx.reborrow(), *body, &mut state)?;

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = context_return_type
                        && has_implicit_return(*body, ctx.tree)
                    {
                        ctx.infer.push_constraint(Constraint::Subtype {
                            sub_type: body_ty_id,
                            super_type: return_ty_id,
                            variance: None,
                        });

                        if !self.is_infer_var_type(return_ty_id, ctx.types)
                            && !self.is_infer_var_type(body_ty_id, ctx.types)
                            && self.is_type_assignable(
                                &mut ctx.type_context_reborrow(),
                                return_ty_id,
                                body_ty_id,
                            ) == Assignability::NotAssignable
                        {
                            self.emit_unassignable_type_for_types(
                                ctx.module_type_view(),
                                body.into_any(),
                                return_ty_id,
                                body_ty_id,
                            );
                        }
                    }
                }
                let static_key = key.and_then(|key| {
                    self.static_key_from_key(
                        ctx.compiler_context.revision(),
                        ctx.profile,
                        ctx.tree,
                        ctx.symbols,
                        ctx.types,
                        key,
                    )
                });

                // object literal methods are readonly only under const assertions
                let is_optional = false;
                let is_readonly = matches!(state.const_context, ConstContext::AsConst);

                if let Some(key) = static_key {
                    Ok(Some(ObjectLiteralField {
                        field: TypeField {
                            key,
                            ty: method_ty_id,
                            is_optional,
                            is_readonly,
                        },
                        property_id,
                    }))
                } else {
                    Ok(None)
                }
            }
            Property::Spread { .. } => {
                unreachable!("spread properties are handled before infer_property")
            }
            Property::Error { .. } => Ok(None),
        }
    }

    /// Get the target symbol for a reference expression.
    pub(crate) fn reference_symbol_for_expression(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        match ctx.tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => Some(self.canonical_symbol_id(
                ctx.module_symbol_view(),
                *target_symbol,
                CanonicalSymbolMode::FollowAliases,
            )),
            _ => None,
        }
    }

    /// Get the target symbol for a reference type expression.
    pub(crate) fn reference_symbol_for_type_expression(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> Option<GlobalSymbolId> {
        match ctx.tree.get(expression_id) {
            TypeExpression::LocalReference { target_symbol, .. }
            | TypeExpression::ModuleReference { target_symbol, .. }
            | TypeExpression::GlobalReference { target_symbol, .. } => {
                Some(self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    *target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                ))
            }
            _ => None,
        }
    }

    /// Peel nested parenthesized expressions to the underlying expression.
    pub(crate) fn unwrap_parenthesized_expression(
        &self,
        mut expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> LocalNodeId<Expression> {
        loop {
            let Expression::Parenthesized { expression } = tree.get(expression_id) else {
                break;
            };

            expression_id = *expression;
        }

        expression_id
    }

    /// Peel nested parenthesized type expressions to the underlying type expression.
    pub(crate) fn unwrap_parenthesized_type_expression(
        &self,
        mut expression_id: LocalNodeId<TypeExpression>,
        tree: &NodeTree,
    ) -> LocalNodeId<TypeExpression> {
        loop {
            let TypeExpression::Parenthesized { expression } = tree.get(expression_id) else {
                break;
            };

            expression_id = *expression;
        }

        expression_id
    }

    /// Resolve a global symbol name with one explicit local symbol table when available.
    pub(crate) fn symbol_name_for_global_in(
        &self,
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<StringId> {
        self.with_module_symbols_or_local_for_artifact(
            view.compiler_context,
            view.module,
            view.profile,
            symbol.module_id,
            view.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |_, symbols| symbols.get_symbol(symbol.local_id).name(),
        )
        .ok()
        .flatten()
    }

    /// Resolve the dependency item that introduced a symbol when possible.
    pub(crate) fn dependency_item_for_symbol(
        &self,
        tree: &NodeTree,
        primary_declaration: GlobalNodeIdAny,
        symbol_id: LocalSymbolId,
    ) -> Option<LocalNodeId<DependencyItem>> {
        // return dependency items directly
        if primary_declaration.local_id.ty == NodeType::DependencyItem {
            return Some(primary_declaration.local_id.into_typed());
        }

        // scan import/export expressions for matching dependency items
        if primary_declaration.local_id.ty != NodeType::Expression {
            return None;
        }
        let expression_id = primary_declaration.local_id.into_typed::<Expression>();
        let items: &[LocalNodeId<DependencyItem>] = match tree.get(expression_id) {
            Expression::Import { items, .. } => items.as_deref().unwrap_or(&[]),
            Expression::ReExport { items, .. } | Expression::Export { items, .. } => items,
            _ => return None,
        };

        for item_id in items {
            if tree.get(*item_id).symbol() == Some(symbol_id) {
                return Some(*item_id);
            }
        }

        None
    }

    /// Check whether a remote export is type-only for a specific name.
    fn is_type_only_export_name(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
        export_name: StringId,
    ) -> bool {
        let Some(exports) = self
            .require_artifact_dir_interface(revision, module_id, profile)
            .ok()
            .map(|snapshot| snapshot.exported_symbols.clone())
        else {
            return false;
        };

        let key = StaticKey::Name(export_name);
        let has_value = exports.contains_key(&(SymbolSpace::Value, key));
        let has_type = exports.contains_key(&(SymbolSpace::Type, key));
        has_type && !has_value
    }

    /// Emit a type-only value error and return an error type id.
    fn type_only_value_error_type(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalTypeId {
        let error = AnalyzeError::TypeOnlyValue {
            node: expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
        };
        self.report_infer_type_error(ctx.types, expression_id.into_any(), error)
    }

    /// Report one infer diagnostic and return an error type id for the source node.
    pub(crate) fn report_infer_type_error(
        &self,
        types: &mut TypeTable,
        node_id: LocalNodeIdAny,
        error: AnalyzeError,
    ) -> LocalTypeId {
        self.error(error);
        types.insert_type_from_any(Type::Error, node_id)
    }

    /// Resolve export name and module id for a remote dependency item.
    fn remote_dependency_export_target(
        &self,
        module: &Module,
        profile: ProfileId,
        dependency: &DependencyItem,
    ) -> (Option<StringId>, Option<ModuleId>) {
        match dependency {
            DependencyItem::Remote {
                mode,
                name,
                target: _,
                target_module,
                ..
            } => {
                let default_name = self.repository.strings.intern("default");
                let export_name = match mode {
                    DependencyMode::Item => name.map(|name| name.string()),
                    DependencyMode::Default => {
                        name.map(|name| name.string()).or(Some(default_name))
                    }
                    DependencyMode::Namespace => None,
                };
                let target_module_id = target_module
                    .ty
                    .or(target_module.value)
                    .and_then(|target| target.module_id());
                (export_name, target_module_id)
            }
            DependencyItem::UnresolvedRemote {
                mode,
                name,
                target,
                target_module,
                ..
            } => {
                let default_name = self.repository.strings.intern("default");
                let export_name = match mode {
                    DependencyMode::Item => name.map(|name| name.string()),
                    DependencyMode::Default => {
                        name.map(|name| name.string()).or(Some(default_name))
                    }
                    DependencyMode::Namespace => None,
                };
                let target_module_id = target_module
                    .or_else(|| {
                        self.imported_module_resolution_for_specifier(
                            module.id,
                            profile,
                            None,
                            *target,
                            ModuleEdgeRelation::Import,
                            None,
                        )
                    })
                    .and_then(|targets| targets.ty.or(targets.value))
                    .and_then(|target| target.module_id());
                (export_name, target_module_id)
            }
            _ => (None, None),
        }
    }

    /// Infer a reference expression (local, module, or global).
    pub(crate) fn infer_reference_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        generic_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        state: &InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_REFERENCE);
        let mut allow_type_only_reference = false;
        // validate local references against type-only exports
        if target_symbol.module_id == ctx.module.id {
            let symbol_entry = ctx.symbols.get_symbol(target_symbol.local_id);
            let dependency_id = symbol_entry
                .primary_declaration
                .and_then(|primary_declaration| {
                    self.dependency_item_for_symbol(
                        ctx.tree,
                        primary_declaration,
                        target_symbol.local_id,
                    )
                });
            allow_type_only_reference =
                ctx.module.language_type.is_declaration() && dependency_id.is_some();

            // reject type-only symbols used as values
            if !self.symbol_is_value_capable(ctx.profile, target_symbol)
                && !allow_type_only_reference
            {
                let ty_id = self.type_only_value_error_type(&mut ctx.reborrow(), expression_id);
                return Ok(ty_id);
            }

            // reject aliases that resolve to type-only symbols
            if let Some(target) = symbol_entry.target_symbol
                && !self.symbol_is_value_capable(ctx.profile, target)
                && !allow_type_only_reference
            {
                let ty_id = self.type_only_value_error_type(&mut ctx.reborrow(), expression_id);
                return Ok(ty_id);
            }

            // resolve the dependency item that introduced this symbol
            if let Some(dependency_id) = dependency_id {
                let dependency = ctx.tree.get(dependency_id);
                let dependency_kind = match dependency {
                    DependencyItem::Local { kind, .. }
                    | DependencyItem::Remote { kind, .. }
                    | DependencyItem::UnresolvedLocal { kind, .. }
                    | DependencyItem::UnresolvedRemote { kind, .. } => Some(*kind),
                    DependencyItem::Value { .. } | DependencyItem::Error => None,
                };

                // reject type-only dependencies used in value positions
                let is_type_only_dependency = match dependency_kind {
                    Some(DependencyKind::Type) => true,
                    Some(DependencyKind::Value) => dependency
                        .target_symbol()
                        .is_some_and(|target| !self.symbol_is_value_capable(ctx.profile, target)),
                    None => false,
                };
                if is_type_only_dependency && !allow_type_only_reference {
                    let ty_id = self.type_only_value_error_type(&mut ctx.reborrow(), expression_id);
                    return Ok(ty_id);
                }

                // reject value imports that resolve to type-only exports
                if dependency_kind == Some(DependencyKind::Value) {
                    let (export_name, target_module_id) =
                        self.remote_dependency_export_target(ctx.module, state.profile, dependency);

                    if let Some(export_name) = export_name
                        && let Some(target_module_id) = target_module_id
                        && self.is_type_only_export_name(
                            ctx.compiler_context.revision(),
                            target_module_id,
                            state.profile,
                            export_name,
                        )
                        && !allow_type_only_reference
                    {
                        let is_value_capable = dependency.target_symbol().is_some_and(|target| {
                            self.symbol_is_value_capable(ctx.profile, target)
                        });
                        if !is_value_capable {
                            let ty_id =
                                self.type_only_value_error_type(&mut ctx.reborrow(), expression_id);
                            return Ok(ty_id);
                        }
                    }
                }
            }

            // fall back to export ctx when dependency items are missing
            if dependency_id.is_none()
                && let Some(target_symbol) = symbol_entry.target_symbol
                && let Some(StaticKey::Name(name)) = symbol_entry.key
                && self.is_type_only_export_name(
                    ctx.compiler_context.revision(),
                    target_symbol.module_id,
                    state.profile,
                    name,
                )
                && !allow_type_only_reference
            {
                let ty_id = self.type_only_value_error_type(&mut ctx.reborrow(), expression_id);
                return Ok(ty_id);
            }
        }

        // resolve the canonical symbol for imported references
        let canonical_symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        // reject type-only symbols in value positions
        if !self.symbol_is_value_capable(ctx.profile, canonical_symbol)
            && !allow_type_only_reference
        {
            let ty_id = self.type_only_value_error_type(&mut ctx.reborrow(), expression_id);
            return Ok(ty_id);
        }

        // reject globalThis references when configured
        if state.options.no_global_this && matches!(ctx.module.source, ModuleSource::User) {
            let global_this_name = self.repository.strings.intern("globalThis");
            if self.symbol_name_for_global_in(ctx.module_symbol_view(), canonical_symbol)
                == Some(global_this_name)
            {
                self.error(AnalyzeError::GlobalThisDisabled {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // synthesize a globalThis object type on demand
        let global_this_name = self.repository.strings.intern("globalThis");
        let is_global_this = self
            .symbol_name_for_global_in(ctx.module_symbol_view(), canonical_symbol)
            == Some(global_this_name);

        // pick the base type for the symbol by applying narrowing and inference
        let narrowed_ty_id =
            self.resolved_narrowed_type_for_symbol(&mut ctx.reborrow(), canonical_symbol, state)?;
        let base_ty_id = if let Some(narrowed_ty_id) = narrowed_ty_id {
            narrowed_ty_id
        } else if is_global_this
            && let Some(global_this_ty_id) = self.infer_global_this_value_type(
                &mut ctx.reborrow(),
                expression_id,
                canonical_symbol,
                state,
            )?
        {
            global_this_ty_id
        } else if canonical_symbol.module_id != ctx.module.id {
            if let Some(value_ty_id) = ctx.types.get_value_type_id(canonical_symbol)
                && !self.unwrapped_value_type_is_unevaluated(value_ty_id, ctx.types)
            {
                value_ty_id
            } else {
                self.resolve_remote_symbol_value_type_for_context(
                    &mut ctx.reborrow(),
                    state,
                    expression_id.into_any(),
                    canonical_symbol,
                )?
            }
        } else if let Some(value_ty_id) = ctx.types.get_value_type_id(canonical_symbol) {
            if !self.unwrapped_value_type_is_unevaluated(value_ty_id, ctx.types) {
                value_ty_id
            } else if let Some(inferred_ty_id) =
                self.infer_direct_binding_value_type(&mut ctx.reborrow(), canonical_symbol, state)?
            {
                inferred_ty_id
            } else {
                value_ty_id
            }
        } else if let Some(inferred_ty_id) =
            self.infer_direct_binding_value_type(&mut ctx.reborrow(), canonical_symbol, state)?
        {
            inferred_ty_id
        } else {
            // local symbol without type: use InferVar for forward references
            let scope = InferScope {
                owner: canonical_symbol,
                function_id: state.in_function.map(|f| f.into_global(ctx.module.id)),
            };
            self.infer_var_type_for_symbol(
                ctx.infer,
                ctx.types,
                canonical_symbol,
                expression_id.into_any(),
                InferOrigin::Expression(expression_id.into_global_any(ctx.module.id)),
                scope,
            )
        };
        // evaluate local unevaluated types before use
        if canonical_symbol.module_id == ctx.module.id {
            let base_value_ty_id = ctx.types.unwrap_value_type_id(base_ty_id);
            if matches!(ctx.types.get_type(base_value_ty_id), Type::Unevaluated(_)) {
                self.resolve_declared_type(&mut ctx.type_context_reborrow(), base_value_ty_id)?;
            }
        }

        // ensure instance types for referenced symbols
        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            base_ty_id,
        )?;

        // handle static arguments for generic instantiation
        let Some(generic_argument_ids) = generic_arguments else {
            return Ok(base_ty_id);
        };

        self.instantiate_callable_type_with_static_arguments(
            &mut ctx.reborrow(),
            expression_id,
            base_ty_id,
            Some(canonical_symbol),
            generic_argument_ids,
        )
    }

    /// Check excess properties on an object literal against a contextual type.
    fn check_excess_object_literal_properties(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        expected_ty_id: Option<LocalTypeId>,
        fields: &[ObjectLiteralField],
    ) -> AnalyzeResult<()> {
        // resolve the contextual target type for excess property checks
        let Some(expected_ty_id) = self.expected_value_type(expected_ty_id, ctx.types) else {
            return Ok(());
        };

        // skip excess checks for record-like literal targets
        if self.is_record_like_object_literal_target(ctx.profile, expected_ty_id, ctx.types) {
            return Ok(());
        }

        let mut candidates: Vec<LocalTypeId> = Vec::new();
        let mut visited = HashSet::new();
        self.collect_object_literal_candidates(
            &mut ctx.reborrow(),
            node_id,
            expected_ty_id,
            &mut candidates,
            &mut visited,
        )?;
        if candidates.is_empty() {
            return Ok(());
        }

        // check if any candidate matches the fields
        for candidate in candidates.iter().copied() {
            if self.object_literal_matches_target(fields, candidate, ctx.types) {
                return Ok(());
            }
        }

        // if no candidate matches, report the first excess property
        let Some(candidate) = candidates.first().copied() else {
            return Ok(());
        };
        let excess_fields = self.object_literal_excess_properties(fields, candidate, ctx.types);
        if excess_fields.is_empty() {
            return Ok(());
        }

        // emit excess property diagnostics
        for (property_id, member_key) in excess_fields {
            self.report_excess_property_for_type(
                ctx.module_type_view(),
                property_id.into_any(),
                candidate,
                member_key,
            );
        }

        Ok(())
    }

    /// Check if an object literal target is a record-like alias.
    fn is_record_like_object_literal_target(
        &self,
        profile: ProfileId,
        target_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        // locate the map and record symbols
        let map_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Map);
        let record_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Record);
        let Some(map_symbol) = map_symbol else {
            return false;
        };

        // walk aliases to find map and record references
        let mut current = target_ty_id;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current) {
                return false;
            }

            match types.get_type(current) {
                // unwrap value wrapper types
                Type::Value { value } => {
                    current = *value;
                }
                // accept direct map and record references
                Type::Reference { symbol, .. } => {
                    if *symbol == map_symbol
                        || record_symbol.is_some_and(|record_symbol| record_symbol == *symbol)
                    {
                        return true;
                    }

                    // follow alias targets when available
                    if let Some(alias_target) = types.get_alias_target_type_id(*symbol) {
                        current = alias_target;
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }

    /// Collect object style candidates for excess property checks.
    fn collect_object_literal_candidates(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        candidates: &mut Vec<LocalTypeId>,
        visited: &mut HashSet<LocalTypeId>,
    ) -> AnalyzeResult<()> {
        // avoid recursive candidate discovery loops
        if !visited.insert(expected_ty_id) {
            return Ok(());
        }

        match ctx.types.get_type(expected_ty_id).clone() {
            Type::Object { .. } => {
                candidates.push(expected_ty_id);
            }
            Type::Reference {
                symbol,
                generic_arguments,
            } => {
                let instance_ty_id = self.resolve_instance_type_for_symbol(
                    &mut ctx.type_context_reborrow(),
                    node_id,
                    symbol,
                )?;
                if let Some(instance_ty_id) = instance_ty_id {
                    let mut candidate_id = instance_ty_id;

                    // specialize instance types with explicit static arguments
                    if let Some(generic_arguments) = generic_arguments.as_ref()
                        && let Some(resolved_arguments) = self
                            .resolve_type_reference_static_arguments(
                                &mut ctx.type_context_reborrow(),
                                node_id,
                                symbol,
                                Some(generic_arguments.as_slice()),
                                true,
                            )?
                        && !resolved_arguments.is_empty()
                    {
                        let substitutions = self.build_type_parameter_substitutions_for_symbol(
                            &mut ctx.type_context_reborrow(),
                            symbol,
                            node_id,
                            &resolved_arguments,
                        );
                        if !substitutions.is_empty() {
                            let mut cache = HashMap::new();
                            candidate_id = self.substitute_static_parameters(
                                instance_ty_id,
                                &substitutions,
                                ctx.types,
                                &mut cache,
                            );
                        }
                    }

                    let normalized = self.normalize_type(
                        &mut ctx.type_context_reborrow(),
                        candidate_id,
                        NormalizationMode::Assign,
                    );
                    candidates.push(normalized);
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                // expand mapped types into object candidates
                let source_id = ctx.types.get_type_source(expected_ty_id);
                let mut normalize_visited = Vec::new();
                let normalized = self.normalize_mapped_type(
                    &mut ctx.type_context_reborrow(),
                    source_id,
                    parameter,
                    modifiers,
                    value,
                    NormalizationMode::Assign,
                    RelationMode::OBJECT_SHAPE,
                    &mut normalize_visited,
                );

                // stop when mapped normalization does not make structural progress
                let normalized_type = ctx.types.get_type(normalized).clone();
                let expected_type = ctx.types.get_type(expected_ty_id).clone();
                if normalized == expected_ty_id || normalized_type == expected_type {
                    return Ok(());
                }

                self.collect_object_literal_candidates(
                    &mut ctx.reborrow(),
                    node_id,
                    normalized,
                    candidates,
                    visited,
                )?;
            }
            Type::Union { elements } => {
                for element in elements {
                    self.collect_object_literal_candidates(
                        &mut ctx.reborrow(),
                        node_id,
                        element,
                        candidates,
                        visited,
                    )?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if an object literal matches a target object type.
    fn object_literal_matches_target(
        &self,
        fields: &[ObjectLiteralField],
        target_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let Type::Object {
            fields: target_fields,
            index_signatures,
            ..
        } = types.get_type(target_ty_id)
        else {
            return false;
        };
        if !index_signatures.is_empty() {
            return true;
        }

        for field in fields {
            let matches = target_fields
                .iter()
                .any(|target_field| target_field.key.matches(&field.field.key));
            if !matches {
                return false;
            }
        }

        true
    }

    /// Find the first excess property key for a target object type.
    fn object_literal_excess_properties(
        &self,
        fields: &[ObjectLiteralField],
        target_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Vec<(LocalNodeId<Property>, StaticKey)> {
        let Type::Object {
            fields: target_fields,
            index_signatures,
            ..
        } = types.get_type(target_ty_id)
        else {
            return Vec::new();
        };
        if !index_signatures.is_empty() {
            return Vec::new();
        }

        let mut excess_fields = Vec::new();
        for field in fields {
            let matches = target_fields
                .iter()
                .any(|target_field| target_field.key.matches(&field.field.key));
            if !matches {
                excess_fields.push((field.property_id, field.field.key));
            }
        }

        excess_fields
    }

    /// Validate that a comptime body is static when required.
    /// Infer a return expression and constrain it to the function return type.
    fn infer_return_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        value: Option<LocalNodeId<Expression>>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // validate return position
        if !state.can_return() {
            self.error(AnalyzeError::InvalidReturn {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // infer the return value when present
        if let Some(value_id) = value {
            let mut return_ctx = state.fork().with_expected_type(state.return_type);
            let value_ty_id =
                self.infer_expression(&mut ctx.reborrow(), value_id, &mut return_ctx)?;

            if let Some(return_ty_id) = state.return_type {
                // enforce explicit ownership when implicit managed values are disabled
                self.check_no_implicit_managed_value(
                    ctx.module_type_view(),
                    value_id,
                    return_ty_id,
                    value_ty_id,
                    ctx.tree,
                    &state.options,
                );

                // constrain the return value to the declared return type
                let options = state.options;
                self.constrain_return_value_type(
                    &mut ctx.reborrow(),
                    value_id,
                    return_ty_id,
                    value_ty_id,
                    &options,
                    state.is_async,
                );
            }
        } else if let Some(return_ty_id) = state.return_type {
            let void_ty_id = ctx.types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Void,
                },
                expression_id.into_any(),
            );
            ctx.infer.push_constraint(Constraint::Subtype {
                sub_type: void_ty_id,
                super_type: return_ty_id,
                variance: None,
            });
        }

        // return expressions always end control flow
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Never,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Constrain a return value to the declared return type.
    fn constrain_return_value_type(
        &self,
        ctx: &mut InferContext<'_>,
        value_id: LocalNodeId<Expression>,
        return_ty_id: LocalTypeId,
        value_ty_id: LocalTypeId,
        _options: &AnalyzeOptions,
        is_async: bool,
    ) {
        // unwrap awaited values for async returns
        let (check_value_ty_id, check_return_ty_id) = if is_async {
            (
                self.unwrap_awaited_type(&mut ctx.type_context_reborrow(), value_ty_id),
                self.unwrap_awaited_type(&mut ctx.type_context_reborrow(), return_ty_id),
            )
        } else {
            (value_ty_id, return_ty_id)
        };

        // detect return types that should skip assignability errors
        let mut static_visited = HashSet::new();
        let has_static_parameters = self.type_contains_static_parameters(
            ctx.type_view(),
            check_return_ty_id,
            &mut static_visited,
        );
        let mut infer_visited = HashSet::new();
        let has_infer_vars =
            self.type_contains_infer_vars(check_return_ty_id, ctx.types, &mut infer_visited);
        let allows_fallthrough =
            self.return_type_allows_fallthrough_infer(check_return_ty_id, ctx.types);
        let source_expression_node = value_id.into_global_any(ctx.module.id);
        let source_expression_type_id = ctx
            .infer
            .inferred_type_for_node(source_expression_node)
            .or_else(|| ctx.types.get_inferred_type_id(source_expression_node));
        let source_expression_requires_convergence =
            source_expression_type_id.is_some_and(|source_type_id| {
                self.type_requires_infer_convergence(ctx.type_view(), source_type_id)
            });
        let source_expression_is_placeholder =
            source_expression_type_id.is_some_and(|source_type_id| {
                self.type_is_solver_placeholder(source_type_id, ctx.types)
            });
        let source_expression_is_unknown = matches!(
            ctx.types.get_type(check_value_ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            }
        );
        let should_defer_return_diagnostic = source_expression_requires_convergence
            || (source_expression_is_unknown && source_expression_is_placeholder);

        // emit the return type error when the assignment is invalid
        if !has_static_parameters
            && !has_infer_vars
            && !allows_fallthrough
            && self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                check_return_ty_id,
                check_value_ty_id,
            ) == Assignability::NotAssignable
        {
            if should_defer_return_diagnostic {
                self.push_relation_obligation_for_target_type_and_source_expression(
                    ctx.module,
                    value_id.into_any(),
                    check_return_ty_id,
                    value_id,
                    TypeRelationObligationDiagnostic::UnassignableType,
                    ctx.infer,
                );
            } else {
                self.emit_unassignable_type_for_types(
                    ctx.module_type_view(),
                    value_id.into_any(),
                    check_return_ty_id,
                    check_value_ty_id,
                );
            }
        }

        ctx.infer.push_constraint(Constraint::Subtype {
            sub_type: check_value_ty_id,
            super_type: check_return_ty_id,
            variance: None,
        });
    }

    fn warn_ignored_return_value(
        &self,
        ctx: &InferContext<'_>,
        statement_id: LocalNodeId<Expression>,
    ) {
        // only warn for call like statements
        let statement = ctx.tree.get(statement_id);
        if !matches!(statement, Expression::Call { .. } | Expression::New { .. }) {
            return;
        }

        // look up the resolution for the statement
        let node_id = statement_id.into_global_any(ctx.module.id);
        let Some(resolution) = self.query_resolution_for_node_infer(node_id, ctx.infer, ctx.types)
        else {
            return;
        };

        // check for mustUse targets
        let mut should_warn = false;
        for symbol_id in self.resolution_target_symbols(resolution) {
            let Some(decorators) = self.symbol_decorators_for(ctx.module_symbol_view(), symbol_id)
            else {
                continue;
            };
            if decorators.is_must_use {
                should_warn = true;
                break;
            }
        }
        if !should_warn {
            return;
        }

        // emit ignored return value warning
        self.warning(AnalyzeWarning::IgnoredReturnValue {
            node: node_id.into_anchored(Some(ctx.profile)),
        });
    }

    fn resolution_target_symbols(&self, resolution: &Resolution) -> Vec<GlobalSymbolId> {
        // collect target symbols for static or dynamic resolutions
        match resolution {
            Resolution::Static { candidate, .. } => vec![candidate.target_symbol],
            Resolution::Dynamic { candidates, .. } => candidates
                .iter()
                .map(|candidate| candidate.target_symbol)
                .collect(),
            Resolution::Unresolved { .. } | Resolution::Builtin { .. } => Vec::new(),
        }
    }

    fn symbol_decorators_for(
        &self,
        view: ModuleSymbolView<'_>,
        symbol_id: GlobalSymbolId,
    ) -> Option<SymbolDecorators> {
        // read local symbol decorators
        if symbol_id.module_id == view.module.id {
            let symbol = view.symbols.get_symbol(symbol_id.local_id);
            return Some(symbol.decorators.clone());
        }

        // read decorators from the target module artifact
        self.require_dir_resolved(
            view.compiler_context.revision(),
            symbol_id.module_id,
            view.profile,
        )
        .ok()?;

        let snapshot = self
            .require_artifact_dir_resolved(
                view.compiler_context.revision(),
                symbol_id.module_id,
                view.profile,
            )
            .ok()?;
        let symbol = snapshot.symbols.get_symbol(symbol_id.local_id);

        Some(symbol.decorators.clone())
    }
}

/// Return the implicit return expression for one body when present.
pub(crate) fn implicit_return_expression(
    expression_id: LocalNodeId<Expression>,
    tree: &NodeTree,
) -> Option<LocalNodeId<Expression>> {
    // treat statement-like expressions as non-returning values
    match tree.get(expression_id) {
        Expression::Return { .. }
        | Expression::Break { .. }
        | Expression::Continue { .. }
        | Expression::If {
            else_expression: None,
            ..
        } => None,
        Expression::Block(block) => {
            // read the block expression list
            let block = tree.get(*block);
            let last_expression_id = block.tail_expression?;

            // ignore statement-like trailing expressions
            if matches!(
                tree.get(last_expression_id),
                Expression::Return { .. }
                    | Expression::Break { .. }
                    | Expression::Continue { .. }
                    | Expression::If {
                        else_expression: None,
                        ..
                    }
            ) {
                return None;
            }

            Some(last_expression_id)
        }
        _ => Some(expression_id),
    }
}

/// Check whether an expression participates in implicit return typing.
pub(crate) fn has_implicit_return(expression_id: LocalNodeId<Expression>, tree: &NodeTree) -> bool {
    implicit_return_expression(expression_id, tree).is_some()
}
