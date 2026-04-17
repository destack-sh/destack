use std::collections::{HashMap, HashSet};

use super::SignatureResolutionMode;
use super::argument::StaticArgumentValidationMode;
use super::member::{MemberLookupMode, MemberResolution};
use crate::analyze::StaticSubstitutionEnvironment;
use crate::analyze::common::{
    AnalyzeIndex, CanonicalSymbolMode, InferContext, TreeSymbolView, TypeView,
};
use crate::analyze::infer::RemoteValueTypeReadDomain;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferState};
use destack_dir::{
    Argument, Constraint, Declaration, DispatchKey, DynamicResolutionCandidateSlotId, Expression,
    FunctionKind, FunctionMode, GenericArgument, GenericParameterKind, GenericParameterSpec,
    GlobalSymbolId, InferOrigin, InferTable, LocalInstanceId, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, Member, NodeTree, NodeType, Parameter, Property, ResolutionCandidate,
    ResolvedSignature, StaticArgument, StaticExpression, StaticKey, StringId, SymbolType, Type,
    TypeExpression, TypeLiteral, TypeTable,
};
use destack_workspace::ModuleSource;

/// Resolved member function for an operator invocation.
#[derive(Debug)]
pub(crate) struct ResolvedMemberFunction {
    /// The resolved function signature (parameters and return type).
    pub(crate) signature: ResolvedSignature,
    /// The member resolution for dispatch recording.
    pub(crate) member_resolution: MemberResolution,
    /// The resolved member symbol (if statically known).
    pub(crate) member_symbol: Option<GlobalSymbolId>,
    /// Static arguments used for the member instance.
    pub(crate) instance_arguments: Vec<StaticArgument>,
    /// Static parameter symbols of the resolved signature in argument order.
    pub(crate) signature_static_parameter_symbols: Vec<GlobalSymbolId>,
    /// Bound substitutions already represented by instance base arguments.
    pub(crate) bound_substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
    /// Whether the member was found on the receiver type.
    pub(crate) has_member: bool,
}

/// Candidate data for union member call dispatch.
#[derive(Debug)]
struct UnionMemberCallCandidate {
    /// The union element type used for dispatch.
    receiver_ty_id: LocalTypeId,
    /// The member symbol resolved for this union element.
    symbol: GlobalSymbolId,
    /// The resolved call signature for this candidate.
    signature: ResolvedSignature,
    /// The substitution environment for instancing this member.
    instance_environment: Option<StaticSubstitutionEnvironment>,
}

/// Context for member calls inferred from a call expression.
#[derive(Debug)]
struct MemberCallContext {
    /// The receiver expression id for the member call.
    receiver_id: LocalNodeId<Expression>,
    /// The inferred receiver type id.
    receiver_ty_id: LocalTypeId,
    /// The member key used for lookup.
    member_key: StaticKey,
}

/// Queried callee state for call-expression inference.
#[derive(Debug)]
enum CallExpressionCalleeQuery {
    /// The call result can be resolved immediately.
    EarlyType(LocalTypeId),
    /// The call requires normal signature resolution.
    Callee(CallExpressionCallee),
}

/// Normalized callee metadata used for call inference.
#[derive(Debug)]
struct CallExpressionCallee {
    /// The normalized callee type id.
    callee_ty_id: LocalTypeId,
    /// Whether optional chaining introduced a nullish receiver branch.
    has_optional_nullish: bool,
}

/// Resolved target metadata for call-expression inference.
#[derive(Debug)]
struct CallExpressionResolution {
    /// The resolved callee symbol when statically known.
    callee_symbol: Option<GlobalSymbolId>,
    /// The receiver type for member calls.
    call_receiver_ty_id: Option<LocalTypeId>,
    /// Member resolution metadata for member calls.
    call_member_resolution: Option<MemberResolution>,
    /// Member-call lookup context when the callee is a member expression.
    member_call_context: Option<MemberCallContext>,
    /// Static arguments supplied on the member expression itself.
    member_static_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
    /// Inherited static arguments from the receiver.
    inherited_static_arguments: Vec<StaticArgument>,
    /// Inherited substitutions from the receiver.
    inherited_substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
    /// Instance arguments inferred from member static arguments.
    member_instance_arguments: Option<Vec<StaticArgument>>,
    /// Prefilled static arguments from extension context.
    prefilled_static_arguments: Option<Vec<StaticArgument>>,
    /// Super-constructor value type when applicable.
    super_constructor_value_ty_id: Option<LocalTypeId>,
    /// Whether call-level and member-level static arguments conflict.
    has_static_argument_conflict: bool,
    /// Whether the call expression supplied static arguments.
    call_has_static_arguments: bool,
}

/// Resolved target metadata for new-expression inference.
#[derive(Debug)]
struct NewExpressionResolution {
    /// The inferred callee type for the constructor target.
    callee_ty_id: LocalTypeId,
    /// The resolved callee symbol when statically known.
    callee_symbol: Option<GlobalSymbolId>,
    /// Whether the constructor target is a struct constructor.
    is_struct_constructor: bool,
}

/// Resolved member-call typing context shared by call-resolution paths.
#[derive(Debug)]
struct ResolvedMemberCallTypeContext {
    /// The inferred member type after substitution.
    member_ty_id: Option<LocalTypeId>,
    /// The merged substitutions inherited from receiver and extension context.
    substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
    /// Base instance arguments before call-signature static argument suffixes.
    instance_arguments: Vec<StaticArgument>,
    /// Extension-sourced static arguments used during signature resolution.
    extension_arguments: Option<Vec<StaticArgument>>,
}

/// Shared context for resolving union member-call candidates.
#[derive(Debug)]
struct UnionMemberCallResolutionContext<'a> {
    /// The current module tree and symbol view.
    tree_symbols: TreeSymbolView<'a>,
    /// The task-local analyze index.
    index: AnalyzeIndex,
    /// The call expression being inferred.
    expression_id: LocalNodeId<Expression>,
    /// The receiver expression of the member call.
    receiver_expression_id: LocalNodeId<Expression>,
    /// The union receiver type for diagnostics.
    receiver_union_ty_id: LocalTypeId,
    /// The nominal receiver symbol from syntactic receiver context.
    receiver_nominal_symbol: Option<GlobalSymbolId>,
    /// The member key used for lookup.
    member_key: &'a StaticKey,
    /// Static arguments used for member-call signature resolution.
    generic_arguments: Option<&'a [LocalNodeId<GenericArgument>]>,
    /// Dynamic arguments used for overload selection.
    arguments: &'a [LocalNodeId<Argument>],
}

/// Call-site context for resolving one function signature's static arguments.
#[derive(Debug)]
pub(crate) struct SignatureStaticResolutionContext<'a> {
    /// The syntax node that anchors diagnostics and synthetic types.
    pub(crate) node_id: LocalNodeIdAny,
    /// The declaration owner symbol when available.
    pub(crate) owner_symbol: Option<GlobalSymbolId>,
    /// Static arguments provided at the call site.
    pub(crate) generic_argument_ids: Option<&'a [LocalNodeId<GenericArgument>]>,
    /// Static arguments supplied by receiver or extension context.
    pub(crate) prefilled_static_arguments: Option<&'a [StaticArgument]>,
    /// Existing substitutions that must be applied before assigning call arguments.
    pub(crate) bound_substitutions: Option<&'a HashMap<GlobalSymbolId, LocalTypeId>>,
    /// Dynamic argument ids used for value-parameter inference.
    pub(crate) argument_ids: Option<&'a [LocalNodeId<Argument>]>,
    /// Static parameter type ids of the signature.
    pub(crate) generic_parameter_type_ids: &'a [LocalTypeId],
    /// Dynamic parameter type ids of the signature.
    pub(crate) parameter_type_ids: &'a [LocalTypeId],
    /// Return type of the signature before substitution.
    pub(crate) return_type: Option<LocalTypeId>,
    /// Expected return type used for reverse static inference.
    pub(crate) expected_return_type: Option<LocalTypeId>,
    /// Resolution mode controlling diagnostics and recovery behavior.
    pub(crate) mode: SignatureResolutionMode,
    /// Whether unresolved value static arguments may defer resolution.
    pub(crate) allow_missing_value_arguments: bool,
}

/// Call-site context for resolving one concrete call signature.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CallSignatureResolutionContext<'a> {
    /// The call expression being resolved.
    pub(crate) expression_id: LocalNodeId<Expression>,
    /// The declaration symbol of the callee when statically known.
    pub(crate) callee_symbol: Option<GlobalSymbolId>,
    /// Static arguments provided at the call site.
    pub(crate) generic_arguments: Option<&'a [LocalNodeId<GenericArgument>]>,
    /// Static arguments inherited from receiver or extension context.
    pub(crate) prefilled_static_arguments: Option<&'a [StaticArgument]>,
    /// Existing substitutions applied before static-argument resolution.
    pub(crate) bound_substitutions: Option<&'a HashMap<GlobalSymbolId, LocalTypeId>>,
    /// Dynamic arguments for argument-guided static inference.
    pub(crate) arguments: Option<&'a [LocalNodeId<Argument>]>,
    /// Receiver type for `this` substitution in member calls.
    pub(crate) call_receiver_ty_id: Option<LocalTypeId>,
    /// Expected return type used for reverse static inference.
    pub(crate) expected_return_type: Option<LocalTypeId>,
    /// Resolution mode controlling diagnostics and recovery behavior.
    pub(crate) mode: SignatureResolutionMode,
    /// Whether unresolved value static arguments may defer resolution.
    pub(crate) allow_missing_value_arguments: bool,
}

/// Call-site context for selecting one overload from multiple signatures.
#[derive(Debug, Clone, Copy)]
struct OverloadSelectionContext<'a> {
    /// The call expression being resolved.
    expression_id: LocalNodeId<Expression>,
    /// The declaration symbol of the callee when statically known.
    callee_symbol: Option<GlobalSymbolId>,
    /// Static arguments provided at the call site.
    generic_arguments: Option<&'a [LocalNodeId<GenericArgument>]>,
    /// Static arguments inherited from receiver or extension context.
    prefilled_static_arguments: Option<&'a [StaticArgument]>,
    /// Existing substitutions applied before static-argument resolution.
    bound_substitutions: Option<&'a HashMap<GlobalSymbolId, LocalTypeId>>,
    /// Dynamic arguments used for overload applicability checks.
    arguments: &'a [LocalNodeId<Argument>],
    /// Receiver type for `this` substitution in member calls.
    call_receiver_ty_id: Option<LocalTypeId>,
    /// Resolution mode controlling diagnostics and recovery behavior.
    mode: SignatureResolutionMode,
}

/// Resolved static-argument state for one signature instantiation.
#[derive(Debug)]
struct ResolvedStaticArgumentState {
    /// The resolved static arguments in declaration order.
    resolved_arguments: Vec<StaticArgument>,
    /// Substitutions produced by static-argument validation.
    substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
    /// Whether a value static argument was missing and replaced by an error placeholder.
    has_missing_value_argument: bool,
    /// Whether value substitutions need materialization normalization.
    has_value_substitution: bool,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect all callable signatures for a type.
    pub(crate) fn call_signatures_for_type(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Vec<LocalTypeId> {
        let mut visited = HashSet::new();
        self.call_signatures_for_type_inner(ty_id, types, &mut visited)
    }

    /// Collect all callable signatures for a type with cycle tracking.
    fn call_signatures_for_type_inner(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Vec<LocalTypeId> {
        if !visited.insert(ty_id) {
            return Vec::new();
        }

        // use direct function types as call signatures
        if matches!(types.get_type(ty_id), Type::Function { .. }) {
            return vec![ty_id];
        }

        // collect call signatures from object types
        if let Type::Object {
            call_signatures, ..
        } = types.get_type(ty_id)
        {
            let mut collected = Vec::new();
            for signature_id in call_signatures {
                collected.extend(self.call_signatures_for_type_inner(
                    *signature_id,
                    types,
                    visited,
                ));
            }
            return collected;
        }

        // collect call signatures from intersections
        if let Type::Intersection { elements } = types.get_type(ty_id) {
            let mut collected = Vec::new();
            for element_id in elements {
                collected.extend(self.call_signatures_for_type_inner(*element_id, types, visited));
            }
            return collected;
        }

        // unwrap value types to their underlying representation
        if let Type::Value { value } = types.get_type(ty_id) {
            return self.call_signatures_for_type_inner(*value, types, visited);
        }

        // follow nominal references into their instance types
        if let Type::Reference { symbol, .. } = types.get_type(ty_id)
            && let Some(instance_id) = types.get_instance_type_id(*symbol)
        {
            return self.call_signatures_for_type_inner(instance_id, types, visited);
        }

        Vec::new()
    }

    /// Collect all construct signatures for a type.
    fn construct_signatures_for_type(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Vec<LocalTypeId> {
        let mut visited = HashSet::new();
        self.construct_signatures_for_type_inner(ty_id, types, &mut visited)
    }

    /// Collect all construct signatures for a type with cycle tracking.
    fn construct_signatures_for_type_inner(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Vec<LocalTypeId> {
        if !visited.insert(ty_id) {
            return Vec::new();
        }

        // use direct function types as construct signatures
        if matches!(types.get_type(ty_id), Type::Function { .. }) {
            return vec![ty_id];
        }

        // collect construct signatures from object types
        if let Type::Object {
            construct_signatures,
            ..
        } = types.get_type(ty_id)
        {
            let mut collected = Vec::new();
            for signature_id in construct_signatures {
                collected.extend(self.construct_signatures_for_type_inner(
                    *signature_id,
                    types,
                    visited,
                ));
            }
            return collected;
        }

        // collect construct signatures from intersections
        if let Type::Intersection { elements } = types.get_type(ty_id) {
            let mut collected = Vec::new();
            for element_id in elements {
                collected.extend(self.construct_signatures_for_type_inner(
                    *element_id,
                    types,
                    visited,
                ));
            }
            return collected;
        }

        // follow nominal references into their instance types
        if let Type::Reference { symbol, .. } = types.get_type(ty_id)
            && let Some(instance_id) = types.get_instance_type_id(*symbol)
        {
            return self.construct_signatures_for_type_inner(instance_id, types, visited);
        }

        Vec::new()
    }

    /// Resolve a function signature and apply `this` substitutions when needed.
    pub(crate) fn resolve_call_signature(
        &self,
        ctx: &mut InferContext<'_>,
        signature_ty_id: LocalTypeId,
        context: CallSignatureResolutionContext<'_>,
    ) -> AnalyzeResult<Option<ResolvedSignature>> {
        let CallSignatureResolutionContext {
            expression_id,
            callee_symbol,
            generic_arguments,
            prefilled_static_arguments,
            bound_substitutions,
            arguments,
            call_receiver_ty_id,
            expected_return_type,
            mode,
            allow_missing_value_arguments,
        } = context;

        let Type::Function {
            generic_parameters,
            parameters,
            return_type,
            ..
        } = ctx.types.get_type(signature_ty_id).clone()
        else {
            return Ok(None);
        };

        let has_explicit_static_arguments =
            generic_arguments.is_some_and(|arguments| !arguments.is_empty());
        if has_explicit_static_arguments
            && generic_parameters.is_empty()
            && let Some(callee_symbol) = callee_symbol
        {
            let parameter_symbols = self
                .collect_static_parameter_symbols(ctx.type_view(), callee_symbol)
                .unwrap_or_default();
            if !parameter_symbols.is_empty() {
                return Err(AnalyzeError::Internal {
                    message: format!(
                        "missing signature static parameters for generic callable {callee_symbol:?}"
                    ),
                });
            }
        }

        let resolved = self
            .resolve_function_static_arguments(
                ctx,
                SignatureStaticResolutionContext {
                    node_id: expression_id.into_any(),
                    owner_symbol: callee_symbol,
                    generic_argument_ids: generic_arguments,
                    prefilled_static_arguments,
                    bound_substitutions,
                    argument_ids: arguments,
                    generic_parameter_type_ids: &generic_parameters,
                    parameter_type_ids: &parameters,
                    return_type,
                    expected_return_type,
                    mode,
                    allow_missing_value_arguments,
                },
            )?
            .unwrap_or(ResolvedSignature {
                parameters,
                return_type,
                generic_arguments: Vec::new(),
            });

        // substitute `this` for member calls
        if let Some(receiver_ty_id) = call_receiver_ty_id {
            let mut cache = HashMap::new();
            let mut mapped_parameters = Vec::with_capacity(resolved.parameters.len());
            for parameter in resolved.parameters.iter() {
                mapped_parameters.push(self.substitute_this_type(
                    *parameter,
                    receiver_ty_id,
                    ctx.types,
                    &mut cache,
                ));
            }
            let mapped_return = resolved.return_type.map(|return_type| {
                self.substitute_this_type(return_type, receiver_ty_id, ctx.types, &mut cache)
            });

            return Ok(Some(ResolvedSignature {
                parameters: mapped_parameters,
                return_type: mapped_return,
                generic_arguments: resolved.generic_arguments,
            }));
        }

        Ok(Some(resolved))
    }

    /// Select the matching call signature overload for a call expression.
    fn select_call_signature(
        &self,
        ctx: &mut InferContext<'_>,
        signature_ids: &[LocalTypeId],
        context: OverloadSelectionContext<'_>,
    ) -> AnalyzeResult<Option<(LocalTypeId, ResolvedSignature)>> {
        let OverloadSelectionContext {
            expression_id,
            callee_symbol,
            generic_arguments,
            prefilled_static_arguments,
            bound_substitutions,
            arguments,
            call_receiver_ty_id,
            mode,
        } = context;

        let _timing = self.timing_scope(tags::ANALYZE_INFER_OVERLOAD_RESOLVE);

        // only attempt selection when overloads exist
        if signature_ids.len() <= 1 {
            return Ok(None);
        }

        // resolve and filter applicable overloads
        let mut candidates = Vec::new();
        for signature_ty_id in signature_ids {
            let Some(resolved) = self.resolve_call_signature(
                &mut ctx.reborrow(),
                *signature_ty_id,
                CallSignatureResolutionContext {
                    expression_id,
                    callee_symbol,
                    generic_arguments,
                    prefilled_static_arguments,
                    bound_substitutions,
                    arguments: Some(arguments),
                    call_receiver_ty_id,
                    expected_return_type: None,
                    mode,
                    allow_missing_value_arguments: true,
                },
            )?
            else {
                continue;
            };

            if !self.is_signature_applicable(&mut ctx.reborrow(), &resolved, arguments)? {
                continue;
            }

            candidates.push((*signature_ty_id, resolved));
        }

        // drop equivalent overloads introduced by declaration merging
        let mut candidates = self.dedupe_signature_candidates(&mut ctx.reborrow(), candidates);

        if candidates.is_empty() {
            return Ok(None);
        }
        if candidates.len() == 1 {
            return Ok(Some(candidates.remove(0)));
        }

        // prefer the first applicable signature in declaration order
        Ok(candidates.into_iter().next())
    }

    /// Drop duplicate overloads that resolve to equivalent shapes.
    pub(crate) fn dedupe_signature_candidates(
        &self,
        ctx: &mut InferContext<'_>,
        candidates: Vec<(LocalTypeId, ResolvedSignature)>,
    ) -> Vec<(LocalTypeId, ResolvedSignature)> {
        let mut deduped = Vec::new();

        for (signature_id, resolved) in candidates {
            let already_seen = deduped.iter().any(|(_, existing)| {
                self.signature_shapes_equivalent(&mut ctx.reborrow(), &resolved, existing)
            });
            if !already_seen {
                deduped.push((signature_id, resolved));
            }
        }

        deduped
    }

    /// Check whether two resolved signatures are equivalent after normalization.
    fn signature_shapes_equivalent(
        &self,
        ctx: &mut InferContext<'_>,
        left: &ResolvedSignature,
        right: &ResolvedSignature,
    ) -> bool {
        // compare parameter counts first
        if left.parameters.len() != right.parameters.len() {
            return false;
        }

        // compare parameter shapes bi-directionally
        for (left_ty_id, right_ty_id) in left.parameters.iter().zip(right.parameters.iter()) {
            let left_assignable = self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                *left_ty_id,
                *right_ty_id,
            );
            let right_assignable = self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                *right_ty_id,
                *left_ty_id,
            );
            if left_assignable == Assignability::NotAssignable
                || right_assignable == Assignability::NotAssignable
            {
                return false;
            }
        }

        // compare return types when present
        match (left.return_type, right.return_type) {
            (None, None) => true,
            (Some(left_return), Some(right_return)) => {
                let left_assignable = self.is_type_assignable(
                    &mut ctx.type_context_reborrow(),
                    left_return,
                    right_return,
                );
                let right_assignable = self.is_type_assignable(
                    &mut ctx.type_context_reborrow(),
                    right_return,
                    left_return,
                );
                left_assignable != Assignability::NotAssignable
                    && right_assignable != Assignability::NotAssignable
            }
            _ => false,
        }
    }

    /// Infer argument types for a resolved signature and emit subtype constraints.
    pub(crate) fn infer_invocation_arguments(
        &self,
        ctx: &mut InferContext<'_>,
        arguments: &[LocalNodeId<Argument>],
        parameter_types: &[LocalTypeId],
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        state: &mut InferState,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // infer arguments left to right, adding constraints eagerly so later contextual typing
        let mut argument_ty_ids = Vec::with_capacity(arguments.len());
        for (index, argument_id) in arguments.iter().enumerate() {
            let parameter_ty_id = parameter_types.get(index).copied();

            // materialize contextual expectation from the current inferred state
            let expected_arg_ty_id = parameter_ty_id.and_then(|parameter_ty_id| {
                self.expected_parameter_type_for_inference(&*ctx, parameter_ty_id)
            });
            self.infer_argument(&mut ctx.reborrow(), *argument_id, expected_arg_ty_id, state)?;

            // resolve inferred argument type
            let argument_value_id = ctx.tree.get(*argument_id).value();
            let argument_ty_id = if let Some(argument_ty_id) = ctx
                .infer
                .inferred_type_for_node(argument_value_id.into_global_any(ctx.module.id))
            {
                let argument_unwrapped_ty_id = self
                    .ensure_unwrapped_value_type_evaluated(&mut ctx.reborrow(), argument_ty_id)?;
                if self.unwrapped_value_type_is_unevaluated(argument_unwrapped_ty_id, &*ctx.types) {
                    self.infer_expression(&mut ctx.reborrow(), argument_value_id, state)?
                } else {
                    argument_ty_id
                }
            } else {
                self.infer_expression(&mut ctx.reborrow(), argument_value_id, state)?
            };
            argument_ty_ids.push(argument_ty_id);

            // add one argument parameter constraint immediately
            let Some(parameter_ty_id) = parameter_ty_id else {
                continue;
            };
            self.add_invocation_argument_constraint(
                &mut ctx.reborrow(),
                *argument_id,
                argument_ty_id,
                parameter_ty_id,
                bound_substitutions,
                state,
            );
        }

        // capture template literal inference constraints
        self.add_template_literal_inference_constraints(
            &mut ctx.reborrow(),
            arguments,
            &argument_ty_ids,
            parameter_types,
        );

        Ok(argument_ty_ids)
    }

    /// Select an expected parameter type for inference without widening type parameters.
    fn expected_parameter_type_for_inference(
        &self,
        ctx: &InferContext<'_>,
        parameter_ty_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        // avoid contextual typing for type parameters
        let expected_ty_id = self.expected_value_type(Some(parameter_ty_id), &*ctx.types)?;
        if let Type::Reference {
            symbol,
            generic_arguments: None,
        } = ctx.types.get_type(expected_ty_id)
            && self.symbol_is_static_parameter(ctx.symbol_type_view(), *symbol)
        {
            return None;
        }
        Some(expected_ty_id)
    }

    /// Add one argument constraint and static parameter bound for an invocation.
    fn add_invocation_argument_constraint(
        &self,
        ctx: &mut InferContext<'_>,
        argument_id: LocalNodeId<Argument>,
        argument_ty_id: LocalTypeId,
        parameter_ty_id: LocalTypeId,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        _state: &InferState,
    ) {
        // connect argument type to parameter type
        ctx.infer.push_constraint(Constraint::Subtype {
            sub_type: argument_ty_id,
            super_type: parameter_ty_id,
            variance: None,
        });

        // enforce static parameter bounds for inferred type parameters
        let Some(parameter_symbol) =
            self.parameter_symbol_for_argument_constraint(ctx.types, ctx.infer, parameter_ty_id)
        else {
            return;
        };

        // skip non-static parameters
        if !self.symbol_is_static_parameter(ctx.symbol_type_view(), parameter_symbol) {
            return;
        }

        // resolve the static parameter constraint
        let argument_value_id = ctx.tree.get(argument_id).value();
        let constraint_id = self.generic_parameter_constraint_type(
            &mut ctx.type_context_reborrow(),
            parameter_symbol,
            argument_value_id.into_any(),
        );
        let Some(constraint_id) = constraint_id else {
            return;
        };

        // apply inherited substitutions to static constraints when needed
        let constraint_id = if let Some(bound_substitutions) = bound_substitutions
            && !bound_substitutions.is_empty()
        {
            let mut cache = HashMap::new();
            self.substitute_static_parameters(
                constraint_id,
                bound_substitutions,
                ctx.types,
                &mut cache,
            )
        } else {
            constraint_id
        };
        if matches!(
            ctx.types.get_type(constraint_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown | TypeLiteral::Any
            }
        ) {
            return;
        }

        // emit a constraint violation error when needed
        if self.is_type_assignable(
            &mut ctx.type_context_reborrow(),
            constraint_id,
            argument_ty_id,
        ) == Assignability::NotAssignable
        {
            self.emit_unassignable_type_for_types(
                ctx.module_type_view(),
                argument_value_id.into_any(),
                constraint_id,
                argument_ty_id,
            );
        }
    }

    /// Resolve the parameter symbol used to constrain an argument type.
    fn parameter_symbol_for_argument_constraint(
        &self,
        types: &TypeTable,
        infer: &InferTable,
        param_ty_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // resolve inferred type parameter symbols first
        if let Type::InferVar { id } = types.get_type(param_ty_id) {
            return infer
                .vars
                .get(id.0 as usize)
                .and_then(|var| match var.origin {
                    InferOrigin::TypeParameter(symbol) => Some(symbol),
                    _ => None,
                });
        }

        // then unwrap reference-like parameter types
        self.unwrap_type_value_symbol(types, param_ty_id)
    }

    /// Validate argument assignability for a resolved signature.
    fn check_invocation_assignability(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        arguments: &[LocalNodeId<Argument>],
        argument_ty_ids: &[LocalTypeId],
        parameter_types: &[LocalTypeId],
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        // check argument assignability against parameters
        for (index, (argument_ty_id, param_ty_id)) in argument_ty_ids
            .iter()
            .zip(parameter_types.iter())
            .enumerate()
        {
            // enforce explicit ownership when implicit managed values are disabled
            if let Some(argument) = arguments.get(index) {
                let argument_value = ctx.tree.get(*argument).value();
                self.check_no_implicit_managed_value(
                    ctx.module_type_view(),
                    argument_value,
                    *param_ty_id,
                    *argument_ty_id,
                    ctx.tree,
                    options,
                );
            }

            let has_unresolved_infer = self.is_infer_var_type(*param_ty_id, ctx.types)
                || self.is_infer_var_type(*argument_ty_id, ctx.types);
            let is_assignable = self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                *param_ty_id,
                *argument_ty_id,
            ) != Assignability::NotAssignable;
            if !has_unresolved_infer && !is_assignable {
                // allow static literal arguments to flow to literal parameter types
                if let Some(argument) = arguments.get(index)
                    && {
                        let argument_value_id = ctx.tree.get(*argument).value();

                        self.static_literal_type_from_argument(
                            &mut ctx.reborrow(),
                            argument_value_id,
                        )?
                    }
                    .is_some_and(|literal_ty_id| {
                        self.is_type_assignable(
                            &mut ctx.type_context_reborrow(),
                            *param_ty_id,
                            literal_ty_id,
                        ) != Assignability::NotAssignable
                    })
                {
                    continue;
                }

                let argument_node = arguments
                    .get(index)
                    .map(|id| id.into_any())
                    .unwrap_or_else(|| expression_id.into_any());
                if let Some(error) = self.unassignable_type_error_for_types(
                    ctx.module_type_view(),
                    argument_node,
                    *param_ty_id,
                    *argument_ty_id,
                ) {
                    return Err(error);
                }
            }
        }

        Ok(())
    }

    /// Check if a resolved signature is applicable to the argument list.
    pub(crate) fn is_signature_applicable(
        &self,
        ctx: &mut InferContext<'_>,
        resolved: &ResolvedSignature,
        arguments: &[LocalNodeId<Argument>],
    ) -> AnalyzeResult<bool> {
        if arguments.len() > resolved.parameters.len() {
            return Ok(false);
        }

        for (index, argument_id) in arguments.iter().enumerate() {
            let Some(param_ty_id) = resolved.parameters.get(index).copied() else {
                return Ok(false);
            };

            if !self.is_signature_argument_applicable(
                &mut ctx.reborrow(),
                *argument_id,
                param_ty_id,
            )? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if one call argument is applicable to a signature parameter.
    fn is_signature_argument_applicable(
        &self,
        ctx: &mut InferContext<'_>,
        argument_id: LocalNodeId<Argument>,
        param_ty_id: LocalTypeId,
    ) -> AnalyzeResult<bool> {
        let argument = ctx.tree.get(argument_id);
        if matches!(argument, Argument::Spread { .. }) {
            return Ok(true);
        }

        let argument_value_id = argument.value();
        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            argument_value_id.into_any(),
            param_ty_id,
        )?;

        if !self.is_signature_lambda_argument_applicable(
            param_ty_id,
            argument_value_id,
            ctx.tree,
            &*ctx.types,
        ) {
            return Ok(false);
        }

        if !self.is_signature_scalar_literal_argument_applicable(
            &mut ctx.reborrow(),
            param_ty_id,
            argument_value_id,
        ) {
            return Ok(false);
        }

        if !self.is_signature_static_literal_argument_applicable(
            &mut ctx.reborrow(),
            param_ty_id,
            argument_value_id,
        )? {
            return Ok(false);
        }

        self.is_signature_known_argument_type_applicable(
            &mut ctx.reborrow(),
            param_ty_id,
            argument_value_id,
        )
    }

    /// Check lambda argument shape compatibility with a function parameter.
    fn is_signature_lambda_argument_applicable(
        &self,
        param_ty_id: LocalTypeId,
        argument_value_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        types: &TypeTable,
    ) -> bool {
        let argument_value = tree.get(argument_value_id);
        if let Expression::Declaration(declaration) = argument_value
            && let Declaration::Function(declaration) = tree.get(*declaration)
            && matches!(declaration.signature.kind, FunctionKind::Lambda)
        {
            let param_signatures = self.call_signatures_for_type(param_ty_id, types);
            let Some(signature_id) = param_signatures.first().copied() else {
                return true;
            };
            let Type::Function {
                parameters,
                return_type,
                ..
            } = types.get_type(signature_id)
            else {
                return true;
            };
            let expected_params = parameters.as_slice();
            let expected_return = *return_type;

            if declaration.signature.parameters.len() > expected_params.len() {
                return false;
            }

            let expects_predicate = expected_return.is_some_and(|return_ty_id| {
                matches!(types.get_type(return_ty_id), Type::Predicate { .. })
            });
            if expects_predicate {
                let has_predicate_return =
                    declaration.signature.return_type.is_some_and(|return_id| {
                        matches!(tree.get(return_id), TypeExpression::Predicate { .. })
                    });
                if !has_predicate_return {
                    return false;
                }
            }
        }

        true
    }

    /// Check scalar literal assignability for signature applicability.
    fn is_signature_scalar_literal_argument_applicable(
        &self,
        ctx: &mut InferContext<'_>,
        param_ty_id: LocalTypeId,
        argument_value_id: LocalNodeId<Expression>,
    ) -> bool {
        let argument_value = ctx.tree.get(argument_value_id);
        let Expression::ScalarLiteral { value } = argument_value else {
            return true;
        };

        let literal_ty = self.infer_scalar_literal(value);
        let ty = Type::TypeLiteral { value: literal_ty };
        let literal_ty_id = ctx
            .types
            .insert_type_from_any(ty, argument_value_id.into_any());

        self.is_signature_candidate_argument_assignable(
            &mut ctx.reborrow(),
            param_ty_id,
            literal_ty_id,
        )
    }

    /// Check static literal assignability for signature applicability.
    fn is_signature_static_literal_argument_applicable(
        &self,
        ctx: &mut InferContext<'_>,
        param_ty_id: LocalTypeId,
        argument_value_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<bool> {
        let Some(literal_ty_id) =
            self.static_literal_type_from_argument(&mut ctx.reborrow(), argument_value_id)?
        else {
            return Ok(true);
        };

        Ok(self.is_signature_candidate_argument_assignable(
            &mut ctx.reborrow(),
            param_ty_id,
            literal_ty_id,
        ))
    }

    /// Check known argument-type assignability for signature applicability.
    fn is_signature_known_argument_type_applicable(
        &self,
        ctx: &mut InferContext<'_>,
        param_ty_id: LocalTypeId,
        argument_value_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<bool> {
        let argument_value = ctx.tree.get(argument_value_id);
        let argument_ty_id = ctx
            .infer
            .inferred_type_for_node(argument_value_id.into_global_any(ctx.module.id))
            .or_else(|| {
                argument_value
                    .target_symbol()
                    .and_then(|symbol| ctx.types.get_type_id_for_symbol(ctx.symbols, symbol))
            });
        let Some(argument_ty_id) = argument_ty_id else {
            return Ok(true);
        };
        let argument_unwrapped_ty_id =
            self.ensure_unwrapped_value_type_evaluated(&mut ctx.reborrow(), argument_ty_id)?;
        if self.unwrapped_value_type_is_unevaluated(argument_unwrapped_ty_id, &*ctx.types) {
            return Ok(true);
        }

        Ok(self.is_signature_candidate_argument_assignable(
            &mut ctx.reborrow(),
            param_ty_id,
            argument_ty_id,
        ))
    }

    /// Resolve a scalar literal type for static argument expressions when possible.
    fn static_literal_type_from_argument(
        &self,
        ctx: &mut InferContext<'_>,
        argument_value_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // evaluate to a scalar literal when possible
        let value = self.query_static_expression_value(
            &mut ctx.type_context_reborrow(),
            argument_value_id,
            None,
        )?;
        let literal_value = match value {
            Some(StaticExpression::ScalarLiteral { value }) => value,
            Some(StaticExpression::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value),
            }) => value,
            Some(StaticExpression::Type { ty }) => match ctx.types.get_type(ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(value),
                } => value.clone(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal_value),
        };
        Ok(Some(
            ctx.types
                .insert_type_from_any(ty, argument_value_id.into_any()),
        ))
    }

    /// Prepare member-call typing context shared by member call resolution paths.
    fn resolve_member_call_type_context(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty_id: Option<LocalTypeId>,
        receiver_ty: &Type,
        member_key: &StaticKey,
        member_symbol: Option<GlobalSymbolId>,
        lookup_mode: MemberLookupMode,
    ) -> AnalyzeResult<ResolvedMemberCallTypeContext> {
        // inherit static arguments and substitutions from the receiver
        let mut inherited = self.resolve_inherited_static_arguments(
            &mut ctx.reborrow(),
            receiver_expression_id.into_any(),
            receiver_ty_id,
            receiver_ty,
        )?;
        if let Some(member_symbol) = member_symbol {
            self.extend_owner_substitutions_from_inherited(
                &mut ctx.type_context_reborrow(),
                receiver_expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
                &mut inherited.substitutions,
            );
        }

        // resolve extension substitutions for member symbols
        let extension_context = if let Some(member_symbol) = member_symbol {
            self.resolve_extension_member_context(
                &mut ctx.type_context_reborrow(),
                receiver_expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
            )?
        } else {
            None
        };

        // merge inherited and extension substitutions
        let substitutions = self.merge_member_substitutions(&inherited, extension_context.as_ref());

        // select instance arguments for member instancing
        let instance_arguments = if let Some(context) = extension_context.as_ref() {
            context.arguments.clone()
        } else {
            inherited.arguments.clone()
        };

        // infer member type for this receiver
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            &mut ctx.type_context_reborrow(),
            receiver_expression_id.into_any(),
            receiver_ty,
            member_key,
            lookup_mode,
            &mut member_type_visited,
        )?;
        let member_ty_id = self.resolve_member_type_for_symbol(
            &mut ctx.reborrow(),
            receiver_expression_id,
            member_symbol,
            member_ty_id,
        )?;

        // apply static substitutions before call-signature resolution
        let member_ty_id = member_ty_id.map(|member_ty_id| {
            if substitutions.is_empty() {
                member_ty_id
            } else {
                let mut cache = HashMap::new();
                self.substitute_static_parameters(
                    member_ty_id,
                    &substitutions,
                    ctx.types,
                    &mut cache,
                )
            }
        });

        Ok(ResolvedMemberCallTypeContext {
            member_ty_id,
            substitutions,
            instance_arguments,
            extension_arguments: extension_context.map(|context| context.arguments),
        })
    }

    /// Resolve per variant member call candidates for a union receiver.
    fn resolve_union_member_call_candidates(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_union_ty_id: LocalTypeId,
        element_ids: &[LocalTypeId],
        member_key: &StaticKey,
        generic_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        arguments: &[LocalNodeId<Argument>],
    ) -> AnalyzeResult<Option<Vec<UnionMemberCallCandidate>>> {
        // query receiver context used by per element lookup filtering
        let receiver_context = self.query_member_receiver_context_for_expression(
            &ctx.type_context_reborrow(),
            receiver_expression_id,
            Some(receiver_union_ty_id),
        );
        let context = UnionMemberCallResolutionContext {
            tree_symbols: ctx.tree_symbol_view(),
            index: ctx.index.clone(),
            expression_id,
            receiver_expression_id,
            receiver_union_ty_id,
            receiver_nominal_symbol: receiver_context.nominal_symbol,
            member_key,
            generic_arguments,
            arguments,
        };

        // resolve one candidate per union element
        let mut candidates = Vec::new();
        for element_id in element_ids {
            let Some(candidate) = self.resolve_union_member_call_candidate_for_element(
                &context,
                &mut ctx.reborrow(),
                *element_id,
            )?
            else {
                return Ok(None);
            };

            candidates.push(candidate);
        }

        Ok(Some(candidates))
    }

    /// Resolve one union member-call candidate for a specific union element.
    fn resolve_union_member_call_candidate_for_element(
        &self,
        context: &UnionMemberCallResolutionContext<'_>,
        ctx: &mut InferContext<'_>,
        element_id: LocalTypeId,
    ) -> AnalyzeResult<Option<UnionMemberCallCandidate>> {
        // resolve the element type and target member symbol
        let element_ty = ctx.types.get_type(element_id).clone();
        let member_symbol = self.resolve_union_member_symbol_for_element(
            context,
            element_id,
            &element_ty,
            ctx.types,
        )?;
        let Some(member_symbol) = member_symbol else {
            self.report_union_member_call_missing_member(context, &mut ctx.reborrow())?;
            return Ok(None);
        };

        // prepare member call typing state for this element
        let lookup_mode = if context.receiver_nominal_symbol.is_some() {
            MemberLookupMode::Value
        } else {
            MemberLookupMode::Instance
        };
        let resolved_context = self.resolve_member_call_type_context(
            &mut ctx.reborrow(),
            context.receiver_expression_id,
            Some(element_id),
            &element_ty,
            context.member_key,
            Some(member_symbol),
            lookup_mode,
        )?;
        let Some(member_ty_id) = resolved_context.member_ty_id else {
            self.report_union_member_call_missing_member(context, &mut ctx.reborrow())?;
            return Ok(None);
        };

        // resolve one callable signature for this element member
        let resolved = self.resolve_union_member_call_candidate_signature(
            context,
            &mut ctx.reborrow(),
            element_id,
            member_symbol,
            member_ty_id,
            &resolved_context,
        )?;
        let Some(resolved) = resolved else {
            return Ok(None);
        };
        let (signature_ty_id, resolved_signature) = resolved;

        let signature_parameter_symbols =
            self.query_signature_static_parameter_symbols(signature_ty_id, ctx.types);

        // compose canonical member substitution environment from receiver and signature substitutions
        let instance_environment = self.compose_member_instance_environment(
            ctx,
            member_symbol,
            &resolved_context.instance_arguments,
            &resolved_context.substitutions,
            &resolved_signature.generic_arguments,
            &signature_parameter_symbols,
        );

        Ok(Some(UnionMemberCallCandidate {
            receiver_ty_id: element_id,
            symbol: member_symbol,
            signature: resolved_signature,
            instance_environment,
        }))
    }

    /// Resolve the member symbol for one union element.
    fn resolve_union_member_symbol_for_element(
        &self,
        context: &UnionMemberCallResolutionContext<'_>,
        element_id: LocalTypeId,
        element_ty: &Type,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // first try direct member lookup on the element type
        let mut visited = Vec::new();
        let mut member_symbol = self.resolve_member_symbol_for_type(
            context.tree_symbols.compiler_context,
            context.tree_symbols.module,
            context.tree_symbols.module.id,
            context.tree_symbols.profile,
            &context.index,
            context.tree_symbols.tree,
            context.tree_symbols.symbols,
            types,
            element_ty,
            context.member_key,
            &mut visited,
            true,
        )?;
        if member_symbol.is_some() {
            return Ok(member_symbol);
        }

        // then fall back to instance owner lookup when available
        if let Some(instance_symbol) = types.symbol_for_instance_type(element_id) {
            member_symbol = self.resolve_member_symbol_for_symbol(
                context.tree_symbols.compiler_context,
                context.tree_symbols.module,
                context.tree_symbols.module.id,
                context.tree_symbols.profile,
                &context.index,
                context.tree_symbols.tree,
                context.tree_symbols.symbols,
                types,
                instance_symbol,
                context.member_key,
                MemberLookupMode::Instance,
                &mut visited,
            )?;
        }

        Ok(member_symbol)
    }

    /// Resolve one callable signature for a union element member candidate.
    fn resolve_union_member_call_candidate_signature(
        &self,
        context: &UnionMemberCallResolutionContext<'_>,
        ctx: &mut InferContext<'_>,
        element_id: LocalTypeId,
        member_symbol: GlobalSymbolId,
        member_ty_id: LocalTypeId,
        resolved_context: &ResolvedMemberCallTypeContext,
    ) -> AnalyzeResult<Option<(LocalTypeId, ResolvedSignature)>> {
        // query callable signatures for the resolved member type
        let call_signatures = self.call_signatures_for_type(member_ty_id, &*ctx.types);
        if call_signatures.is_empty() {
            self.emit_non_callable_for_callee_type(
                ctx.module_type_view(),
                context.expression_id.into_any(),
                context.receiver_union_ty_id,
            );
            return Ok(None);
        }

        // select the best overload when multiple signatures exist
        let bound_substitutions =
            (!resolved_context.substitutions.is_empty()).then_some(&resolved_context.substitutions);
        if call_signatures.len() > 1 {
            let selection = self.select_call_signature(
                &mut ctx.reborrow(),
                &call_signatures,
                OverloadSelectionContext {
                    expression_id: context.expression_id,
                    callee_symbol: Some(member_symbol),
                    generic_arguments: context.generic_arguments,
                    prefilled_static_arguments: resolved_context.extension_arguments.as_deref(),
                    bound_substitutions,
                    arguments: context.arguments,
                    call_receiver_ty_id: Some(element_id),
                    mode: SignatureResolutionMode::Synthesize,
                },
            )?;
            let Some((signature_ty_id, resolved)) = selection else {
                self.emit_no_overload_for_receiver_type(
                    ctx.module_type_view(),
                    context.expression_id.into_any(),
                    context.receiver_union_ty_id,
                );
                return Ok(None);
            };

            return Ok(Some((signature_ty_id, resolved)));
        }

        // resolve the singleton signature directly
        let signature_ty_id = call_signatures[0];
        let resolved = self.resolve_call_signature(
            &mut ctx.reborrow(),
            signature_ty_id,
            CallSignatureResolutionContext {
                expression_id: context.expression_id,
                callee_symbol: Some(member_symbol),
                generic_arguments: context.generic_arguments,
                prefilled_static_arguments: resolved_context.extension_arguments.as_deref(),
                bound_substitutions,
                arguments: Some(context.arguments),
                call_receiver_ty_id: Some(element_id),
                expected_return_type: None,
                mode: SignatureResolutionMode::Synthesize,
                allow_missing_value_arguments: false,
            },
        )?;
        let Some(resolved) = resolved else {
            self.emit_non_callable_for_callee_type(
                ctx.module_type_view(),
                context.expression_id.into_any(),
                context.receiver_union_ty_id,
            );
            return Ok(None);
        };

        Ok(Some((signature_ty_id, resolved)))
    }

    /// Report a missing-member diagnostic for union member-call resolution.
    fn report_union_member_call_missing_member(
        &self,
        context: &UnionMemberCallResolutionContext<'_>,
        ctx: &mut InferContext<'_>,
    ) -> AnalyzeResult<()> {
        // allow associated blockers only for projection receivers
        let allow_associated_contract_blocker = self
            .is_projection_receiver_expression(&mut ctx.reborrow(), context.receiver_expression_id);

        self.report_missing_member_diagnostic(
            context.tree_symbols.type_view(ctx.types),
            context.expression_id.into_any(),
            context.receiver_union_ty_id,
            *context.member_key,
            allow_associated_contract_blocker,
        )?;

        Ok(())
    }

    /// Infer a call expression.
    pub(crate) fn infer_call_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        generic_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        arguments: &[LocalNodeId<Argument>],
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_CALL);

        // enforce call restrictions from options
        self.validate_call_expression(ctx, expression_id, left_id, false);

        // query and normalize the callee state
        let callee = match self.infer_call_expression_callee(
            &mut ctx.reborrow(),
            expression_id,
            left_id,
            state,
        )? {
            CallExpressionCalleeQuery::EarlyType(type_id) => return Ok(type_id),
            CallExpressionCalleeQuery::Callee(callee) => callee,
        };
        let finish_result = |type_id: LocalTypeId, types: &mut TypeTable| {
            self.optional_chain_result_type(
                expression_id,
                type_id,
                callee.has_optional_nullish,
                types,
            )
        };

        // malformed argument slots poison the whole invocation
        if self.arguments_have_error_slots(ctx.tree, arguments) {
            self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;

            let type_id = self.synthesize_call_error_result_type(expression_id, &mut *ctx.types);
            return Ok(finish_result(type_id, &mut *ctx.types));
        }

        // ensure instance types for callable references
        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            callee.callee_ty_id,
        )?;

        // resolve symbols, inherited substitutions, and member-call context
        let call = self.resolve_call_expression_target(
            &mut ctx.reborrow(),
            expression_id,
            left_id,
            callee.callee_ty_id,
            generic_arguments,
            state,
        )?;

        // resolve effective static-argument sources
        let effective_static_arguments =
            self.query_call_signature_static_arguments(generic_arguments, &call);
        let union_static_arguments =
            self.query_union_member_call_static_arguments(generic_arguments, &call);

        // handle union receiver member calls
        let union_return_type_id = self.infer_union_member_call_expression(
            &mut ctx.reborrow(),
            expression_id,
            call.member_call_context.as_ref(),
            union_static_arguments,
            arguments,
            state,
        )?;
        if let Some(union_return_type_id) = union_return_type_id {
            return Ok(finish_result(union_return_type_id, &mut *ctx.types));
        }

        // query call signatures from the normalized call target
        let call_signatures = if self.expression_is_super_reference_for_call(
            ctx.tree,
            self.unwrap_parenthesized_expression(left_id, ctx.tree),
        ) {
            if let Some(super_constructor_value_ty_id) = call.super_constructor_value_ty_id {
                self.construct_signatures_for_type(super_constructor_value_ty_id, &*ctx.types)
            } else {
                Vec::new()
            }
        } else {
            self.call_signatures_for_type(callee.callee_ty_id, &*ctx.types)
        };
        let ty_id = if call_signatures.is_empty() {
            self.infer_non_callable_call_expression(
                &mut ctx.reborrow(),
                expression_id,
                left_id,
                callee.callee_ty_id,
                &call,
                arguments,
                state,
            )?
        } else {
            // resolve one concrete signature for this call
            let bound_substitutions =
                (!call.inherited_substitutions.is_empty()).then_some(&call.inherited_substitutions);
            let selection = self.select_call_signature(
                &mut ctx.reborrow(),
                &call_signatures,
                OverloadSelectionContext {
                    expression_id,
                    callee_symbol: call.callee_symbol,
                    generic_arguments: effective_static_arguments,
                    prefilled_static_arguments: call.prefilled_static_arguments.as_deref(),
                    bound_substitutions,
                    arguments,
                    call_receiver_ty_id: call.call_receiver_ty_id,
                    mode: SignatureResolutionMode::Synthesize,
                },
            )?;
            if let Some((signature_ty_id, resolved_signature)) = selection {
                self.infer_resolved_call_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    arguments,
                    &call,
                    signature_ty_id,
                    resolved_signature,
                    state,
                )?
            } else if call_signatures.len() > 1 {
                self.emit_no_overload_for_receiver_type(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    callee.callee_ty_id,
                );
                self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;
                self.synthesize_call_error_result_type(expression_id, &mut *ctx.types)
            } else {
                let signature_ty_id = call_signatures[0];
                let resolved = self.resolve_call_signature(
                    &mut ctx.reborrow(),
                    signature_ty_id,
                    CallSignatureResolutionContext {
                        expression_id,
                        callee_symbol: call.callee_symbol,
                        generic_arguments: effective_static_arguments,
                        prefilled_static_arguments: call.prefilled_static_arguments.as_deref(),
                        bound_substitutions,
                        arguments: Some(arguments),
                        call_receiver_ty_id: call.call_receiver_ty_id,
                        expected_return_type: state.expected_type,
                        mode: SignatureResolutionMode::Synthesize,
                        allow_missing_value_arguments: false,
                    },
                )?;
                if let Some(resolved_signature) = resolved {
                    self.infer_resolved_call_expression(
                        &mut ctx.reborrow(),
                        expression_id,
                        arguments,
                        &call,
                        signature_ty_id,
                        resolved_signature,
                        state,
                    )?
                } else {
                    self.synthesize_call_error_result_type(expression_id, &mut *ctx.types)
                }
            }
        };

        Ok(finish_result(ty_id, &mut *ctx.types))
    }

    /// Infer argument constraints, record call resolutions, and return the call result type.
    fn infer_resolved_call_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        arguments: &[LocalNodeId<Argument>],
        call: &CallExpressionResolution,
        signature_ty_id: LocalTypeId,
        resolved_signature: ResolvedSignature,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // apply inherited substitutions from the receiver context
        let resolved_signature = self.apply_inherited_call_substitutions(
            &mut ctx.reborrow(),
            resolved_signature,
            &call.inherited_substitutions,
        );
        let resolved_parameters = &resolved_signature.parameters;

        // enforce strict call arity for synthetic call wrappers
        if self.should_enforce_strict_call_member_arity(ctx, call)
            && let Some(minimum_arguments) =
                self.minimum_required_argument_count_for_strict_call_member(ctx, call)
            && arguments.len() < minimum_arguments
        {
            self.error(AnalyzeError::InvalidArgumentArity {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
                expected: minimum_arguments,
                actual: arguments.len(),
            });
            self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;

            return Ok(self.synthesize_call_error_result_type(expression_id, &mut *ctx.types));
        }

        // enforce static-expression requirements for comptime dynamic parameters
        if !self.comptime_arguments_are_static(
            &mut ctx.reborrow(),
            signature_ty_id,
            arguments,
            call.callee_symbol,
        )? {
            self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;

            return Ok(self.synthesize_call_error_result_type(expression_id, &mut *ctx.types));
        }

        // infer argument types and constraints
        let argument_ty_ids = self.infer_invocation_arguments(
            &mut ctx.reborrow(),
            arguments,
            resolved_parameters,
            (!call.inherited_substitutions.is_empty()).then_some(&call.inherited_substitutions),
            state,
        )?;

        // check argument assignability against parameters
        self.check_invocation_assignability(
            &mut ctx.reborrow(),
            expression_id,
            arguments,
            &argument_ty_ids,
            resolved_parameters,
            &state.options,
        )?;

        let resolved_return_type = resolved_signature.return_type;

        // commit static or member resolution for downstream lowering
        self.record_call_expression_resolution(
            &mut ctx.reborrow(),
            expression_id,
            call,
            signature_ty_id,
            resolved_signature,
        )?;

        Ok(resolved_return_type.unwrap_or_else(|| {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            ctx.types.insert_type_from(ty, expression_id)
        }))
    }

    /// Return true when strict call-member arity should be enforced for this call.
    fn should_enforce_strict_call_member_arity(
        &self,
        ctx: &InferContext<'_>,
        call: &CallExpressionResolution,
    ) -> bool {
        if !ctx.options.strict_bind_call_apply {
            return false;
        }

        let Some(member_context) = call.member_call_context.as_ref() else {
            return false;
        };
        let call_name = self.repository.strings.intern("call");
        if !matches!(member_context.member_key, StaticKey::Name(name) if name == call_name) {
            return false;
        }
        let Some(receiver_ty_id) = call.call_receiver_ty_id else {
            return false;
        };
        if self.receiver_declares_member_name(ctx.type_view(), receiver_ty_id, call_name) {
            return false;
        }

        !self
            .call_signatures_for_type(receiver_ty_id, &*ctx.types)
            .is_empty()
    }

    /// Return the minimum required argument count for one resolved signature.
    fn minimum_required_argument_count(
        &self,
        ctx: TypeView<'_>,
        signature_ty_id: LocalTypeId,
    ) -> Option<usize> {
        let parameters = self.parameters_for_signature_source(ctx, signature_ty_id, None)?;

        Some(self.minimum_required_argument_count_for_parameters(ctx.tree, parameters))
    }

    /// Return strict `call` minimum arity from receiver signatures, plus `thisArg`.
    fn minimum_required_argument_count_for_strict_call_member(
        &self,
        ctx: &InferContext<'_>,
        call: &CallExpressionResolution,
    ) -> Option<usize> {
        let receiver_ty_id = call.call_receiver_ty_id?;
        let receiver_signatures = self.call_signatures_for_type(receiver_ty_id, ctx.types);

        let mut minimum_required_receiver_arguments: Option<usize> = None;
        for signature_ty_id in receiver_signatures {
            let Some(required_receiver_arguments) =
                self.minimum_required_argument_count(ctx.type_view(), signature_ty_id)
            else {
                continue;
            };

            minimum_required_receiver_arguments = Some(match minimum_required_receiver_arguments {
                Some(existing) => existing.min(required_receiver_arguments),
                None => required_receiver_arguments,
            });
        }

        minimum_required_receiver_arguments.map(|minimum| minimum + 1)
    }

    /// Return the minimum required argument count for one parameter list.
    fn minimum_required_argument_count_for_parameters(
        &self,
        tree: &NodeTree,
        parameters: &[LocalNodeId<Parameter>],
    ) -> usize {
        let mut minimum = 0;
        for parameter_id in parameters {
            let parameter = tree.get(*parameter_id);
            let is_optional = match parameter {
                Parameter::Named {
                    is_optional,
                    default,
                    ..
                }
                | Parameter::Pattern {
                    is_optional,
                    default,
                    ..
                } => *is_optional || default.is_some(),
                Parameter::VariadicNamed { .. }
                | Parameter::VariadicPattern { .. }
                | Parameter::Error { .. } => true,
            };
            if !is_optional {
                minimum += 1;
            }
        }

        minimum
    }

    /// Validate comptime arguments are static expressions for this signature.
    fn comptime_arguments_are_static(
        &self,
        ctx: &mut InferContext<'_>,
        signature_ty_id: LocalTypeId,
        arguments: &[LocalNodeId<Argument>],
        expected_owner_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<bool> {
        let comptime_indexes = self.comptime_parameter_indexes_for_signature(
            ctx.type_view(),
            signature_ty_id,
            expected_owner_symbol,
        );
        if comptime_indexes.is_empty() {
            return Ok(true);
        }

        let mut all_static = true;
        for index in comptime_indexes {
            let Some(argument_id) = arguments.get(index).copied() else {
                continue;
            };
            let argument_expression_id = ctx.tree.get(argument_id).value();
            let value = self.query_static_expression_value(
                &mut ctx.type_context_reborrow(),
                argument_expression_id,
                None,
            )?;
            let is_static = value
                .as_ref()
                .is_some_and(|value| self.static_value_argument_is_static(value, ctx.type_view()));
            if is_static {
                continue;
            }

            self.error(AnalyzeError::NonStaticArgument {
                node: argument_expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            all_static = false;
        }

        Ok(all_static)
    }

    /// Return parameter indexes marked as comptime on one signature source.
    fn comptime_parameter_indexes_for_signature(
        &self,
        ctx: TypeView<'_>,
        signature_ty_id: LocalTypeId,
        expected_owner_symbol: Option<GlobalSymbolId>,
    ) -> Vec<usize> {
        let _ctx = ctx;
        let _signature_ty_id = signature_ty_id;
        let _expected_owner_symbol = expected_owner_symbol;

        Vec::new()
    }

    /// Return parameter ids for one signature source when syntax metadata is available.
    fn parameters_for_signature_source<'a>(
        &self,
        ctx: TypeView<'a>,
        signature_ty_id: LocalTypeId,
        expected_owner_symbol: Option<GlobalSymbolId>,
    ) -> Option<&'a [LocalNodeId<Parameter>]> {
        let signature_source = ctx.types.get_type_source(signature_ty_id);
        let (parameters, source_owner_symbol) = match signature_source.ty {
            NodeType::Declaration => {
                let declaration_id = signature_source.into_typed::<Declaration>();
                if !ctx.tree.has_node_id(declaration_id.id) {
                    return None;
                }
                let declaration = ctx.tree.get(declaration_id);
                match declaration {
                    Declaration::Function(declaration) => (
                        declaration.signature.parameters.as_slice(),
                        Some(declaration.symbol.into_global(ctx.module.id)),
                    ),
                    _ => return None,
                }
            }
            NodeType::Member => {
                let member_id = signature_source.into_typed::<Member>();
                if !ctx.tree.has_node_id(member_id.id) {
                    return None;
                }
                let member = ctx.tree.get(member_id);
                match member {
                    Member::Method { signature, .. } => (
                        signature.parameters.as_slice(),
                        Some(member.symbol().into_global(ctx.module.id)),
                    ),
                    _ => return None,
                }
            }
            NodeType::Property => {
                let property_id = signature_source.into_typed::<Property>();
                if !ctx.tree.has_node_id(property_id.id) {
                    return None;
                }
                let property = ctx.tree.get(property_id);
                match property {
                    Property::Method {
                        symbol, signature, ..
                    } => (
                        signature.parameters.as_slice(),
                        Some(symbol.into_global(ctx.module.id)),
                    ),
                    _ => return None,
                }
            }
            _ => return None,
        };
        if !self.signature_source_owner_matches_expected(
            ctx,
            source_owner_symbol,
            expected_owner_symbol,
        ) {
            return None;
        }
        if !self.signature_source_parameters_match_signature(ctx, signature_ty_id, parameters) {
            return None;
        }

        Some(parameters)
    }

    /// Return true when one signature source owner matches the expected callee symbol.
    fn signature_source_owner_matches_expected(
        &self,
        ctx: TypeView<'_>,
        source_owner_symbol: Option<GlobalSymbolId>,
        expected_owner_symbol: Option<GlobalSymbolId>,
    ) -> bool {
        let Some(expected_owner_symbol) = expected_owner_symbol else {
            return true;
        };
        let Some(source_owner_symbol) = source_owner_symbol else {
            return false;
        };

        let normalize = |symbol| {
            let symbol = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            self.declaration_symbol_id(ctx.module_symbol_view(), symbol)
                .unwrap_or(symbol)
        };

        normalize(source_owner_symbol) == normalize(expected_owner_symbol)
    }

    /// Return true when source parameters still match the function type signature.
    fn signature_source_parameters_match_signature(
        &self,
        ctx: TypeView<'_>,
        signature_ty_id: LocalTypeId,
        parameters: &[LocalNodeId<Parameter>],
    ) -> bool {
        let Type::Function {
            parameters: signature_parameters,
            ..
        } = ctx.types.get_type(signature_ty_id)
        else {
            return false;
        };
        if signature_parameters.len() != parameters.len() {
            return false;
        }

        for (index, parameter_id) in parameters.iter().enumerate() {
            if !ctx.tree.has_node_id(parameter_id.id) {
                return false;
            }

            let parameter = ctx.tree.get(*parameter_id);
            let parameter_symbol = parameter.symbol().into_global(ctx.module.id);
            let parameter_symbol_entry = ctx.symbols.get_symbol(parameter_symbol.local_id);
            if parameter_symbol_entry.primary_declaration
                != Some(parameter_id.into_global_any(ctx.module.id))
            {
                return false;
            }

            let parameter_type_id = ctx
                .types
                .get_declared_or_inferred_type_id(parameter_id.into_global_any(ctx.module.id));
            let Some(parameter_type_id) = parameter_type_id else {
                return false;
            };

            let source_parameter_type_id = self.unwrap_type_value(parameter_type_id, ctx.types);
            let signature_parameter_type_id =
                self.unwrap_type_value(signature_parameters[index], ctx.types);
            if source_parameter_type_id != signature_parameter_type_id {
                return false;
            }
        }

        true
    }

    /// Return true when one receiver type declares a member field with the given name.
    fn receiver_declares_member_name(
        &self,
        ctx: TypeView<'_>,
        receiver_ty_id: LocalTypeId,
        member_name: StringId,
    ) -> bool {
        let mut visited = HashSet::new();
        self.receiver_declares_member_name_inner(ctx, receiver_ty_id, member_name, &mut visited)
    }

    /// Return true when one receiver type declares a member field with the given name.
    fn receiver_declares_member_name_inner(
        &self,
        ctx: TypeView<'_>,
        receiver_ty_id: LocalTypeId,
        member_name: StringId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        if !visited.insert(receiver_ty_id) {
            return false;
        }

        match ctx.types.get_type(receiver_ty_id) {
            Type::Object { fields, .. } => fields
                .iter()
                .any(|field| matches!(field.key, StaticKey::Name(name) if name == member_name)),
            Type::Union { elements } | Type::Intersection { elements } => {
                elements.iter().any(|element_id| {
                    self.receiver_declares_member_name_inner(ctx, *element_id, member_name, visited)
                })
            }
            Type::Value { value }
            | Type::Readonly { target_type: value }
            | Type::KeyOf { target_type: value }
            | Type::Must { target_type: value }
            | Type::AsComptime { target_type: value }
            | Type::Not { target_type: value }
            | Type::ValueOf { right: value, .. }
            | Type::ReferenceOf { right: value, .. }
            | Type::PointerOf { right: value, .. } => {
                self.receiver_declares_member_name_inner(ctx, *value, member_name, visited)
            }
            _ => false,
        }
    }

    /// Apply inherited substitutions to a resolved signature.
    fn apply_inherited_call_substitutions(
        &self,
        ctx: &mut InferContext<'_>,
        resolved_signature: ResolvedSignature,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> ResolvedSignature {
        if substitutions.is_empty() {
            return resolved_signature;
        }

        let mut materialize_cache = HashMap::new();
        let mut substitute_cache = HashMap::new();
        let parameters = resolved_signature
            .parameters
            .iter()
            .map(|parameter| {
                let materialized = self.materialize_static_arguments_in_type(
                    &mut ctx.type_context_reborrow(),
                    *parameter,
                    &mut materialize_cache,
                );
                self.substitute_static_parameters(
                    materialized,
                    substitutions,
                    &mut *ctx.types,
                    &mut substitute_cache,
                )
            })
            .collect();
        let return_type = resolved_signature.return_type.map(|return_type| {
            let materialized = self.materialize_static_arguments_in_type(
                &mut ctx.type_context_reborrow(),
                return_type,
                &mut materialize_cache,
            );
            self.substitute_static_parameters(
                materialized,
                substitutions,
                &mut *ctx.types,
                &mut substitute_cache,
            )
        });

        ResolvedSignature {
            parameters,
            return_type,
            generic_arguments: resolved_signature.generic_arguments,
        }
    }

    /// Record call-resolution entries after successful signature inference.
    fn record_call_expression_resolution(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        call: &CallExpressionResolution,
        signature_ty_id: LocalTypeId,
        resolved_signature: ResolvedSignature,
    ) -> AnalyzeResult<()> {
        // register the instance when the call resolves to a symbol
        let call_instance_id = if let Some(callee_symbol) = call.callee_symbol {
            if call.call_member_resolution.is_none() {
                let signature_parameter_symbols =
                    self.query_signature_static_parameter_symbols(signature_ty_id, &*ctx.types);
                let environment = StaticSubstitutionEnvironment::from_parameter_symbols(
                    resolved_signature.generic_arguments.clone(),
                    signature_parameter_symbols,
                    0,
                )
                .or_else(|| {
                    self.instance_environment_for_symbol_arguments(
                        &ctx.reborrow(),
                        callee_symbol,
                        resolved_signature.generic_arguments.clone(),
                        0,
                    )
                });
                if let Some(environment) = environment {
                    self.record_node_provisional_instance(
                        expression_id.into_global_any(ctx.module.id),
                        callee_symbol,
                        environment,
                        &mut *ctx.infer,
                        &mut *ctx.types,
                    )?
                } else {
                    None
                }
            } else {
                let signature_parameter_symbols =
                    self.query_signature_static_parameter_symbols(signature_ty_id, &*ctx.types);
                let base_instance_arguments = call
                    .member_instance_arguments
                    .as_deref()
                    .or(call.prefilled_static_arguments.as_deref())
                    .unwrap_or(call.inherited_static_arguments.as_slice());
                let environment = self.compose_member_instance_environment(
                    &ctx.reborrow(),
                    callee_symbol,
                    base_instance_arguments,
                    &call.inherited_substitutions,
                    &resolved_signature.generic_arguments,
                    &signature_parameter_symbols,
                );
                if let Some(environment) = environment {
                    self.record_node_provisional_instance(
                        expression_id.into_global_any(ctx.module.id),
                        callee_symbol,
                        environment,
                        &mut *ctx.infer,
                        &mut *ctx.types,
                    )?
                } else {
                    None
                }
            }
        } else {
            None
        };

        // commit member or static resolution metadata
        match (&call.call_member_resolution, call.callee_symbol) {
            (Some(member_resolution), _) => {
                self.record_provisional_member_resolution(
                    expression_id.into_global_any(ctx.module.id),
                    call.call_receiver_ty_id,
                    member_resolution,
                    call_instance_id,
                    Some(resolved_signature),
                    true,
                    &mut *ctx.infer,
                    &mut *ctx.types,
                );
            }
            (None, Some(callee_symbol)) => {
                self.record_provisional_static_resolution(
                    expression_id.into_global_any(ctx.module.id),
                    call.call_receiver_ty_id,
                    callee_symbol,
                    call_instance_id,
                    Some(resolved_signature),
                    &mut *ctx.infer,
                    &mut *ctx.types,
                );
            }
            _ => {}
        }

        Ok(())
    }

    /// Infer a non-callable call expression and recover with an error type.
    fn infer_non_callable_call_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        call_target_id: LocalNodeId<Expression>,
        callee_ty_id: LocalTypeId,
        call: &CallExpressionResolution,
        arguments: &[LocalNodeId<Argument>],
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // suppress secondary diagnostics when member lookup already failed
        let has_missing_member = matches!(
            call.call_member_resolution,
            Some(MemberResolution::None | MemberResolution::Unresolved)
        );
        let is_super_call = self.expression_is_super_reference_for_call(
            ctx.tree,
            self.unwrap_parenthesized_expression(call_target_id, ctx.tree),
        );
        let callee_has_primary_error =
            self.type_blocks_cascading_diagnostic(callee_ty_id, &*ctx.types);

        // report non-callable callee types unless they are dynamic placeholders
        let suppress_non_callable_diagnostic = has_missing_member
            || (callee_has_primary_error && !is_super_call)
            || matches!(
                ctx.types.get_type(callee_ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Any
                }
            )
            || ctx.types.get_type(callee_ty_id).is_infer();
        if !suppress_non_callable_diagnostic {
            if is_super_call {
                self.error(AnalyzeError::NonCallable {
                    node: expression_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            } else {
                self.emit_non_callable_for_callee_type(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    callee_ty_id,
                );
            }
        }

        // infer arguments without expected types
        self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;

        if callee_has_primary_error {
            return Ok(ctx.types.insert_type_from(Type::Error, expression_id));
        }

        Ok(self.synthesize_call_error_result_type(expression_id, &mut *ctx.types))
    }

    /// Infer call arguments without contextual parameter types.
    fn infer_call_arguments_without_context(
        &self,
        ctx: &mut InferContext<'_>,
        arguments: &[LocalNodeId<Argument>],
        state: &mut InferState,
    ) -> AnalyzeResult<()> {
        for argument_id in arguments {
            self.infer_argument(&mut ctx.reborrow(), *argument_id, None, state)?;
        }

        Ok(())
    }

    /// Return true when one argument list contains malformed slots.
    fn arguments_have_error_slots(
        &self,
        tree: &NodeTree,
        arguments: &[LocalNodeId<Argument>],
    ) -> bool {
        arguments
            .iter()
            .any(|argument_id| matches!(tree.get(*argument_id), Argument::Error { .. }))
    }

    /// Synthesize the call-expression error recovery type for one expression.
    fn synthesize_call_error_result_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        types.insert_type_from(Type::Error, expression_id)
    }

    /// Infer and normalize the callee state for call-expression inference.
    fn infer_call_expression_callee(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<CallExpressionCalleeQuery> {
        let is_optional_chain = self.is_optional_chain_call_target(ctx.tree, left_id);
        let callee_ty_id = match ctx.tree.get(left_id) {
            Expression::Maybe { left } => {
                self.infer_expression(&mut ctx.reborrow(), *left, state)?
            }
            _ => self.infer_expression(&mut ctx.reborrow(), left_id, state)?,
        };

        if !is_optional_chain {
            return Ok(CallExpressionCalleeQuery::Callee(CallExpressionCallee {
                callee_ty_id,
                has_optional_nullish: false,
            }));
        }

        let (non_nullish, has_nullish) = self.strip_nullish_from_union(callee_ty_id, ctx.types);
        let Some(non_nullish) = non_nullish else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            };
            let type_id = ctx.types.insert_type_from(ty, expression_id);
            return Ok(CallExpressionCalleeQuery::EarlyType(type_id));
        };

        Ok(CallExpressionCalleeQuery::Callee(CallExpressionCallee {
            callee_ty_id: non_nullish,
            has_optional_nullish: has_nullish,
        }))
    }

    /// Resolve call-target metadata used for signature and instance resolution.
    fn resolve_call_expression_target(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        callee_ty_id: LocalTypeId,
        generic_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        state: &mut InferState,
    ) -> AnalyzeResult<CallExpressionResolution> {
        let call_has_static_arguments =
            generic_arguments.is_some_and(|arguments| !arguments.is_empty());
        let unwrapped_left_id = self.unwrap_parenthesized_expression(left_id, ctx.tree);
        match ctx.tree.get(unwrapped_left_id) {
            Expression::Member {
                left: receiver_id,
                name,
                generic_arguments: member_static_arguments,
                ..
            } => self.resolve_member_call_expression_target(
                &mut ctx.reborrow(),
                expression_id,
                unwrapped_left_id,
                *receiver_id,
                match *name {
                    Some(name) => name,
                    None => {
                        return Ok(CallExpressionResolution {
                            callee_symbol: None,
                            call_receiver_ty_id: None,
                            call_member_resolution: None,
                            member_call_context: None,
                            member_static_arguments: None,
                            inherited_static_arguments: Vec::new(),
                            inherited_substitutions: HashMap::new(),
                            member_instance_arguments: None,
                            prefilled_static_arguments: None,
                            super_constructor_value_ty_id: None,
                            has_static_argument_conflict: false,
                            call_has_static_arguments,
                        });
                    }
                },
                Some(member_static_arguments.clone()),
                call_has_static_arguments,
                state,
            ),
            _ => self.resolve_non_member_call_expression_target(
                &mut ctx.reborrow(),
                expression_id,
                unwrapped_left_id,
                callee_ty_id,
                call_has_static_arguments,
            ),
        }
    }

    /// Resolve member-call target metadata for signature and instance resolution.
    fn resolve_member_call_expression_target(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        unwrapped_left_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        member_name: StringId,
        member_static_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        call_has_static_arguments: bool,
        state: &mut InferState,
    ) -> AnalyzeResult<CallExpressionResolution> {
        // query receiver type and receiver lookup context
        let receiver_ty_id =
            self.infer_member_call_receiver_type(&mut ctx.reborrow(), receiver_id, state)?;
        let receiver_ty = ctx.types.get_type(receiver_ty_id).clone();
        let receiver_context = self.query_member_receiver_context_for_expression(
            &ctx.type_context_reborrow(),
            receiver_id,
            Some(receiver_ty_id),
        );

        // build member-call context and inherited static substitutions
        let member_key = StaticKey::Name(member_name);
        let member_call_context = Some(MemberCallContext {
            receiver_id,
            receiver_ty_id,
            member_key,
        });
        let inherited = self.resolve_inherited_static_arguments(
            &mut ctx.reborrow(),
            receiver_id.into_any(),
            Some(receiver_ty_id),
            &receiver_ty,
        )?;
        let inherited_static_arguments = inherited.arguments;
        let mut inherited_substitutions = inherited.substitutions;

        // resolve the member target for receiver and lookup mode
        let (member_resolution, member_symbol) = {
            let member_resolution = self.resolve_member_symbol_for_receiver(
                &mut ctx.reborrow(),
                receiver_id,
                receiver_id,
                &receiver_ty,
                &receiver_context,
                &member_key,
            )?;
            let member_symbol = match &member_resolution {
                MemberResolution::Static { symbol } => Some(*symbol),
                _ => None,
            };
            (member_resolution, member_symbol)
        };
        if let Some(member_symbol) = member_symbol {
            self.extend_owner_substitutions_from_inherited(
                &mut ctx.type_context_reborrow(),
                receiver_id.into_any(),
                member_symbol,
                &inherited_static_arguments,
                &mut inherited_substitutions,
            );
        }

        // query extension supplied static arguments for this member call
        let prefilled_static_arguments = if let Some(member_symbol) = member_symbol {
            let context = self.resolve_extension_member_context(
                &mut ctx.type_context_reborrow(),
                receiver_id.into_any(),
                member_symbol,
                &inherited_static_arguments,
            )?;
            context.map(|context| context.arguments)
        } else {
            None
        };

        // check static argument conflicts between call and member sites
        let member_has_static_arguments = member_static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty());
        let has_static_argument_conflict = self.check_call_static_argument_conflict(
            &ctx.reborrow(),
            expression_id,
            call_has_static_arguments,
            member_has_static_arguments,
        );

        // query receiver/member instance arguments when member syntax supplied them
        let member_instance_arguments = if call_has_static_arguments || !member_has_static_arguments
        {
            None
        } else {
            self.query_instance_arguments_for_node_infer(
                unwrapped_left_id.into_global_any(ctx.module.id),
                member_symbol,
                &*ctx.infer,
                &*ctx.types,
            )
        };
        Ok(CallExpressionResolution {
            callee_symbol: member_symbol,
            call_receiver_ty_id: Some(receiver_ty_id),
            call_member_resolution: Some(member_resolution),
            member_call_context,
            member_static_arguments,
            inherited_static_arguments,
            inherited_substitutions,
            member_instance_arguments,
            prefilled_static_arguments,
            super_constructor_value_ty_id: None,
            has_static_argument_conflict,
            call_has_static_arguments,
        })
    }

    /// Resolve non-member call target metadata for signature and instance resolution.
    fn resolve_non_member_call_expression_target(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        unwrapped_left_id: LocalNodeId<Expression>,
        callee_ty_id: LocalTypeId,
        call_has_static_arguments: bool,
    ) -> AnalyzeResult<CallExpressionResolution> {
        let is_super_constructor_call =
            self.expression_is_super_reference_for_call(ctx.tree, unwrapped_left_id);
        if is_super_constructor_call {
            let mut super_constructor_value_ty_id = None;
            if let Some(super_symbol) = self.super_symbol_for_type(callee_ty_id, &*ctx.types) {
                super_constructor_value_ty_id = self.super_constructor_value_type_for_symbol(
                    &mut ctx.reborrow(),
                    expression_id.into_any(),
                    super_symbol,
                )?;
            }

            let super_symbol =
                self.super_constructor_symbol_from_type(&mut ctx.reborrow(), callee_ty_id)?;
            return Ok(CallExpressionResolution {
                callee_symbol: super_symbol,
                call_receiver_ty_id: None,
                call_member_resolution: None,
                member_call_context: None,
                member_static_arguments: None,
                inherited_static_arguments: Vec::new(),
                inherited_substitutions: HashMap::new(),
                member_instance_arguments: None,
                prefilled_static_arguments: None,
                super_constructor_value_ty_id,
                has_static_argument_conflict: false,
                call_has_static_arguments,
            });
        }

        let callee_symbol =
            self.reference_symbol_for_expression(ctx.tree_symbol_view(), unwrapped_left_id);
        Ok(CallExpressionResolution {
            callee_symbol,
            call_receiver_ty_id: None,
            call_member_resolution: None,
            member_call_context: None,
            member_static_arguments: None,
            inherited_static_arguments: Vec::new(),
            inherited_substitutions: HashMap::new(),
            member_instance_arguments: None,
            prefilled_static_arguments: None,
            super_constructor_value_ty_id: None,
            has_static_argument_conflict: false,
            call_has_static_arguments,
        })
    }

    /// Infer the receiver type for a member call target.
    fn infer_member_call_receiver_type(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Expression::Maybe { left } = ctx.tree.get(receiver_id) {
            let receiver_ty_id = self.infer_expression(&mut ctx.reborrow(), *left, state)?;
            let (non_nullish, _) = self.strip_nullish_from_union(receiver_ty_id, ctx.types);
            return Ok(non_nullish.unwrap_or(receiver_ty_id));
        }

        if let Some(receiver_ty_id) = ctx
            .infer
            .inferred_type_for_node(receiver_id.into_global_any(ctx.module.id))
        {
            let receiver_unwrapped_ty_id =
                self.ensure_unwrapped_value_type_evaluated(&mut ctx.reborrow(), receiver_ty_id)?;
            if !self.unwrapped_value_type_is_unevaluated(receiver_unwrapped_ty_id, ctx.types) {
                return Ok(receiver_ty_id);
            }
        }

        self.infer_expression(&mut ctx.reborrow(), receiver_id, state)
    }

    /// Report whether call and member static arguments conflict.
    fn check_call_static_argument_conflict(
        &self,
        ctx: &InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        call_has_static_arguments: bool,
        member_has_static_arguments: bool,
    ) -> bool {
        let has_static_argument_conflict = call_has_static_arguments && member_has_static_arguments;
        if has_static_argument_conflict {
            self.error(AnalyzeError::ConflictingStaticArguments {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        has_static_argument_conflict
    }

    /// Query effective static arguments for direct call-signature resolution.
    fn query_call_signature_static_arguments<'a>(
        &self,
        generic_arguments: Option<&'a [LocalNodeId<GenericArgument>]>,
        call: &'a CallExpressionResolution,
    ) -> Option<&'a [LocalNodeId<GenericArgument>]> {
        if call.has_static_argument_conflict {
            return None;
        }

        if call.call_has_static_arguments {
            return generic_arguments;
        }
        None
    }

    /// Query effective static arguments for union member-call candidate resolution.
    fn query_union_member_call_static_arguments<'a>(
        &self,
        generic_arguments: Option<&'a [LocalNodeId<GenericArgument>]>,
        call: &'a CallExpressionResolution,
    ) -> Option<&'a [LocalNodeId<GenericArgument>]> {
        if call.has_static_argument_conflict {
            return None;
        }

        if call.call_has_static_arguments {
            return generic_arguments;
        }

        call.member_static_arguments.as_deref()
    }

    /// Infer dynamic union member-call dispatch when the receiver is a union.
    fn infer_union_member_call_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_call_context: Option<&MemberCallContext>,
        union_static_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        arguments: &[LocalNodeId<Argument>],
        state: &mut InferState,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(context) = member_call_context else {
            return Ok(None);
        };
        let Some(element_ids) =
            self.query_union_member_call_element_types(context.receiver_ty_id, &*ctx.types)
        else {
            return Ok(None);
        };

        // resolve union candidates for the member call
        let candidates = self.resolve_union_member_call_candidates(
            &mut ctx.reborrow(),
            expression_id,
            context.receiver_id,
            context.receiver_ty_id,
            &element_ids,
            &context.member_key,
            union_static_arguments,
            arguments,
        )?;
        let Some(candidates) = candidates else {
            self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;
            self.record_provisional_unresolved_resolution(
                expression_id.into_global_any(ctx.module.id),
                Some(context.receiver_ty_id),
                Vec::new(),
                Vec::new(),
                &mut *ctx.infer,
                &mut *ctx.types,
            );

            return Ok(Some(self.synthesize_call_error_result_type(
                expression_id,
                &mut *ctx.types,
            )));
        };

        let argument_ty_ids = self.infer_union_member_call_argument_types(
            &mut ctx.reborrow(),
            arguments,
            &candidates,
            state,
        )?;
        self.infer_union_member_call_argument_constraints(
            &candidates,
            &argument_ty_ids,
            &mut *ctx.infer,
        );

        if !self.check_union_member_call_argument_assignability(
            &mut ctx.reborrow(),
            &argument_ty_ids,
            &candidates,
        ) {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                context.receiver_ty_id,
            );

            return Ok(Some(
                self.synthesize_call_error_result_type(expression_id, ctx.types),
            ));
        }

        let return_type_id = self.infer_union_member_call_result_type(
            expression_id,
            context.receiver_ty_id,
            &candidates,
            &mut *ctx.types,
        );
        self.record_union_member_call_resolution(
            &mut ctx.reborrow(),
            expression_id,
            context.receiver_ty_id,
            candidates,
        )?;

        Ok(Some(return_type_id))
    }

    /// Query union element types for a union member call receiver.
    fn query_union_member_call_element_types(
        &self,
        receiver_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<Vec<LocalTypeId>> {
        let Type::Union { elements } = types.get_type(receiver_ty_id) else {
            return None;
        };

        Some(elements.clone())
    }

    /// Query uniform expected argument types across union-call candidates.
    fn query_union_member_call_expected_argument_types(
        &self,
        candidates: &[UnionMemberCallCandidate],
        argument_count: usize,
    ) -> Vec<Option<LocalTypeId>> {
        let mut expected_argument_types = Vec::with_capacity(argument_count);
        for index in 0..argument_count {
            let mut expected = None;
            let mut is_uniform = true;
            for candidate in candidates {
                let Some(param_ty_id) = candidate.signature.parameters.get(index).copied() else {
                    is_uniform = false;
                    break;
                };
                if let Some(current) = expected {
                    if current != param_ty_id {
                        is_uniform = false;
                        break;
                    }
                } else {
                    expected = Some(param_ty_id);
                }
            }
            expected_argument_types.push(if is_uniform { expected } else { None });
        }

        expected_argument_types
    }

    /// Infer argument types for one union member-call dispatch.
    fn infer_union_member_call_argument_types(
        &self,
        ctx: &mut InferContext<'_>,
        arguments: &[LocalNodeId<Argument>],
        candidates: &[UnionMemberCallCandidate],
        state: &mut InferState,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        let expected_argument_types =
            self.query_union_member_call_expected_argument_types(candidates, arguments.len());

        let mut argument_ty_ids = Vec::with_capacity(arguments.len());
        for (index, argument_id) in arguments.iter().enumerate() {
            let expected_arg_ty_id = expected_argument_types.get(index).copied().flatten();
            self.infer_argument(&mut ctx.reborrow(), *argument_id, expected_arg_ty_id, state)?;

            let argument = ctx.tree.get(*argument_id);
            let argument_value_id = argument.value();
            let argument_ty_id = if let Some(ty_id) = ctx
                .infer
                .inferred_type_for_node(argument_value_id.into_global_any(ctx.module.id))
            {
                let argument_unwrapped_ty_id =
                    self.ensure_unwrapped_value_type_evaluated(&mut ctx.reborrow(), ty_id)?;
                if self.unwrapped_value_type_is_unevaluated(argument_unwrapped_ty_id, ctx.types) {
                    self.infer_expression(&mut ctx.reborrow(), argument_value_id, state)?
                } else {
                    ty_id
                }
            } else {
                self.infer_expression(&mut ctx.reborrow(), argument_value_id, state)?
            };
            argument_ty_ids.push(argument_ty_id);
        }

        Ok(argument_ty_ids)
    }

    /// Infer subtype constraints between call arguments and union candidate parameters.
    fn infer_union_member_call_argument_constraints(
        &self,
        candidates: &[UnionMemberCallCandidate],
        argument_ty_ids: &[LocalTypeId],
        infer: &mut InferTable,
    ) {
        for candidate in candidates {
            for (argument_ty_id, param_ty_id) in argument_ty_ids
                .iter()
                .zip(candidate.signature.parameters.iter())
            {
                infer.push_constraint(Constraint::Subtype {
                    sub_type: *argument_ty_id,
                    super_type: *param_ty_id,
                    variance: None,
                });
            }
        }
    }

    /// Check argument assignability against every union-call candidate.
    fn check_union_member_call_argument_assignability(
        &self,
        ctx: &mut InferContext<'_>,
        argument_ty_ids: &[LocalTypeId],
        candidates: &[UnionMemberCallCandidate],
    ) -> bool {
        for candidate in candidates {
            for (argument_ty_id, param_ty_id) in argument_ty_ids
                .iter()
                .zip(candidate.signature.parameters.iter())
            {
                if !self.is_signature_candidate_argument_assignable(
                    &mut ctx.reborrow(),
                    *param_ty_id,
                    *argument_ty_id,
                ) {
                    return false;
                }
            }
        }

        true
    }

    /// Check candidate argument assignability, deferring unresolved inference state during overload filtering.
    pub(crate) fn is_signature_candidate_argument_assignable(
        &self,
        ctx: &mut InferContext<'_>,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
    ) -> bool {
        // fast path: direct infer vars defer to solve
        let has_unresolved_infer = self.is_infer_var_type(target_type_id, ctx.types)
            || self.is_infer_var_type(source_type_id, ctx.types);
        let is_assignable = self.is_type_assignable(
            &mut ctx.type_context_reborrow(),
            target_type_id,
            source_type_id,
        ) != Assignability::NotAssignable;
        if has_unresolved_infer || is_assignable {
            return true;
        }

        // defer relation checks that depend on unresolved convergence state
        if self.type_requires_infer_convergence(ctx.type_view(), target_type_id) {
            return true;
        }
        if self.type_requires_infer_convergence(ctx.type_view(), source_type_id) {
            return true;
        }

        false
    }

    /// Infer one return type for union member-call dispatch candidates.
    fn infer_union_member_call_result_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        candidates: &[UnionMemberCallCandidate],
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut void_type_id = None;
        let mut return_type_ids = Vec::new();
        for candidate in candidates {
            let return_type_id = match candidate.signature.return_type {
                Some(return_type_id) => return_type_id,
                None => *void_type_id.get_or_insert_with(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    types.insert_type_from(ty, expression_id)
                }),
            };
            return_type_ids.push(return_type_id);
        }

        match return_type_ids.len() {
            0 => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }
            1 => return_type_ids[0],
            _ => self.union_type_from_list(return_type_ids, receiver_ty_id, types),
        }
    }

    /// Record dynamic resolution candidates for union member-call dispatch.
    fn record_union_member_call_resolution(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        candidates: Vec<UnionMemberCallCandidate>,
    ) -> AnalyzeResult<()> {
        let source_node_id = expression_id.into_global_any(ctx.module.id);
        let mut deferred_candidate_attachments = Vec::new();
        let mut resolution_candidates = Vec::with_capacity(candidates.len());
        for (candidate_index, candidate) in candidates.into_iter().enumerate() {
            let instance_id = if let Some(environment) = candidate.instance_environment {
                let (instance_id, obligation_id) = self
                    .record_symbol_provisional_instance_with_obligation(
                        candidate.symbol,
                        environment,
                        ctx.infer,
                        ctx.types,
                    )?;
                if let Some(obligation_id) = obligation_id {
                    deferred_candidate_attachments.push((candidate_index, obligation_id));
                }
                instance_id
            } else {
                None
            };

            resolution_candidates.push(ResolutionCandidate {
                key: Some(DispatchKey::single(candidate.receiver_ty_id)),
                target_symbol: candidate.symbol,
                instance: instance_id,
                resolved_signature: Some(candidate.signature),
            });
        }

        self.record_provisional_dynamic_resolution(
            source_node_id,
            Some(receiver_ty_id),
            resolution_candidates,
            ctx.infer,
            ctx.types,
        );
        for (candidate_index, obligation_id) in deferred_candidate_attachments {
            let candidate_slot = DynamicResolutionCandidateSlotId::new(candidate_index as u32);
            ctx.infer
                .push_instance_commit_obligation_for_resolution_candidate(
                    source_node_id,
                    candidate_slot,
                    obligation_id,
                );
        }

        Ok(())
    }

    /// Check whether a call expression is an optional chain call target.
    fn is_optional_chain_call_target(
        &self,
        tree: &NodeTree,
        left_id: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(left_id) {
            Expression::Maybe { .. } => true,
            Expression::Member { left, .. } | Expression::Index { left, .. } => {
                matches!(tree.get(*left), Expression::Maybe { .. })
            }
            _ => false,
        }
    }

    /// Return true when a call target is exactly `super`.
    fn expression_is_super_reference_for_call(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        matches!(tree.get(expression_id), Expression::Super)
    }

    /// Resolve the explicit constructor symbol for a super constructor call.
    fn super_constructor_symbol_from_type(
        &self,
        ctx: &mut InferContext<'_>,
        super_ty_id: LocalTypeId,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve the base symbol from the inferred super type
        let Some(base_symbol) = self.super_symbol_for_type(super_ty_id, ctx.types) else {
            return Ok(None);
        };

        self.explicit_constructor_symbol_for_class(ctx, base_symbol)
    }

    /// Resolve the nominal symbol from an inferred super type.
    fn super_symbol_for_type(
        &self,
        super_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        match types.get_type(super_ty_id) {
            Type::Reference { symbol, .. } => Some(*symbol),
            Type::Value { value } => self.super_symbol_for_type(*value, types),
            _ => None,
        }
    }

    /// Resolve the value type for a super constructor target symbol.
    fn super_constructor_value_type_for_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        super_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // reuse local value types when they are already available
        if let Some(value_ty_id) = ctx.types.get_value_type_id(super_symbol) {
            return Ok(Some(value_ty_id));
        }

        // resolve local symbols from the current module table
        if super_symbol.module_id == ctx.module.id {
            return Ok(ctx.types.get_value_type_id(super_symbol));
        }

        // import remote value types on demand
        let value_ty_id = self.resolve_remote_symbol_value_type(
            &mut ctx.type_context_reborrow(),
            node_id,
            super_symbol,
            RemoteValueTypeReadDomain::Interface,
        )?;

        Ok(Some(value_ty_id))
    }

    /// Resolve an explicit constructor method symbol for a class.
    fn explicit_constructor_symbol_for_class(
        &self,
        ctx: &InferContext<'_>,
        class_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        self.with_module_tree_symbol_view_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            class_symbol.module_id,
            ctx.tree,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                // resolve the nominal declaration for the class symbol
                let class_entry = view.symbols.get_symbol(class_symbol.local_id);
                let declaration_id = class_entry.primary_declaration?.local_id;
                if declaration_id.ty != NodeType::Declaration {
                    return None;
                }

                // resolve class members and locate the explicit constructor method
                let declaration_id = declaration_id.into_typed::<Declaration>();
                let Declaration::Class(declaration) = view.tree.get(declaration_id) else {
                    return None;
                };
                let members = &declaration.members;

                for member_id in members {
                    let member = view.tree.get(*member_id);
                    let Member::Method {
                        signature, symbol, ..
                    } = member
                    else {
                        continue;
                    };

                    if signature.mode == Some(FunctionMode::Constructor) {
                        return Some(symbol.into_global(view.module.id));
                    }
                }

                None
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Infer a constructor call expression.
    pub(crate) fn infer_new_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        generic_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        arguments: &[LocalNodeId<Argument>],
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // enforce constructor restrictions from options
        self.validate_call_expression(ctx, expression_id, left_id, true);

        // query and normalize the constructor target
        let target =
            self.infer_new_expression_target(&mut ctx.reborrow(), expression_id, left_id, state)?;

        // malformed argument slots poison the whole construction
        if self.arguments_have_error_slots(ctx.tree, arguments) {
            self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;

            return Ok(self.synthesize_call_error_result_type(expression_id, &mut *ctx.types));
        }

        // resolve construct signatures for the callee type
        let construct_signatures =
            self.construct_signatures_for_type(target.callee_ty_id, &*ctx.types);
        let ty_id = if construct_signatures.is_empty() {
            self.infer_non_constructable_new_expression(
                &mut ctx.reborrow(),
                expression_id,
                &target,
                arguments,
                state,
            )?
        } else {
            // resolve one concrete constructor signature
            let selection = self.select_call_signature(
                &mut ctx.reborrow(),
                &construct_signatures,
                OverloadSelectionContext {
                    expression_id,
                    callee_symbol: target.callee_symbol,
                    generic_arguments,
                    prefilled_static_arguments: None,
                    bound_substitutions: None,
                    arguments,
                    call_receiver_ty_id: None,
                    mode: SignatureResolutionMode::Synthesize,
                },
            )?;
            if let Some((_, resolved_signature)) = selection {
                self.infer_resolved_new_expression(
                    &mut ctx.reborrow(),
                    expression_id,
                    &target,
                    arguments,
                    resolved_signature,
                    state,
                )?
            } else if construct_signatures.len() > 1 {
                self.emit_no_overload_for_receiver_type(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    target.callee_ty_id,
                );
                self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;
                self.synthesize_call_error_result_type(expression_id, &mut *ctx.types)
            } else {
                let signature_ty_id = construct_signatures[0];
                let resolved = self.resolve_call_signature(
                    &mut ctx.reborrow(),
                    signature_ty_id,
                    CallSignatureResolutionContext {
                        expression_id,
                        callee_symbol: target.callee_symbol,
                        generic_arguments,
                        prefilled_static_arguments: None,
                        bound_substitutions: None,
                        arguments: Some(arguments),
                        call_receiver_ty_id: None,
                        expected_return_type: state.expected_type,
                        mode: SignatureResolutionMode::Synthesize,
                        allow_missing_value_arguments: false,
                    },
                )?;
                if let Some(resolved_signature) = resolved {
                    self.infer_resolved_new_expression(
                        &mut ctx.reborrow(),
                        expression_id,
                        &target,
                        arguments,
                        resolved_signature,
                        state,
                    )?
                } else {
                    self.synthesize_call_error_result_type(expression_id, &mut *ctx.types)
                }
            }
        };

        // reject managed allocations when managed memory is disabled
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

    /// Infer and normalize constructor target metadata for new-expression inference.
    fn infer_new_expression_target(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<NewExpressionResolution> {
        let callee_ty_id = self.infer_expression(&mut ctx.reborrow(), left_id, state)?;

        // ensure instance types for constructor references
        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            expression_id.into_any(),
            callee_ty_id,
        )?;

        // resolve the constructor symbol when possible
        let callee_id = self.unwrap_parenthesized_expression(left_id, ctx.tree);
        let callee_symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), callee_id);
        let struct_constructor_symbol = callee_symbol.map(|symbol| {
            self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::FollowAliases,
            )
        });
        let is_struct_constructor =
            struct_constructor_symbol.is_some_and(|symbol| symbol.ty() == SymbolType::Struct);

        Ok(NewExpressionResolution {
            callee_ty_id,
            callee_symbol,
            is_struct_constructor,
        })
    }

    /// Infer constructor arguments, commit constructor resolution, and return the constructed type.
    fn infer_resolved_new_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        target: &NewExpressionResolution,
        arguments: &[LocalNodeId<Argument>],
        resolved_signature: ResolvedSignature,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let resolved_parameters = &resolved_signature.parameters;

        // enforce exact arity for struct constructors
        if target.is_struct_constructor && arguments.len() != resolved_parameters.len() {
            self.emit_no_overload_for_receiver_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                target.callee_ty_id,
            );
            self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;

            return Ok(self.synthesize_call_error_result_type(expression_id, &mut *ctx.types));
        }

        // infer argument types and constraints
        let argument_ty_ids = self.infer_invocation_arguments(
            &mut ctx.reborrow(),
            arguments,
            resolved_parameters,
            None,
            state,
        )?;

        // check argument assignability against parameters
        self.check_invocation_assignability(
            &mut ctx.reborrow(),
            expression_id,
            arguments,
            &argument_ty_ids,
            resolved_parameters,
            &state.options,
        )?;

        let resolved_return_type = resolved_signature.return_type;

        // register the constructor instance when static arguments were resolved
        let constructor_instance_id = if let Some(callee_symbol) = target.callee_symbol {
            let environment = self.instance_environment_for_symbol_arguments(
                &ctx.reborrow(),
                callee_symbol,
                resolved_signature.generic_arguments.clone(),
                0,
            );
            if let Some(environment) = environment {
                self.record_node_provisional_instance(
                    expression_id.into_global_any(ctx.module.id),
                    callee_symbol,
                    environment,
                    &mut *ctx.infer,
                    &mut *ctx.types,
                )?
            } else {
                None
            }
        } else {
            None
        };

        // commit constructor resolution when possible
        if let Some(callee_symbol) = target.callee_symbol {
            self.record_provisional_static_resolution(
                expression_id.into_global_any(ctx.module.id),
                None,
                callee_symbol,
                constructor_instance_id,
                Some(resolved_signature),
                &mut *ctx.infer,
                &mut *ctx.types,
            );
        }

        Ok(resolved_return_type.unwrap_or_else(|| {
            self.synthesize_call_error_result_type(expression_id, &mut *ctx.types)
        }))
    }

    /// Infer behavior for non-constructable new-expression targets.
    fn infer_non_constructable_new_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        target: &NewExpressionResolution,
        arguments: &[LocalNodeId<Argument>],
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // reuse the instance type for class and struct constructors without signatures
        if let Some(callee_symbol) = target.callee_symbol
            && matches!(callee_symbol.ty(), SymbolType::Struct | SymbolType::Class)
            && let Some(instance_type_id) = ctx.types.get_instance_type_id(callee_symbol)
        {
            self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;

            return Ok(instance_type_id);
        }

        // infer arguments and return an error type when no constructor target is available
        self.infer_call_arguments_without_context(&mut ctx.reborrow(), arguments, state)?;

        Ok(self.synthesize_call_error_result_type(expression_id, &mut *ctx.types))
    }

    /// Resolve static arguments and substitutions for a function type.
    pub(crate) fn resolve_function_static_arguments(
        &self,
        ctx: &mut InferContext<'_>,
        context: SignatureStaticResolutionContext<'_>,
    ) -> AnalyzeResult<Option<ResolvedSignature>> {
        // handle fast paths when there are no static parameters
        let has_static_arguments = context
            .generic_argument_ids
            .is_some_and(|args| !args.is_empty());

        if context.generic_parameter_type_ids.is_empty() && !has_static_arguments {
            return Ok(None);
        }

        if context.generic_parameter_type_ids.is_empty() {
            if let Some(argument_ids) = context.generic_argument_ids {
                for argument_id in argument_ids {
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: argument_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                        message: "too many static arguments".to_string(),
                    });
                }
            }

            return Ok(Some(ResolvedSignature {
                parameters: context.parameter_type_ids.to_vec(),
                return_type: context.return_type,
                generic_arguments: Vec::new(),
            }));
        }

        let generic_parameters = self.collect_signature_static_parameters(
            &mut ctx.reborrow(),
            context.node_id,
            context.generic_parameter_type_ids,
        );
        let assigned_arguments = self.assign_signature_static_arguments(
            &mut ctx.reborrow(),
            context.node_id,
            context.generic_argument_ids,
            context.prefilled_static_arguments,
            &generic_parameters,
            context.bound_substitutions,
        );
        let Some(resolved_state) = self.resolve_signature_static_argument_state(
            ctx,
            &context,
            &generic_parameters,
            assigned_arguments,
        )?
        else {
            return Ok(None);
        };
        let (resolved_parameters, resolved_return_type) = self
            .instantiate_signature_from_resolved_state(
                &mut ctx.reborrow(),
                context.node_id,
                context.owner_symbol,
                context.parameter_type_ids,
                context.return_type,
                &resolved_state,
            );

        Ok(Some(ResolvedSignature {
            parameters: resolved_parameters,
            return_type: resolved_return_type,
            generic_arguments: resolved_state.resolved_arguments,
        }))
    }

    /// Collect static parameter metadata for one signature.
    fn collect_signature_static_parameters(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        generic_parameter_type_ids: &[LocalTypeId],
    ) -> Vec<GenericParameterSpec> {
        let generic_parameter_symbols =
            self.generic_parameter_symbols_for_type_ids(generic_parameter_type_ids, ctx.types);
        let mut parameters = Vec::with_capacity(generic_parameter_symbols.len());
        for symbol_id in generic_parameter_symbols {
            parameters.push(self.resolve_static_parameter(
                &mut ctx.type_context_reborrow(),
                symbol_id,
                node_id,
            ));
        }

        parameters
    }

    /// Assign call and prefilled static arguments to signature parameters.
    fn assign_signature_static_arguments(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        generic_argument_ids: Option<&[LocalNodeId<GenericArgument>]>,
        prefilled_static_arguments: Option<&[StaticArgument]>,
        generic_parameters: &[GenericParameterSpec],
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
    ) -> Vec<Option<StaticArgument>> {
        let argument_ids = generic_argument_ids.unwrap_or(&[]);
        let argument_values = argument_ids
            .iter()
            .map(|argument_id| StaticArgument::Unevaluated {
                node: argument_id.into_global_any(ctx.module.id),
            })
            .collect::<Vec<_>>();
        // prefilled arguments are only meaningful when the signature still exposes
        // the corresponding owner parameters positionally
        let prefilled_arguments = if bound_substitutions.is_some() {
            &[]
        } else {
            prefilled_static_arguments.unwrap_or(&[])
        };
        let prefilled_count = prefilled_arguments.len().min(generic_parameters.len());
        let mut assigned_arguments: Vec<Option<StaticArgument>> =
            vec![None; generic_parameters.len()];

        // assign explicitly prefilled arguments first
        for (index, argument) in prefilled_arguments.iter().take(prefilled_count).enumerate() {
            assigned_arguments[index] = Some(argument.clone());
        }

        // reserve already bound substitution slots before assigning call-site arguments
        if let Some(bound_substitutions) = bound_substitutions {
            for (index, parameter) in generic_parameters.iter().enumerate() {
                if assigned_arguments[index].is_some() {
                    continue;
                }
                let Some(substitution_type_id) = bound_substitutions.get(&parameter.symbol) else {
                    continue;
                };
                assigned_arguments[index] = Some(StaticArgument::Evaluated {
                    name: parameter.name,
                    value: StaticExpression::Type {
                        ty: *substitution_type_id,
                    },
                });
            }
        }

        // map call-site arguments onto the remaining unbound static parameter slots
        let mut call_slot_indexes = Vec::new();
        let mut parameters_for_call = Vec::new();
        for (index, parameter) in generic_parameters.iter().enumerate() {
            if assigned_arguments[index].is_some() {
                continue;
            }
            call_slot_indexes.push(index);
            parameters_for_call.push(parameter.clone());
        }
        let assigned_for_call = self.assign_static_argument_values(
            TreeSymbolView::new(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                ctx.tree,
                ctx.symbols,
            ),
            node_id,
            &argument_values,
            &parameters_for_call,
        );

        for (index, argument) in assigned_for_call.into_iter().enumerate() {
            let Some(target_index) = call_slot_indexes.get(index).copied() else {
                break;
            };
            if assigned_arguments[target_index].is_none() {
                assigned_arguments[target_index] = argument;
            }
        }

        assigned_arguments
    }

    /// Resolve static arguments and substitutions for one signature.
    fn resolve_signature_static_argument_state(
        &self,
        ctx: &mut InferContext<'_>,
        context: &SignatureStaticResolutionContext<'_>,
        generic_parameters: &[GenericParameterSpec],
        assigned_arguments: Vec<Option<StaticArgument>>,
    ) -> AnalyzeResult<Option<ResolvedStaticArgumentState>> {
        let SignatureStaticResolutionContext {
            node_id,
            owner_symbol,
            bound_substitutions,
            argument_ids,
            parameter_type_ids,
            return_type,
            expected_return_type,
            mode,
            allow_missing_value_arguments,
            ..
        } = context;

        // query expected return type mapping for type parameter backfill
        let expected_return_mapping = if *mode == SignatureResolutionMode::Synthesize {
            if let (Some(return_type), Some(expected_return_type)) =
                (*return_type, *expected_return_type)
            {
                self.generic_arguments_from_expected_return_type(
                    &mut ctx.type_context_reborrow(),
                    return_type,
                    expected_return_type,
                )?
            } else {
                None
            }
        } else {
            None
        };

        // resolve one static argument slot at a time
        let mut substitutions = HashMap::new();
        let mut bound_substitutions = bound_substitutions.cloned().unwrap_or_default();
        let mut resolved_arguments = Vec::with_capacity(generic_parameters.len());
        let mut has_missing_value_argument = false;
        let mut has_value_substitution = false;
        for (index, generic_parameter) in generic_parameters.iter().enumerate() {
            let assigned_argument = assigned_arguments.get(index).cloned().flatten();

            // query inferred value arguments from dynamic argument positions
            let inferred_argument = if assigned_argument.is_some() {
                None
            } else if let Some(argument_ids) = *argument_ids {
                self.infer_static_argument_from_dynamic_arguments(
                    &mut ctx.reborrow(),
                    generic_parameter,
                    parameter_type_ids,
                    argument_ids,
                )?
            } else {
                None
            };

            // query expected argument backfill from return type compatibility
            let expected_argument = if assigned_argument.is_some()
                || generic_parameter.kind != GenericParameterKind::Type
            {
                None
            } else if let Some(expected_argument) = expected_return_mapping
                .as_ref()
                .and_then(|mapping| mapping.get(&generic_parameter.symbol))
                .cloned()
            {
                let call_site = TreeSymbolView::new(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                );
                let call_site_options = ctx.options;
                self.resolve_static_argument(
                    &mut ctx.type_context_reborrow(),
                    call_site,
                    call_site_options,
                    generic_parameter,
                    Some(expected_argument),
                    true,
                )?
            } else {
                None
            };

            // resolve one concrete static argument value for this slot
            let call_site = TreeSymbolView::new(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                ctx.tree,
                ctx.symbols,
            );
            let call_site_options = ctx.options;
            let resolved_assigned_argument = self.resolve_static_argument(
                &mut ctx.type_context_reborrow(),
                call_site,
                call_site_options,
                generic_parameter,
                assigned_argument,
                true,
            )?;
            let mut resolved_argument = if let Some(resolved_argument) = resolved_assigned_argument
            {
                resolved_argument
            } else if let Some(inferred_argument) = inferred_argument {
                inferred_argument
            } else if let Some(expected_argument) = expected_argument {
                expected_argument
            } else {
                if generic_parameter.kind == GenericParameterKind::Value
                    && *mode == SignatureResolutionMode::Synthesize
                    && *allow_missing_value_arguments
                {
                    return Ok(None);
                }

                if generic_parameter.kind == GenericParameterKind::Value {
                    has_missing_value_argument = true;
                }

                self.synthesize_missing_static_argument_for_function(
                    &mut ctx.reborrow(),
                    *node_id,
                    *owner_symbol,
                    generic_parameter,
                )?
            };

            // query the error-node anchor used by type-argument validation
            let error_node = match &resolved_argument {
                StaticArgument::Unevaluated { node } => *node,
                StaticArgument::Evaluated { .. } => (*node_id).into_global(ctx.module.id),
            };

            // validate and collect substitutions for this argument
            let materialized_substitution = if generic_parameter.kind == GenericParameterKind::Type
            {
                Some(self.materialize_static_type_argument(
                    &mut ctx.type_context_reborrow(),
                    error_node,
                    generic_parameter,
                    &resolved_argument,
                    StaticArgumentValidationMode::Analyze,
                )?)
            } else {
                None
            };
            let (mut ctx, infer) = ctx.split_type_context_and_infer();
            let substitution = self.validate_static_argument(
                &mut ctx,
                error_node,
                generic_parameter,
                &resolved_argument,
                materialized_substitution,
                &bound_substitutions,
                Some(infer),
            )?;
            if let Some(substitution_ty_id) = substitution {
                substitutions.insert(generic_parameter.symbol, substitution_ty_id);
                bound_substitutions.insert(generic_parameter.symbol, substitution_ty_id);
                resolved_argument = self.normalize_signature_static_argument_substitution(
                    generic_parameter,
                    substitution_ty_id,
                    resolved_argument,
                    &mut has_value_substitution,
                    ctx.types,
                );
            } else if generic_parameter.kind == GenericParameterKind::Type
                && let StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } = &resolved_argument
            {
                substitutions.insert(generic_parameter.symbol, *ty);
                bound_substitutions.insert(generic_parameter.symbol, *ty);
            }

            resolved_arguments.push(resolved_argument);
        }

        Ok(Some(ResolvedStaticArgumentState {
            resolved_arguments,
            substitutions,
            has_missing_value_argument,
            has_value_substitution,
        }))
    }

    /// Normalize value static arguments when substitution produced a concrete type.
    fn normalize_signature_static_argument_substitution(
        &self,
        generic_parameter: &GenericParameterSpec,
        substitution_ty_id: LocalTypeId,
        resolved_argument: StaticArgument,
        has_value_substitution: &mut bool,
        types: &mut TypeTable,
    ) -> StaticArgument {
        if generic_parameter.kind != GenericParameterKind::Value {
            return resolved_argument;
        }
        if matches!(types.get_type(substitution_ty_id), Type::Error) {
            return resolved_argument;
        }

        *has_value_substitution = true;
        let argument_name = match &resolved_argument {
            StaticArgument::Evaluated { name, .. } => *name,
            StaticArgument::Unevaluated { .. } => None,
        };
        let replacement = StaticArgument::Evaluated {
            name: argument_name,
            value: StaticExpression::Type {
                ty: substitution_ty_id,
            },
        };

        self.normalize_value_static_argument(replacement, types)
    }

    /// Instantiate the dynamic signature from resolved static substitutions.
    fn instantiate_signature_from_resolved_state(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        parameters: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
        state: &ResolvedStaticArgumentState,
    ) -> (Vec<LocalTypeId>, Option<LocalTypeId>) {
        // apply substitutions to the dynamic signature
        let mut materialize_cache = HashMap::new();
        let mut substitute_cache = HashMap::new();
        let resolved_parameters = if state.has_missing_value_argument {
            let error_ty_id = ctx.types.insert_type_from_any(Type::Error, node_id);
            parameters.iter().map(|_| error_ty_id).collect::<Vec<_>>()
        } else {
            parameters
                .iter()
                .map(|parameter| {
                    self.instantiate_type_with_substitutions(
                        &mut ctx.type_context_reborrow(),
                        node_id,
                        owner_symbol,
                        *parameter,
                        &state.substitutions,
                        &mut materialize_cache,
                        &mut substitute_cache,
                    )
                })
                .collect::<Vec<_>>()
        };
        let resolved_return_type = if state.has_missing_value_argument {
            Some(ctx.types.insert_type_from_any(Type::Error, node_id))
        } else {
            return_type.map(|return_type| {
                self.instantiate_type_with_substitutions(
                    &mut ctx.type_context_reborrow(),
                    node_id,
                    owner_symbol,
                    return_type,
                    &state.substitutions,
                    &mut materialize_cache,
                    &mut substitute_cache,
                )
            })
        };

        // normalize substituted value arguments for downstream assignability
        if state.has_value_substitution {
            let mut normalize_cache = HashMap::new();
            let normalized_dynamic_parameters = resolved_parameters
                .iter()
                .map(|parameter| {
                    self.materialize_static_arguments_in_type(
                        &mut ctx.type_context_reborrow(),
                        *parameter,
                        &mut normalize_cache,
                    )
                })
                .collect::<Vec<_>>();
            let normalized_return_type = resolved_return_type.map(|return_type| {
                self.materialize_static_arguments_in_type(
                    &mut ctx.type_context_reborrow(),
                    return_type,
                    &mut normalize_cache,
                )
            });
            (normalized_dynamic_parameters, normalized_return_type)
        } else {
            (resolved_parameters, resolved_return_type)
        }
    }

    /// Resolve a member function for an operator invocation.
    ///
    /// Handles the common pattern of looking up a member on a receiver type,
    /// applying inherited substitutions, and resolving the function signature.
    pub(crate) fn resolve_member_function(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty_id: Option<LocalTypeId>,
        receiver_ty: &Type,
        member_key: &StaticKey,
    ) -> AnalyzeResult<Option<ResolvedMemberFunction>> {
        // resolve member dispatch for the receiver type
        let member_resolution =
            self.resolve_member_symbol(&mut ctx.reborrow(), receiver_ty, member_key)?;
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // decide how to filter member lookups for this receiver
        let receiver_context = self.query_member_receiver_context_for_expression(
            &ctx.type_context_reborrow(),
            receiver_expression_id,
            receiver_ty_id,
        );
        let lookup_mode = receiver_context.lookup_mode;

        // resolve member-call typing context for this receiver
        let resolved_context = self.resolve_member_call_type_context(
            &mut ctx.reborrow(),
            receiver_expression_id,
            receiver_ty_id,
            receiver_ty,
            member_key,
            member_symbol,
            lookup_mode,
        )?;
        let has_member = resolved_context.member_ty_id.is_some();

        // bail if we don't know the member type
        let Some(member_ty_id) = resolved_context.member_ty_id else {
            return Ok(Some(ResolvedMemberFunction {
                signature: ResolvedSignature {
                    parameters: Vec::new(),
                    return_type: None,
                    generic_arguments: Vec::new(),
                },
                member_resolution,
                member_symbol,
                instance_arguments: resolved_context.instance_arguments,
                signature_static_parameter_symbols: Vec::new(),
                bound_substitutions: HashMap::new(),
                has_member: false,
            }));
        };

        // resolve function signature
        let Type::Function {
            generic_parameters,
            parameters,
            return_type,
            ..
        } = ctx.types.get_type(member_ty_id).clone()
        else {
            return Ok(None);
        };
        let signature_static_parameter_symbols =
            self.generic_parameter_symbols_for_type_ids(&generic_parameters, &*ctx.types);
        let signature = self
            .resolve_function_static_arguments(
                &mut ctx.reborrow(),
                SignatureStaticResolutionContext {
                    node_id: expression_id.into_any(),
                    owner_symbol: member_symbol,
                    generic_argument_ids: None,
                    prefilled_static_arguments: None,
                    bound_substitutions: (!resolved_context.substitutions.is_empty())
                        .then_some(&resolved_context.substitutions),
                    argument_ids: None,
                    generic_parameter_type_ids: &generic_parameters,
                    parameter_type_ids: &parameters,
                    return_type,
                    expected_return_type: None,
                    mode: SignatureResolutionMode::Check,
                    allow_missing_value_arguments: false,
                },
            )?
            .unwrap_or(ResolvedSignature {
                parameters,
                return_type,
                generic_arguments: Vec::new(),
            });

        Ok(Some(ResolvedMemberFunction {
            signature,
            member_resolution,
            member_symbol,
            instance_arguments: resolved_context.instance_arguments,
            signature_static_parameter_symbols,
            bound_substitutions: resolved_context.substitutions,
            has_member,
        }))
    }

    /// Record the member-call instance id for a resolved member invocation.
    pub(crate) fn record_member_call_instance_id(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        resolved: &ResolvedMemberFunction,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let Some(member_symbol) = resolved.member_symbol else {
            return Ok(None);
        };

        let environment = self.compose_member_instance_environment(
            &ctx.reborrow(),
            member_symbol,
            &resolved.instance_arguments,
            &resolved.bound_substitutions,
            &resolved.signature.generic_arguments,
            &resolved.signature_static_parameter_symbols,
        );
        let Some(environment) = environment else {
            return Ok(None);
        };

        self.record_node_provisional_instance(
            expression_id.into_global_any(ctx.module.id),
            member_symbol,
            environment,
            ctx.infer,
            ctx.types,
        )
    }

    /// Record resolution entries for a member function invocation.
    ///
    /// Returns the instance ID if one was committed.
    pub(crate) fn record_member_call_resolution(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        resolved: &ResolvedMemberFunction,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let instance_id =
            self.record_member_call_instance_id(&mut ctx.reborrow(), expression_id, resolved)?;

        // record member resolution
        self.record_provisional_member_resolution(
            expression_id.into_global_any(ctx.module.id),
            Some(receiver_ty_id),
            &resolved.member_resolution,
            instance_id,
            Some(resolved.signature.clone()),
            resolved.has_member,
            ctx.infer,
            ctx.types,
        );

        Ok(instance_id)
    }
}
