use std::collections::{HashMap, HashSet};

use super::SignatureResolutionMode;
use super::member::{MemberLookupMode, MemberReceiverContext, MemberResolution};
use crate::analyze::StaticSubstitutionEnvironment;
use crate::analyze::common::{AnalyzeDependencyStage, CanonicalSymbolMode};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    Argument, Constraint, Declaration, DispatchKey, DynamicResolutionCandidateSlotId, Expression,
    FunctionKind, FunctionMode, GlobalNodeIdAny, GlobalSymbolId, InferOrigin, InferTable,
    LocalInstanceId, LocalNodeId, LocalNodeIdAny, LocalTypeId, Member, NodeTree, NodeType,
    ResolutionCandidate, ResolvedSignature, StaticArgument, StaticExpression, StaticKey,
    StaticParameter, StaticParameterKind, StringId, SymbolTable, SymbolType, Type, TypeLiteral,
    TypeTable,
};
use destack_workspace::{Module, ModuleSource, ProfileId};

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

/// Resolution result for call-signature selection.
#[derive(Debug)]
enum CallExpressionSignatureResolution {
    /// A concrete signature was resolved.
    Resolved {
        /// The selected signature type id.
        signature_ty_id: LocalTypeId,
        /// The resolved signature.
        signature: ResolvedSignature,
    },
    /// Signature resolution produced an indeterminate call result type.
    IndeterminateType(LocalTypeId),
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
    /// The current module.
    module: &'a Module,
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
    static_arguments: Option<&'a [LocalNodeId<Argument>]>,
    /// Dynamic arguments used for overload selection.
    dynamic_arguments: &'a [LocalNodeId<Argument>],
    /// The active profile.
    profile: ProfileId,
    /// Analyze options for relation checks.
    options: &'a AnalyzeOptions,
    /// The current module tree.
    tree: &'a NodeTree,
    /// The current module symbol table.
    symbols: &'a SymbolTable,
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

    /// Select a callable signature type for a callee type when possible.
    pub(crate) fn call_signature_for_type(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        self.call_signatures_for_type(ty_id, types).first().copied()
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
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        callee_symbol: Option<GlobalSymbolId>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        prefilled_static_arguments: Option<&[StaticArgument]>,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        dynamic_arguments: Option<&[LocalNodeId<Argument>]>,
        signature_ty_id: LocalTypeId,
        call_receiver_ty_id: Option<LocalTypeId>,
        expected_return_type: Option<LocalTypeId>,
        mode: SignatureResolutionMode,
        allow_missing_value_arguments: bool,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<ResolvedSignature>> {
        let Type::Function {
            static_parameters,
            dynamic_parameters,
            return_type,
            ..
        } = types.get_type(signature_ty_id).clone()
        else {
            return Ok(None);
        };

        let has_explicit_static_arguments =
            static_arguments.is_some_and(|static_arguments| !static_arguments.is_empty());
        if has_explicit_static_arguments && static_parameters.is_empty() {
            if let Some(callee_symbol) = callee_symbol {
                let parameter_symbols = self
                    .collect_static_parameter_symbols(
                        module,
                        callee_symbol,
                        profile,
                        tree,
                        symbols,
                        types,
                    )
                    .unwrap_or_default();
                if !parameter_symbols.is_empty() {
                    return Err(AnalyzeError::Internal {
                        message: format!(
                            "missing signature static parameters for generic callable {callee_symbol:?}"
                        ),
                    });
                }
            }
        }

        let resolved = self.resolve_function_signature(
            module,
            expression_id.into_any(),
            callee_symbol,
            static_arguments,
            prefilled_static_arguments,
            bound_substitutions,
            dynamic_arguments,
            &static_parameters,
            &dynamic_parameters,
            return_type,
            expected_return_type,
            mode,
            allow_missing_value_arguments,
            profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?;

        // substitute `this` for member calls
        if let Some(receiver_ty_id) = call_receiver_ty_id {
            let mut cache = HashMap::new();
            let mapped_parameters = resolved
                .dynamic_parameters
                .iter()
                .map(|parameter| {
                    self.substitute_this_type(*parameter, receiver_ty_id, types, &mut cache)
                })
                .collect::<Vec<_>>();
            let mapped_return = resolved.return_type.map(|return_type| {
                self.substitute_this_type(return_type, receiver_ty_id, types, &mut cache)
            });

            return Ok(Some(ResolvedSignature {
                dynamic_parameters: mapped_parameters,
                return_type: mapped_return,
                static_arguments: resolved.static_arguments,
            }));
        }

        Ok(Some(resolved))
    }

    /// Select the matching call signature overload for a call expression.
    fn select_call_signature(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        callee_symbol: Option<GlobalSymbolId>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        prefilled_static_arguments: Option<&[StaticArgument]>,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        signature_ids: &[LocalTypeId],
        dynamic_arguments: &[LocalNodeId<Argument>],
        call_receiver_ty_id: Option<LocalTypeId>,
        mode: SignatureResolutionMode,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<(LocalTypeId, ResolvedSignature)>> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_OVERLOAD_RESOLVE);

        // only attempt selection when overloads exist
        if signature_ids.len() <= 1 {
            return Ok(None);
        }

        // resolve and filter applicable overloads
        let candidates = self.collect_applicable_signatures(
            module,
            expression_id,
            callee_symbol,
            static_arguments,
            prefilled_static_arguments,
            bound_substitutions,
            signature_ids,
            dynamic_arguments,
            call_receiver_ty_id,
            mode,
            profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?;

        // drop equivalent overloads introduced by declaration merging
        let mut candidates =
            self.dedupe_signature_candidates(module, profile, candidates, symbols, types, options);

        if candidates.is_empty() {
            return Ok(None);
        }
        if candidates.len() == 1 {
            return Ok(Some(candidates.remove(0)));
        }

        // prefer the first applicable signature in declaration order
        Ok(candidates.into_iter().next())
    }

    /// Resolve and filter applicable overloads for call selection.
    fn collect_applicable_signatures(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        callee_symbol: Option<GlobalSymbolId>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        prefilled_static_arguments: Option<&[StaticArgument]>,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        signature_ids: &[LocalTypeId],
        dynamic_arguments: &[LocalNodeId<Argument>],
        call_receiver_ty_id: Option<LocalTypeId>,
        mode: SignatureResolutionMode,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Vec<(LocalTypeId, ResolvedSignature)>> {
        // collect resolved signatures that apply to the call site
        let mut candidates = Vec::new();

        for signature_ty_id in signature_ids {
            let Some(resolved) = self.resolve_call_signature(
                module,
                expression_id,
                callee_symbol,
                static_arguments,
                prefilled_static_arguments,
                bound_substitutions,
                Some(dynamic_arguments),
                *signature_ty_id,
                call_receiver_ty_id,
                None,
                mode,
                true,
                profile,
                options,
                tree,
                symbols,
                types,
                infer,
            )?
            else {
                continue;
            };

            if !self.is_signature_applicable(
                module,
                profile,
                &resolved,
                dynamic_arguments,
                tree,
                symbols,
                types,
                options,
            )? {
                continue;
            }

            candidates.push((*signature_ty_id, resolved));
        }

        Ok(candidates)
    }

    /// Drop duplicate overloads that resolve to equivalent shapes.
    pub(crate) fn dedupe_signature_candidates(
        &self,
        module: &Module,
        profile: ProfileId,
        candidates: Vec<(LocalTypeId, ResolvedSignature)>,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Vec<(LocalTypeId, ResolvedSignature)> {
        let mut deduped = Vec::new();

        for (signature_id, resolved) in candidates {
            let already_seen = deduped.iter().any(|(_, existing)| {
                self.signature_shapes_equivalent(
                    module, profile, &resolved, existing, symbols, types, options,
                )
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
        module: &Module,
        profile: ProfileId,
        left: &ResolvedSignature,
        right: &ResolvedSignature,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        // compare parameter counts first
        if left.dynamic_parameters.len() != right.dynamic_parameters.len() {
            return false;
        }

        // compare parameter shapes bi-directionally
        for (left_ty_id, right_ty_id) in left
            .dynamic_parameters
            .iter()
            .zip(right.dynamic_parameters.iter())
        {
            let left_assignable = self.is_type_assignable(
                module,
                profile,
                symbols,
                *left_ty_id,
                *right_ty_id,
                types,
                options,
            );
            let right_assignable = self.is_type_assignable(
                module,
                profile,
                symbols,
                *right_ty_id,
                *left_ty_id,
                types,
                options,
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
                    module,
                    profile,
                    symbols,
                    left_return,
                    right_return,
                    types,
                    options,
                );
                let right_assignable = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    right_return,
                    left_return,
                    types,
                    options,
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
        module: &Module,
        dynamic_arguments: &[LocalNodeId<Argument>],
        parameter_types: &[LocalTypeId],
        profile: ProfileId,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // infer arguments left to right, adding constraints eagerly so later contextual typing
        let mut argument_ty_ids = Vec::with_capacity(dynamic_arguments.len());
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            let parameter_ty_id = parameter_types.get(index).copied();

            // materialize contextual expectation from the current inferred state
            let expected_arg_ty_id = parameter_ty_id.and_then(|parameter_ty_id| {
                self.expected_parameter_type_for_inference(
                    module,
                    profile,
                    parameter_ty_id,
                    symbols,
                    types,
                )
            });
            self.infer_argument(
                module,
                *argument_id,
                expected_arg_ty_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            // resolve inferred argument type
            let argument_value_id = tree.get(*argument_id).value();
            let argument_ty_id = if let Some(argument_ty_id) =
                types.get_inferred_type_id(argument_value_id.into_global_any(module.id))
            {
                argument_ty_id
            } else {
                self.infer_expression(module, argument_value_id, tree, symbols, types, infer, ctx)?
            };
            argument_ty_ids.push(argument_ty_id);

            // add one argument parameter constraint immediately
            let Some(parameter_ty_id) = parameter_ty_id else {
                continue;
            };
            self.add_invocation_argument_constraint(
                module,
                *argument_id,
                argument_ty_id,
                parameter_ty_id,
                bound_substitutions,
                options,
                tree,
                symbols,
                types,
                infer,
                ctx,
            );
        }

        // capture template literal inference constraints
        self.add_template_literal_inference_constraints(
            module,
            ctx.profile,
            dynamic_arguments,
            &argument_ty_ids,
            parameter_types,
            tree,
            symbols,
            types,
            infer,
            options,
        );

        Ok(argument_ty_ids)
    }

    /// Select an expected parameter type for inference without widening type parameters.
    fn expected_parameter_type_for_inference(
        &self,
        module: &Module,
        profile: ProfileId,
        parameter_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // avoid contextual typing for type parameters
        let expected_ty_id = self.expected_value_type(Some(parameter_ty_id), types)?;
        if let Type::Reference {
            symbol,
            static_arguments: None,
        } = types.get_type(expected_ty_id)
            && self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
        {
            return None;
        }
        Some(expected_ty_id)
    }

    /// Add one argument constraint and static parameter bound for an invocation.
    #[allow(clippy::too_many_arguments)]
    fn add_invocation_argument_constraint(
        &self,
        module: &Module,
        argument_id: LocalNodeId<Argument>,
        argument_ty_id: LocalTypeId,
        parameter_ty_id: LocalTypeId,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &InferContext,
    ) {
        // connect argument type to parameter type
        infer.push_constraint(Constraint::Subtype {
            sub_type: argument_ty_id,
            super_type: parameter_ty_id,
            variance: None,
        });

        // enforce static parameter bounds for inferred type parameters
        let Some(parameter_symbol) =
            self.parameter_symbol_for_argument_constraint(types, infer, parameter_ty_id)
        else {
            return;
        };

        // skip non-static parameters
        if !self.symbol_is_static_parameter(module, ctx.profile, parameter_symbol, symbols, types) {
            return;
        }

        // resolve the static parameter constraint
        let argument_value_id = tree.get(argument_id).value();
        let constraint_id = self.static_parameter_constraint_type(
            module,
            ctx.profile,
            parameter_symbol,
            argument_value_id.into_any(),
            symbols,
            types,
        );
        let Some(constraint_id) = constraint_id else {
            return;
        };

        // apply inherited substitutions to static constraints when needed
        let constraint_id = if let Some(bound_substitutions) = bound_substitutions
            && !bound_substitutions.is_empty()
        {
            let mut cache = HashMap::new();
            self.substitute_static_parameters(constraint_id, bound_substitutions, types, &mut cache)
        } else {
            constraint_id
        };
        if matches!(
            types.get_type(constraint_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown | TypeLiteral::Any
            }
        ) {
            return;
        }

        // emit a constraint violation error when needed
        if self.is_type_assignable(
            module,
            ctx.profile,
            symbols,
            constraint_id,
            argument_ty_id,
            types,
            options,
        ) == Assignability::NotAssignable
        {
            self.emit_unassignable_type_for_types(
                module,
                ctx.profile,
                argument_value_id.into_any(),
                constraint_id,
                argument_ty_id,
                types,
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
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        argument_ty_ids: &[LocalTypeId],
        parameter_types: &[LocalTypeId],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        // check argument assignability against parameters
        for (index, (argument_ty_id, param_ty_id)) in argument_ty_ids
            .iter()
            .zip(parameter_types.iter())
            .enumerate()
        {
            // enforce explicit ownership when implicit managed values are disabled
            if let Some(argument) = dynamic_arguments.get(index) {
                let argument_value = tree.get(*argument).value();
                self.check_no_implicit_managed_value(
                    module,
                    profile,
                    argument_value,
                    *param_ty_id,
                    *argument_ty_id,
                    tree,
                    types,
                    options,
                );
            }

            if !self.is_type_assignable_or_deferred(
                module,
                profile,
                symbols,
                *param_ty_id,
                *argument_ty_id,
                types,
                options,
            ) {
                // allow static literal arguments to flow to literal parameter types
                if let Some(argument) = dynamic_arguments.get(index)
                    && let Some(literal_ty_id) = self.static_literal_type_from_argument(
                        module,
                        profile,
                        tree.get(*argument).value(),
                        tree,
                        symbols,
                        types,
                    )?
                    && self.is_type_assignable(
                        module,
                        profile,
                        symbols,
                        *param_ty_id,
                        literal_ty_id,
                        types,
                        options,
                    ) != Assignability::NotAssignable
                {
                    continue;
                }

                let argument_node = dynamic_arguments
                    .get(index)
                    .map(|id| id.into_any())
                    .unwrap_or_else(|| expression_id.into_any());
                if let Some(error) = self.unassignable_type_error_for_types(
                    module,
                    profile,
                    argument_node,
                    *param_ty_id,
                    *argument_ty_id,
                    types,
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
        module: &Module,
        profile: ProfileId,
        resolved: &ResolvedSignature,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<bool> {
        if dynamic_arguments.len() > resolved.dynamic_parameters.len() {
            return Ok(false);
        }

        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            let Some(param_ty_id) = resolved.dynamic_parameters.get(index).copied() else {
                return Ok(false);
            };

            if !self.is_signature_argument_applicable(
                module,
                profile,
                *argument_id,
                param_ty_id,
                tree,
                symbols,
                types,
                options,
            )? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if one call argument is applicable to a signature parameter.
    #[allow(clippy::too_many_arguments)]
    fn is_signature_argument_applicable(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        param_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<bool> {
        let argument = tree.get(argument_id);
        if matches!(argument, Argument::Spread { .. }) {
            return Ok(true);
        }

        let argument_value_id = argument.value();
        self.ensure_reference_instance_types_for_type(
            module,
            profile,
            argument_value_id.into_any(),
            param_ty_id,
            types,
        )?;

        if !self.is_signature_lambda_argument_applicable(
            param_ty_id,
            argument_value_id,
            tree,
            types,
        ) {
            return Ok(false);
        }

        if !self.is_signature_scalar_literal_argument_applicable(
            module,
            profile,
            param_ty_id,
            argument_value_id,
            tree,
            symbols,
            types,
            options,
        ) {
            return Ok(false);
        }

        if !self.is_signature_static_literal_argument_applicable(
            module,
            profile,
            param_ty_id,
            argument_value_id,
            tree,
            symbols,
            types,
            options,
        )? {
            return Ok(false);
        }

        Ok(self.is_signature_known_argument_type_applicable(
            module,
            profile,
            param_ty_id,
            argument_value_id,
            tree,
            symbols,
            types,
            options,
        ))
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
        if let Expression::Declaration { declaration } = argument_value
            && let Declaration::Function { signature, .. } = tree.get(*declaration)
            && matches!(signature.kind, FunctionKind::Lambda)
        {
            let param_signatures = self.call_signatures_for_type(param_ty_id, types);
            let Some(signature_id) = param_signatures.first().copied() else {
                return true;
            };
            let Type::Function {
                dynamic_parameters,
                return_type,
                ..
            } = types.get_type(signature_id)
            else {
                return true;
            };
            let expected_params = dynamic_parameters.as_slice();
            let expected_return = *return_type;

            if signature.dynamic_parameters.len() > expected_params.len() {
                return false;
            }

            let expects_predicate = expected_return.is_some_and(|return_ty_id| {
                matches!(types.get_type(return_ty_id), Type::Predicate { .. })
            });
            if expects_predicate {
                let has_predicate_return = signature.return_type.is_some_and(|return_id| {
                    matches!(tree.get(return_id), Expression::TypePredicate { .. })
                });
                if !has_predicate_return {
                    return false;
                }
            }
        }

        true
    }

    /// Check scalar literal assignability for signature applicability.
    #[allow(clippy::too_many_arguments)]
    fn is_signature_scalar_literal_argument_applicable(
        &self,
        module: &Module,
        profile: ProfileId,
        param_ty_id: LocalTypeId,
        argument_value_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let argument_value = tree.get(argument_value_id);
        let Expression::ScalarLiteral { value } = argument_value else {
            return true;
        };

        let literal_ty = self.infer_scalar_literal(value);
        let ty = Type::TypeLiteral { value: literal_ty };
        let literal_ty_id = types.insert_type_from_any(ty, argument_value_id.into_any());

        self.is_signature_candidate_argument_assignable(
            module,
            profile,
            symbols,
            param_ty_id,
            literal_ty_id,
            types,
            options,
        )
    }

    /// Check static literal assignability for signature applicability.
    #[allow(clippy::too_many_arguments)]
    fn is_signature_static_literal_argument_applicable(
        &self,
        module: &Module,
        profile: ProfileId,
        param_ty_id: LocalTypeId,
        argument_value_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<bool> {
        let Some(literal_ty_id) = self.static_literal_type_from_argument(
            module,
            profile,
            argument_value_id,
            tree,
            symbols,
            types,
        )?
        else {
            return Ok(true);
        };

        Ok(self.is_signature_candidate_argument_assignable(
            module,
            profile,
            symbols,
            param_ty_id,
            literal_ty_id,
            types,
            options,
        ))
    }

    /// Check known argument-type assignability for signature applicability.
    #[allow(clippy::too_many_arguments)]
    fn is_signature_known_argument_type_applicable(
        &self,
        module: &Module,
        profile: ProfileId,
        param_ty_id: LocalTypeId,
        argument_value_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let argument_value = tree.get(argument_value_id);
        let argument_ty_id = types
            .get_inferred_type_id(argument_value_id.into_global_any(module.id))
            .or_else(|| {
                argument_value
                    .target_symbol()
                    .and_then(|symbol| types.get_type_id_for_symbol(symbols, symbol))
            });
        let Some(argument_ty_id) = argument_ty_id else {
            return true;
        };

        self.is_signature_candidate_argument_assignable(
            module,
            profile,
            symbols,
            param_ty_id,
            argument_ty_id,
            types,
            options,
        )
    }

    /// Resolve a scalar literal type for static argument expressions when possible.
    fn static_literal_type_from_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_value_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // evaluate to a scalar literal when possible
        let value = self.evaluate_static_expression_value(
            module,
            profile,
            argument_value_id,
            tree,
            symbols,
            types,
            None,
        )?;
        let Some(StaticExpression::ScalarLiteral { value }) = value else {
            return Ok(None);
        };

        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(value),
        };
        Ok(Some(
            types.insert_type_from_any(ty, argument_value_id.into_any()),
        ))
    }

    /// Prepare member-call typing context shared by member call resolution paths.
    #[allow(clippy::too_many_arguments)]
    fn resolve_member_call_type_context(
        &self,
        module: &Module,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty_id: Option<LocalTypeId>,
        receiver_ty: &Type,
        member_key: &StaticKey,
        member_symbol: Option<GlobalSymbolId>,
        lookup_mode: MemberLookupMode,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<ResolvedMemberCallTypeContext> {
        // inherit static arguments and substitutions from the receiver
        let mut inherited = self.resolve_inherited_static_arguments(
            module,
            profile,
            receiver_expression_id.into_any(),
            receiver_ty_id,
            receiver_ty,
            infer,
            options,
            tree,
            symbols,
            types,
        )?;
        if let Some(member_symbol) = member_symbol {
            self.extend_owner_substitutions_from_inherited_arguments(
                module,
                profile,
                receiver_expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
                &mut inherited.substitutions,
                tree,
                symbols,
                types,
            );
        }

        // resolve extension substitutions for member symbols
        let extension_context = if let Some(member_symbol) = member_symbol {
            self.resolve_extension_member_context(
                module,
                profile,
                receiver_expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
                options,
                tree,
                symbols,
                types,
            )?
        } else {
            None
        };

        // merge inherited and extension substitutions
        let substitutions = self.merge_member_substitutions(&inherited, extension_context.as_ref());

        // select instance arguments for member instancing
        let instance_arguments = self.infer_member_instance_base_arguments(
            &inherited.arguments,
            extension_context
                .as_ref()
                .map(|context| context.arguments.as_slice()),
        );

        // infer member type for this receiver
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            profile,
            receiver_expression_id.into_any(),
            symbols,
            receiver_ty,
            member_key,
            lookup_mode,
            types,
            &mut member_type_visited,
        )?;

        // apply static substitutions before call-signature resolution
        let member_ty_id = member_ty_id.map(|member_ty_id| {
            if substitutions.is_empty() {
                member_ty_id
            } else {
                let mut cache = HashMap::new();
                self.substitute_static_parameters(member_ty_id, &substitutions, types, &mut cache)
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
    #[allow(clippy::too_many_arguments)]
    fn resolve_union_member_call_candidates(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_union_ty_id: LocalTypeId,
        element_ids: &[LocalTypeId],
        member_key: &StaticKey,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<Vec<UnionMemberCallCandidate>>> {
        // query receiver context used by per element lookup filtering
        let receiver_union_ty = types.get_type(receiver_union_ty_id).clone();
        let receiver_context = self.query_member_receiver_context_for_expression(
            module,
            receiver_expression_id,
            Some(receiver_union_ty_id),
            &receiver_union_ty,
            profile,
            tree,
            symbols,
            types,
        );
        let context = UnionMemberCallResolutionContext {
            module,
            expression_id,
            receiver_expression_id,
            receiver_union_ty_id,
            receiver_nominal_symbol: receiver_context.nominal_symbol,
            member_key,
            static_arguments,
            dynamic_arguments,
            profile,
            options,
            tree,
            symbols,
        };

        // resolve one candidate per union element
        let mut candidates = Vec::new();
        for element_id in element_ids {
            let Some(candidate) = self.resolve_union_member_call_candidate_for_element(
                &context,
                *element_id,
                types,
                infer,
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
        element_id: LocalTypeId,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<UnionMemberCallCandidate>> {
        // resolve the element type and target member symbol
        let element_ty = types.get_type(element_id).clone();
        let member_symbol =
            self.resolve_union_member_symbol_for_element(context, element_id, &element_ty, types)?;
        let Some(member_symbol) = member_symbol else {
            self.report_union_member_call_missing_member(context, types)?;
            return Ok(None);
        };

        // prepare member call typing state for this element
        let lookup_mode = self
            .query_member_lookup_mode_for_receiver(context.receiver_nominal_symbol, &element_ty);
        let resolved_context = self.resolve_member_call_type_context(
            context.module,
            context.receiver_expression_id,
            Some(element_id),
            &element_ty,
            context.member_key,
            Some(member_symbol),
            lookup_mode,
            context.profile,
            context.options,
            context.tree,
            context.symbols,
            infer,
            types,
        )?;
        let Some(member_ty_id) = resolved_context.member_ty_id else {
            self.report_union_member_call_missing_member(context, types)?;
            return Ok(None);
        };

        // resolve one callable signature for this element member
        let resolved = self.resolve_union_member_call_candidate_signature(
            context,
            element_id,
            member_symbol,
            member_ty_id,
            &resolved_context,
            types,
            infer,
        )?;
        let Some(resolved) = resolved else {
            return Ok(None);
        };
        let (signature_ty_id, resolved_signature) = resolved;

        let signature_parameter_symbols =
            self.query_signature_static_parameter_symbols(signature_ty_id, types);

        // compose canonical member substitution environment from receiver and signature substitutions
        let instance_environment = self.compose_member_instance_environment(
            context.module,
            context.profile,
            member_symbol,
            &resolved_context.instance_arguments,
            &resolved_context.substitutions,
            &resolved_signature.static_arguments,
            &signature_parameter_symbols,
            context.tree,
            context.symbols,
            types,
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
            context.module,
            element_ty,
            context.member_key,
            context.profile,
            context.tree,
            context.symbols,
            types,
            &mut visited,
            true,
        )?;
        if member_symbol.is_some() {
            return Ok(member_symbol);
        }

        // then fall back to instance owner lookup when available
        if let Some(instance_symbol) = types.symbol_for_instance_type(element_id) {
            member_symbol = self.resolve_member_symbol_for_symbol(
                context.module,
                instance_symbol,
                context.member_key,
                MemberLookupMode::Instance,
                context.profile,
                context.tree,
                context.symbols,
                types,
                &mut visited,
            )?;
        }

        Ok(member_symbol)
    }

    /// Resolve one callable signature for a union element member candidate.
    fn resolve_union_member_call_candidate_signature(
        &self,
        context: &UnionMemberCallResolutionContext<'_>,
        element_id: LocalTypeId,
        member_symbol: GlobalSymbolId,
        member_ty_id: LocalTypeId,
        resolved_context: &ResolvedMemberCallTypeContext,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<(LocalTypeId, ResolvedSignature)>> {
        // query callable signatures for the resolved member type
        let call_signatures = self.call_signatures_for_type(member_ty_id, types);
        if call_signatures.is_empty() {
            self.report_union_member_call_non_callable(context, types);
            return Ok(None);
        }

        // select the best overload when multiple signatures exist
        let bound_substitutions =
            (!resolved_context.substitutions.is_empty()).then_some(&resolved_context.substitutions);
        if call_signatures.len() > 1 {
            let selection = self.select_call_signature(
                context.module,
                context.expression_id,
                Some(member_symbol),
                context.static_arguments,
                resolved_context.extension_arguments.as_deref(),
                bound_substitutions,
                &call_signatures,
                context.dynamic_arguments,
                Some(element_id),
                SignatureResolutionMode::Synthesize,
                context.profile,
                context.options,
                context.tree,
                context.symbols,
                types,
                infer,
            )?;
            let Some((signature_ty_id, resolved)) = selection else {
                self.report_union_member_call_no_overload(context, types);
                return Ok(None);
            };

            return Ok(Some((signature_ty_id, resolved)));
        }

        // resolve the singleton signature directly
        let signature_ty_id = call_signatures[0];
        let resolved = self.resolve_call_signature(
            context.module,
            context.expression_id,
            Some(member_symbol),
            context.static_arguments,
            resolved_context.extension_arguments.as_deref(),
            bound_substitutions,
            Some(context.dynamic_arguments),
            signature_ty_id,
            Some(element_id),
            None,
            SignatureResolutionMode::Synthesize,
            false,
            context.profile,
            context.options,
            context.tree,
            context.symbols,
            types,
            infer,
        )?;
        let Some(resolved) = resolved else {
            self.report_union_member_call_non_callable(context, types);
            return Ok(None);
        };

        Ok(Some((signature_ty_id, resolved)))
    }

    /// Report a missing-member diagnostic for union member-call resolution.
    fn report_union_member_call_missing_member(
        &self,
        context: &UnionMemberCallResolutionContext<'_>,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // allow associated blockers only for projection receivers
        let allow_associated_contract_blocker = self
            .query_expression_is_projection_receiver_for_infer(
                context.module,
                context.profile,
                context.receiver_expression_id,
                context.tree,
                context.symbols,
                types,
            );

        self.report_missing_member_diagnostic_for_receiver_type(
            context.module,
            context.profile,
            context.expression_id,
            context.receiver_union_ty_id,
            *context.member_key,
            context.symbols,
            types,
            allow_associated_contract_blocker,
        )?;

        Ok(())
    }

    /// Report a no-overload diagnostic for union member-call resolution.
    fn report_union_member_call_no_overload(
        &self,
        context: &UnionMemberCallResolutionContext<'_>,
        types: &TypeTable,
    ) {
        self.emit_no_overload_for_receiver_type(
            context.module,
            context.profile,
            context.expression_id.into_any(),
            context.receiver_union_ty_id,
            types,
        );
    }

    /// Report a non-callable diagnostic for union member-call resolution.
    fn report_union_member_call_non_callable(
        &self,
        context: &UnionMemberCallResolutionContext<'_>,
        types: &TypeTable,
    ) {
        self.emit_non_callable_for_callee_type(
            context.module,
            context.profile,
            context.expression_id.into_any(),
            context.receiver_union_ty_id,
            types,
        );
    }

    /// Infer a call expression.
    pub(crate) fn infer_call_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_CALL);

        // enforce call restrictions from options
        self.validate_call_expression(
            module,
            expression_id,
            left_id,
            tree,
            symbols,
            &ctx.options,
            ctx.profile,
            false,
        );

        // query and normalize the callee state
        let callee = match self.infer_call_expression_callee(
            module,
            expression_id,
            left_id,
            tree,
            symbols,
            types,
            infer,
            ctx,
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
        let options = ctx.options;

        // ensure instance types for callable references
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            callee.callee_ty_id,
            types,
        )?;

        // resolve symbols, inherited substitutions, and member-call context
        let call = self.resolve_call_expression_target(
            module,
            expression_id,
            left_id,
            callee.callee_ty_id,
            static_arguments,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        // resolve effective static-argument sources
        let effective_static_arguments =
            self.query_call_signature_static_arguments(static_arguments, &call);
        let union_static_arguments =
            self.query_union_member_signature_static_arguments(static_arguments, &call);

        // handle union receiver member calls with dynamic resolution
        if let Some(union_return_type_id) = self.infer_union_member_call_expression(
            module,
            expression_id,
            call.member_call_context.as_ref(),
            union_static_arguments,
            dynamic_arguments,
            &options,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )? {
            return Ok(finish_result(union_return_type_id, types));
        }

        // query call signatures from the normalized call target
        let call_signatures = self.call_signatures_for_call_target(
            tree,
            left_id,
            callee.callee_ty_id,
            call.super_constructor_value_ty_id,
            types,
        );
        let ty_id = if call_signatures.is_empty() {
            self.infer_non_callable_call_expression(
                module,
                expression_id,
                callee.callee_ty_id,
                &call,
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?
        } else {
            // resolve one concrete signature for this call
            let signature_resolution = self.resolve_call_expression_signature(
                module,
                expression_id,
                callee.callee_ty_id,
                &call,
                &call_signatures,
                effective_static_arguments,
                dynamic_arguments,
                &options,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            // infer and commit from the selected signature when available
            match signature_resolution {
                CallExpressionSignatureResolution::Resolved {
                    signature_ty_id,
                    signature: resolved_signature,
                } => self.infer_resolved_call_expression(
                    module,
                    expression_id,
                    dynamic_arguments,
                    &call,
                    signature_ty_id,
                    resolved_signature,
                    &options,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?,
                CallExpressionSignatureResolution::IndeterminateType(type_id) => type_id,
            }
        };

        Ok(finish_result(ty_id, types))
    }

    /// Query call signatures from a normalized call target.
    fn call_signatures_for_call_target(
        &self,
        tree: &NodeTree,
        left_id: LocalNodeId<Expression>,
        callee_ty_id: LocalTypeId,
        super_constructor_value_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Vec<LocalTypeId> {
        if self.expression_is_super_reference_for_call(
            tree,
            self.unwrap_parenthesized_expression(left_id, tree),
        ) {
            if let Some(super_constructor_value_ty_id) = super_constructor_value_ty_id {
                return self.construct_signatures_for_type(super_constructor_value_ty_id, types);
            }

            return Vec::new();
        }

        self.call_signatures_for_type(callee_ty_id, types)
    }

    /// Resolve one signature for a call expression or default to unknown.
    #[allow(clippy::too_many_arguments)]
    fn resolve_call_expression_signature(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        callee_ty_id: LocalTypeId,
        call: &CallExpressionResolution,
        call_signatures: &[LocalTypeId],
        effective_static_arguments: Option<&[LocalNodeId<Argument>]>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<CallExpressionSignatureResolution> {
        let bound_substitutions =
            (!call.inherited_substitutions.is_empty()).then_some(&call.inherited_substitutions);
        self.resolve_invocation_signature(
            module,
            expression_id,
            callee_ty_id,
            call.callee_symbol,
            effective_static_arguments,
            call.prefilled_static_arguments.as_deref(),
            bound_substitutions,
            call_signatures,
            dynamic_arguments,
            call.call_receiver_ty_id,
            ctx.expected_type,
            options,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )
    }

    /// Infer argument constraints, commit call resolution facts, and return the call result type.
    #[allow(clippy::too_many_arguments)]
    fn infer_resolved_call_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        call: &CallExpressionResolution,
        signature_ty_id: LocalTypeId,
        resolved_signature: ResolvedSignature,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // apply inherited substitutions from the receiver context
        let resolved_signature = self.apply_inherited_call_substitutions(
            module,
            ctx.profile,
            resolved_signature,
            &call.inherited_substitutions,
            tree,
            symbols,
            types,
        );
        let resolved_dynamic_parameters = &resolved_signature.dynamic_parameters;

        // infer argument types and constraints
        let argument_ty_ids = self.infer_invocation_arguments(
            module,
            dynamic_arguments,
            resolved_dynamic_parameters,
            ctx.profile,
            (!call.inherited_substitutions.is_empty()).then_some(&call.inherited_substitutions),
            options,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        // check argument assignability against parameters
        self.check_invocation_assignability(
            module,
            ctx.profile,
            expression_id,
            dynamic_arguments,
            &argument_ty_ids,
            resolved_dynamic_parameters,
            tree,
            symbols,
            types,
            options,
        )?;

        let resolved_return_type = resolved_signature.return_type;

        // commit static or member resolution for downstream lowering
        self.commit_call_expression_resolution(
            module,
            ctx.profile,
            expression_id,
            call,
            signature_ty_id,
            resolved_signature,
            tree,
            symbols,
            infer,
            types,
        )?;

        Ok(resolved_return_type.unwrap_or_else(|| {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            types.insert_type_from(ty, expression_id)
        }))
    }

    /// Apply inherited substitutions to a resolved signature.
    fn apply_inherited_call_substitutions(
        &self,
        module: &Module,
        profile: ProfileId,
        resolved_signature: ResolvedSignature,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> ResolvedSignature {
        if substitutions.is_empty() {
            return resolved_signature;
        }

        let mut materialize_cache = HashMap::new();
        let mut substitute_cache = HashMap::new();
        let dynamic_parameters = resolved_signature
            .dynamic_parameters
            .iter()
            .map(|parameter| {
                let materialized = self.materialize_static_arguments_in_type(
                    module,
                    profile,
                    *parameter,
                    tree,
                    symbols,
                    types,
                    &mut materialize_cache,
                );
                self.substitute_static_parameters(
                    materialized,
                    substitutions,
                    types,
                    &mut substitute_cache,
                )
            })
            .collect();
        let return_type = resolved_signature.return_type.map(|return_type| {
            let materialized = self.materialize_static_arguments_in_type(
                module,
                profile,
                return_type,
                tree,
                symbols,
                types,
                &mut materialize_cache,
            );
            self.substitute_static_parameters(
                materialized,
                substitutions,
                types,
                &mut substitute_cache,
            )
        });

        ResolvedSignature {
            dynamic_parameters,
            return_type,
            static_arguments: resolved_signature.static_arguments,
        }
    }

    /// Commit call-resolution facts after successful signature inference.
    fn commit_call_expression_resolution(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        call: &CallExpressionResolution,
        signature_ty_id: LocalTypeId,
        resolved_signature: ResolvedSignature,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // register the instance when the call resolves to a symbol
        let call_instance_id = if let Some(callee_symbol) = call.callee_symbol {
            if call.call_member_resolution.is_none() {
                let signature_parameter_symbols =
                    self.query_signature_static_parameter_symbols(signature_ty_id, types);
                let environment = StaticSubstitutionEnvironment::from_parameter_symbols(
                    resolved_signature.static_arguments.clone(),
                    signature_parameter_symbols,
                    0,
                )
                .or_else(|| {
                    self.instance_environment_for_symbol_arguments(
                        module,
                        profile,
                        callee_symbol,
                        resolved_signature.static_arguments.clone(),
                        0,
                        tree,
                        symbols,
                        types,
                    )
                });
                if let Some(environment) = environment {
                    self.commit_instance_for_node_maybe(
                        expression_id.into_global_any(module.id),
                        callee_symbol,
                        environment,
                        infer,
                        types,
                    )?
                } else {
                    None
                }
            } else {
                let signature_parameter_symbols =
                    self.query_signature_static_parameter_symbols(signature_ty_id, types);
                let base_instance_arguments = call
                    .member_instance_arguments
                    .as_deref()
                    .or(call.prefilled_static_arguments.as_deref())
                    .unwrap_or(call.inherited_static_arguments.as_slice());
                let environment = self.compose_member_instance_environment(
                    module,
                    profile,
                    callee_symbol,
                    base_instance_arguments,
                    &call.inherited_substitutions,
                    &resolved_signature.static_arguments,
                    &signature_parameter_symbols,
                    tree,
                    symbols,
                    types,
                );
                if let Some(environment) = environment {
                    self.commit_instance_for_node_maybe(
                        expression_id.into_global_any(module.id),
                        callee_symbol,
                        environment,
                        infer,
                        types,
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
                self.commit_member_resolution(
                    expression_id.into_global_any(module.id),
                    call.call_receiver_ty_id,
                    member_resolution,
                    call_instance_id,
                    Some(resolved_signature),
                    true,
                    types,
                );
            }
            (None, Some(callee_symbol)) => {
                self.commit_static_resolution(
                    expression_id.into_global_any(module.id),
                    call.call_receiver_ty_id,
                    callee_symbol,
                    call_instance_id,
                    Some(resolved_signature),
                    types,
                );
            }
            _ => {}
        }

        Ok(())
    }

    /// Infer a non-callable call expression and default to unknown.
    #[allow(clippy::too_many_arguments)]
    fn infer_non_callable_call_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        callee_ty_id: LocalTypeId,
        call: &CallExpressionResolution,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // suppress secondary diagnostics when member lookup already failed
        let has_missing_member = matches!(
            call.call_member_resolution,
            Some(MemberResolution::None | MemberResolution::Unresolved)
        );
        let callee_has_primary_error = self.type_blocks_cascading_diagnostic(callee_ty_id, types);

        // report non-callable callee types unless they are dynamic placeholders
        let is_dynamic_callee = has_missing_member
            || callee_has_primary_error
            || matches!(
                types.get_type(callee_ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Any
                }
            )
            || types.get_type(callee_ty_id).is_infer();
        if !is_dynamic_callee {
            self.emit_non_callable_for_callee_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                callee_ty_id,
                types,
            );
        }

        // infer dynamic arguments without expected types
        self.infer_call_arguments_without_context(
            module,
            dynamic_arguments,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        if callee_has_primary_error {
            return Ok(types.insert_type_from(Type::Error, expression_id));
        }

        Ok(self.synthesize_indeterminate_call_result_type(expression_id, types))
    }

    /// Infer call arguments without contextual parameter types.
    #[allow(clippy::too_many_arguments)]
    fn infer_call_arguments_without_context(
        &self,
        module: &Module,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        for argument_id in dynamic_arguments {
            self.infer_argument(module, *argument_id, None, tree, symbols, types, infer, ctx)?;
        }

        Ok(())
    }

    /// Synthesize an indeterminate semantic result type for one call expression.
    fn synthesize_indeterminate_call_result_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        };

        types.insert_type_from(ty, expression_id)
    }

    /// Infer and normalize the callee state for call-expression inference.
    #[allow(clippy::too_many_arguments)]
    fn infer_call_expression_callee(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<CallExpressionCalleeQuery> {
        let is_optional_chain = self.is_optional_chain_call_target(tree, left_id);
        let callee_ty_id = match tree.get(left_id) {
            Expression::Maybe { left } => {
                self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?
            }
            _ => self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?,
        };

        if !is_optional_chain {
            return Ok(CallExpressionCalleeQuery::Callee(CallExpressionCallee {
                callee_ty_id,
                has_optional_nullish: false,
            }));
        }

        let (non_nullish, has_nullish) = self.strip_nullish_from_union(callee_ty_id, types);
        let Some(non_nullish) = non_nullish else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            };
            let type_id = types.insert_type_from(ty, expression_id);
            return Ok(CallExpressionCalleeQuery::EarlyType(type_id));
        };

        Ok(CallExpressionCalleeQuery::Callee(CallExpressionCallee {
            callee_ty_id: non_nullish,
            has_optional_nullish: has_nullish,
        }))
    }

    /// Resolve call-target metadata used for signature and instance resolution.
    #[allow(clippy::too_many_arguments)]
    fn resolve_call_expression_target(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        callee_ty_id: LocalTypeId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<CallExpressionResolution> {
        let call_has_static_arguments =
            static_arguments.is_some_and(|arguments| !arguments.is_empty());
        let unwrapped_left_id = self.unwrap_parenthesized_expression(left_id, tree);
        match tree.get(unwrapped_left_id) {
            Expression::Member {
                left: receiver_id,
                name,
                static_arguments: member_static_arguments,
                ..
            } => self.resolve_member_call_expression_target(
                module,
                expression_id,
                unwrapped_left_id,
                *receiver_id,
                *name,
                member_static_arguments.clone(),
                call_has_static_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            ),
            _ => self.resolve_non_member_call_expression_target(
                module,
                expression_id,
                unwrapped_left_id,
                callee_ty_id,
                call_has_static_arguments,
                tree,
                symbols,
                types,
                ctx,
            ),
        }
    }

    /// Resolve member-call target metadata for signature and instance resolution.
    #[allow(clippy::too_many_arguments)]
    fn resolve_member_call_expression_target(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        unwrapped_left_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        member_name: StringId,
        member_static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        call_has_static_arguments: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<CallExpressionResolution> {
        // query receiver type and receiver lookup context
        let options = ctx.options;
        let (receiver_ty_id, receiver_ty, receiver_context) = self
            .infer_member_call_receiver_state(
                module,
                receiver_id,
                ctx.profile,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

        // build member-call context and inherited static substitutions
        let member_key = StaticKey::Name(member_name);
        let member_call_context = Some(MemberCallContext {
            receiver_id,
            receiver_ty_id,
            member_key,
        });
        let (inherited_static_arguments, mut inherited_substitutions) = self
            .resolve_member_call_inherited_static_context(
                module,
                receiver_id,
                receiver_ty_id,
                &receiver_ty,
                ctx.profile,
                &options,
                tree,
                symbols,
                infer,
                types,
            )?;

        // resolve the member target for receiver and lookup mode
        let (member_resolution, member_symbol) = self.resolve_member_call_member_resolution(
            module,
            receiver_id,
            &receiver_ty,
            &receiver_context,
            &member_key,
            ctx.profile,
            tree,
            symbols,
            types,
        )?;
        if let Some(member_symbol) = member_symbol {
            self.extend_owner_substitutions_from_inherited_arguments(
                module,
                ctx.profile,
                receiver_id.into_any(),
                member_symbol,
                &inherited_static_arguments,
                &mut inherited_substitutions,
                tree,
                symbols,
                types,
            );
        }

        // query extension supplied static arguments for this member call
        let prefilled_static_arguments = self.resolve_member_call_prefilled_static_arguments(
            module,
            receiver_id,
            member_symbol,
            &inherited_static_arguments,
            ctx.profile,
            &options,
            tree,
            symbols,
            types,
        )?;

        // check static argument conflicts between call and member sites
        let member_has_static_arguments = member_static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty());
        let has_static_argument_conflict = self.check_call_static_argument_conflict(
            module,
            expression_id,
            ctx.profile,
            call_has_static_arguments,
            member_has_static_arguments,
        );

        // query receiver/member instance arguments when member syntax supplied them
        let member_instance_arguments = self.query_member_call_instance_arguments(
            module,
            unwrapped_left_id,
            member_symbol,
            call_has_static_arguments,
            member_has_static_arguments,
            infer,
            types,
        );
        Ok(CallExpressionResolution {
            callee_symbol: member_symbol,
            call_receiver_ty_id: Some(receiver_ty_id),
            call_member_resolution: Some(member_resolution),
            member_call_context,
            inherited_static_arguments,
            inherited_substitutions,
            member_instance_arguments,
            prefilled_static_arguments,
            super_constructor_value_ty_id: None,
            has_static_argument_conflict,
            call_has_static_arguments,
        })
    }

    /// Infer receiver type and receiver context for a member call target.
    #[allow(clippy::too_many_arguments)]
    fn infer_member_call_receiver_state(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<(LocalTypeId, Type, MemberReceiverContext)> {
        let receiver_ty_id = self.infer_member_call_receiver_type(
            module,
            receiver_id,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;
        let receiver_ty = types.get_type(receiver_ty_id).clone();
        let receiver_context = self.query_member_receiver_context_for_expression(
            module,
            receiver_id,
            Some(receiver_ty_id),
            &receiver_ty,
            profile,
            tree,
            symbols,
            types,
        );

        Ok((receiver_ty_id, receiver_ty, receiver_context))
    }

    /// Resolve inherited static arguments for a member call receiver.
    #[allow(clippy::too_many_arguments)]
    fn resolve_member_call_inherited_static_context(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<(Vec<StaticArgument>, HashMap<GlobalSymbolId, LocalTypeId>)> {
        let inherited = self.resolve_inherited_static_arguments(
            module,
            profile,
            receiver_id.into_any(),
            Some(receiver_ty_id),
            receiver_ty,
            infer,
            options,
            tree,
            symbols,
            types,
        )?;

        Ok((inherited.arguments, inherited.substitutions))
    }

    /// Resolve member lookup metadata for a member call target.
    #[allow(clippy::too_many_arguments)]
    fn resolve_member_call_member_resolution(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty: &Type,
        receiver_context: &MemberReceiverContext,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<(MemberResolution, Option<GlobalSymbolId>)> {
        let member_resolution = self.resolve_member_symbol_for_receiver(
            module,
            receiver_id,
            receiver_ty,
            receiver_context,
            member_key,
            profile,
            tree,
            symbols,
            types,
        )?;
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        Ok((member_resolution, member_symbol))
    }

    /// Resolve extension supplied static arguments for a member call target.
    #[allow(clippy::too_many_arguments)]
    fn resolve_member_call_prefilled_static_arguments(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        inherited_static_arguments: &[StaticArgument],
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        let Some(member_symbol) = member_symbol else {
            return Ok(None);
        };
        let context = self.resolve_extension_member_context(
            module,
            profile,
            receiver_id.into_any(),
            member_symbol,
            inherited_static_arguments,
            options,
            tree,
            symbols,
            types,
        )?;

        Ok(context.map(|context| context.arguments))
    }

    /// Query member instance arguments supplied through member static-argument syntax.
    fn query_member_call_instance_arguments(
        &self,
        module: &Module,
        member_expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        call_has_static_arguments: bool,
        member_has_static_arguments: bool,
        infer: &InferTable,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        if call_has_static_arguments || !member_has_static_arguments {
            return None;
        }

        self.query_member_instance_arguments_for_call(
            module,
            member_expression_id,
            member_symbol,
            infer,
            types,
        )
    }

    /// Resolve non-member call target metadata for signature and instance resolution.
    #[allow(clippy::too_many_arguments)]
    fn resolve_non_member_call_expression_target(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        unwrapped_left_id: LocalNodeId<Expression>,
        callee_ty_id: LocalTypeId,
        call_has_static_arguments: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<CallExpressionResolution> {
        let is_super_constructor_call =
            self.expression_is_super_reference_for_call(tree, unwrapped_left_id);
        if is_super_constructor_call {
            let mut super_constructor_value_ty_id = None;
            if let Some(super_symbol) = self.super_symbol_for_type(callee_ty_id, types) {
                super_constructor_value_ty_id = self.super_constructor_value_type_for_symbol(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    super_symbol,
                    types,
                )?;
            }

            let super_symbol = self.super_constructor_symbol_from_type(
                module,
                ctx.profile,
                callee_ty_id,
                tree,
                symbols,
                types,
            )?;
            return Ok(CallExpressionResolution {
                callee_symbol: super_symbol,
                call_receiver_ty_id: None,
                call_member_resolution: None,
                member_call_context: None,
                inherited_static_arguments: Vec::new(),
                inherited_substitutions: HashMap::new(),
                member_instance_arguments: None,
                prefilled_static_arguments: None,
                super_constructor_value_ty_id,
                has_static_argument_conflict: false,
                call_has_static_arguments,
            });
        }

        let callee_symbol = self.reference_symbol_for_expression(
            module,
            unwrapped_left_id,
            ctx.profile,
            tree,
            symbols,
        );
        Ok(CallExpressionResolution {
            callee_symbol,
            call_receiver_ty_id: None,
            call_member_resolution: None,
            member_call_context: None,
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
    #[allow(clippy::too_many_arguments)]
    fn infer_member_call_receiver_type(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Expression::Maybe { left } = tree.get(receiver_id) {
            let receiver_ty_id =
                self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
            let (non_nullish, _) = self.strip_nullish_from_union(receiver_ty_id, types);
            return Ok(non_nullish.unwrap_or(receiver_ty_id));
        }

        if let Some(receiver_ty_id) =
            types.get_inferred_type_id(receiver_id.into_global_any(module.id))
        {
            return Ok(receiver_ty_id);
        }

        self.infer_expression(module, receiver_id, tree, symbols, types, infer, ctx)
    }

    /// Report whether call and member static arguments conflict.
    fn check_call_static_argument_conflict(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        profile: ProfileId,
        call_has_static_arguments: bool,
        member_has_static_arguments: bool,
    ) -> bool {
        let has_static_argument_conflict = call_has_static_arguments && member_has_static_arguments;
        if has_static_argument_conflict {
            self.error(AnalyzeError::ConflictingStaticArguments {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
        }

        has_static_argument_conflict
    }

    /// Query effective static arguments for direct call-signature resolution.
    fn query_call_signature_static_arguments<'a>(
        &self,
        static_arguments: Option<&'a [LocalNodeId<Argument>]>,
        call: &'a CallExpressionResolution,
    ) -> Option<&'a [LocalNodeId<Argument>]> {
        if call.has_static_argument_conflict {
            return None;
        }

        if call.call_has_static_arguments {
            return static_arguments;
        }
        None
    }

    /// Query effective static arguments for union member-call resolution.
    fn query_union_member_signature_static_arguments<'a>(
        &self,
        static_arguments: Option<&'a [LocalNodeId<Argument>]>,
        call: &'a CallExpressionResolution,
    ) -> Option<&'a [LocalNodeId<Argument>]> {
        if call.has_static_argument_conflict {
            return None;
        }
        if call.call_has_static_arguments {
            return static_arguments;
        }
        None
    }

    /// Infer dynamic union member-call dispatch when the receiver is a union.
    #[allow(clippy::too_many_arguments)]
    fn infer_union_member_call_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_call_context: Option<&MemberCallContext>,
        union_static_arguments: Option<&[LocalNodeId<Argument>]>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let Some(context) = member_call_context else {
            return Ok(None);
        };
        let Some(element_ids) =
            self.query_union_member_call_element_types(context.receiver_ty_id, types)
        else {
            return Ok(None);
        };

        // resolve union candidates for the member call
        let candidates = self.resolve_union_member_call_candidates(
            module,
            expression_id,
            context.receiver_id,
            context.receiver_ty_id,
            &element_ids,
            &context.member_key,
            union_static_arguments,
            dynamic_arguments,
            ctx.profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?;
        let Some(candidates) = candidates else {
            return self.infer_unresolved_union_member_call_expression(
                module,
                expression_id,
                context.receiver_ty_id,
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            );
        };

        let argument_ty_ids = self.infer_union_member_call_argument_types(
            module,
            dynamic_arguments,
            &candidates,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;
        self.infer_union_member_call_argument_constraints(&candidates, &argument_ty_ids, infer);

        if !self.check_union_member_call_argument_assignability(
            module,
            ctx.profile,
            symbols,
            &argument_ty_ids,
            &candidates,
            types,
            options,
        ) {
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                context.receiver_ty_id,
                types,
            );

            return Ok(Some(
                self.synthesize_indeterminate_call_result_type(expression_id, types),
            ));
        }

        let return_type_id = self.infer_union_member_call_result_type(
            expression_id,
            context.receiver_ty_id,
            &candidates,
            types,
        );
        self.commit_union_member_call_resolution(
            module,
            expression_id,
            context.receiver_ty_id,
            candidates,
            infer,
            types,
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

    /// Infer unresolved union member-call candidate resolution as indeterminate.
    #[allow(clippy::too_many_arguments)]
    fn infer_unresolved_union_member_call_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        self.infer_call_arguments_without_context(
            module,
            dynamic_arguments,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        self.commit_unresolved_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            Vec::new(),
            Vec::new(),
            types,
        );

        Ok(Some(self.synthesize_indeterminate_call_result_type(
            expression_id,
            types,
        )))
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
                let Some(param_ty_id) = candidate.signature.dynamic_parameters.get(index).copied()
                else {
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
    #[allow(clippy::too_many_arguments)]
    fn infer_union_member_call_argument_types(
        &self,
        module: &Module,
        dynamic_arguments: &[LocalNodeId<Argument>],
        candidates: &[UnionMemberCallCandidate],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        let expected_argument_types = self
            .query_union_member_call_expected_argument_types(candidates, dynamic_arguments.len());

        let mut argument_ty_ids = Vec::with_capacity(dynamic_arguments.len());
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            let expected_arg_ty_id = expected_argument_types.get(index).copied().flatten();
            self.infer_argument(
                module,
                *argument_id,
                expected_arg_ty_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            let argument = tree.get(*argument_id);
            let argument_value_id = argument.value();
            let argument_ty_id = if let Some(ty_id) =
                types.get_inferred_type_id(argument_value_id.into_global_any(module.id))
            {
                ty_id
            } else {
                self.infer_expression(module, argument_value_id, tree, symbols, types, infer, ctx)?
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
                .zip(candidate.signature.dynamic_parameters.iter())
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
    #[allow(clippy::too_many_arguments)]
    fn check_union_member_call_argument_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        argument_ty_ids: &[LocalTypeId],
        candidates: &[UnionMemberCallCandidate],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        for candidate in candidates {
            for (argument_ty_id, param_ty_id) in argument_ty_ids
                .iter()
                .zip(candidate.signature.dynamic_parameters.iter())
            {
                if !self.is_signature_candidate_argument_assignable(
                    module,
                    profile,
                    symbols,
                    *param_ty_id,
                    *argument_ty_id,
                    types,
                    options,
                ) {
                    return false;
                }
            }
        }

        true
    }

    /// Check candidate argument assignability, deferring unresolved inference state during overload filtering.
    #[allow(clippy::too_many_arguments)]
    fn is_signature_candidate_argument_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        // fast path: direct infer vars defer to solve
        if self.is_type_assignable_or_deferred(
            module,
            profile,
            symbols,
            target_type_id,
            source_type_id,
            types,
            options,
        ) {
            return true;
        }

        // defer relation checks that depend on unresolved convergence state
        if self.type_requires_infer_convergence(module, profile, target_type_id, symbols, types) {
            return true;
        }
        if self.type_requires_infer_convergence(module, profile, source_type_id, symbols, types) {
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

    /// Commit dynamic resolution candidates for union member-call dispatch.
    fn commit_union_member_call_resolution(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        candidates: Vec<UnionMemberCallCandidate>,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let source_node_id = expression_id.into_global_any(module.id);
        let mut deferred_candidate_attachments = Vec::new();
        let mut resolution_candidates = Vec::with_capacity(candidates.len());
        for (candidate_index, candidate) in candidates.into_iter().enumerate() {
            let instance_id = if let Some(environment) = candidate.instance_environment {
                let (instance_id, obligation_id) = self
                    .commit_instance_for_symbol_maybe_with_obligation(
                        candidate.symbol,
                        environment,
                        infer,
                        types,
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

        let resolution_id = self.commit_dynamic_resolution(
            source_node_id,
            Some(receiver_ty_id),
            resolution_candidates,
            types,
        );
        for (candidate_index, obligation_id) in deferred_candidate_attachments {
            let candidate_slot = DynamicResolutionCandidateSlotId::new(candidate_index as u32);
            infer.push_instance_commit_obligation_for_resolution_candidate(
                resolution_id,
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
        module: &Module,
        profile: ProfileId,
        super_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve the base symbol from the inferred super type
        let Some(base_symbol) = self.super_symbol_for_type(super_ty_id, types) else {
            return Ok(None);
        };

        self.explicit_constructor_symbol_for_class(module, profile, base_symbol, tree, symbols)
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
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        super_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // reuse local value types when they are already available
        if let Some(value_ty_id) = types.get_value_type_id(super_symbol) {
            return Ok(Some(value_ty_id));
        }

        // resolve local symbols from the current module table
        if super_symbol.module_id == module.id {
            return Ok(types.get_value_type_id(super_symbol));
        }

        // import remote value types on demand
        let value_ty_id = self.resolve_remote_symbol_value_type(
            module,
            profile,
            node_id,
            super_symbol,
            false,
            types,
        )?;

        Ok(Some(value_ty_id))
    }

    /// Resolve an explicit constructor method symbol for a class.
    fn explicit_constructor_symbol_for_class(
        &self,
        module: &Module,
        profile: ProfileId,
        class_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        self.with_module_tree_symbols_or_local_at_stage(
            module,
            profile,
            class_symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                // resolve the nominal declaration for the class symbol
                let class_entry = owner_symbols.get_symbol(class_symbol.local_id);
                let declaration_id = class_entry.primary_declaration?.local_id;
                if declaration_id.ty != NodeType::Declaration {
                    return None;
                }

                // resolve class members and locate the explicit constructor method
                let declaration_id = declaration_id.into_typed::<Declaration>();
                let Declaration::Class { members, .. } = owner_tree.get(declaration_id) else {
                    return None;
                };

                for member_id in members {
                    let member = owner_tree.get(*member_id);
                    let Member::Method {
                        signature, symbol, ..
                    } = member
                    else {
                        continue;
                    };

                    if signature.mode == Some(FunctionMode::Constructor) {
                        return Some(symbol.into_global(owner_module.id));
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
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // enforce constructor restrictions from options
        self.validate_call_expression(
            module,
            expression_id,
            left_id,
            tree,
            symbols,
            &ctx.options,
            ctx.profile,
            true,
        );
        let options = ctx.options;

        // query and normalize the constructor target
        let target = self.infer_new_expression_target(
            module,
            expression_id,
            left_id,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        // resolve construct signatures for the callee type
        let construct_signatures = self.construct_signatures_for_type(target.callee_ty_id, types);
        let ty_id = if construct_signatures.is_empty() {
            self.infer_non_constructable_new_expression(
                module,
                expression_id,
                &target,
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?
        } else {
            // resolve one concrete constructor signature
            let signature_resolution = self.resolve_new_expression_signature(
                module,
                expression_id,
                &target,
                static_arguments,
                dynamic_arguments,
                &construct_signatures,
                &options,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            // infer and commit from the selected constructor signature
            match signature_resolution {
                CallExpressionSignatureResolution::Resolved {
                    signature_ty_id: _,
                    signature: resolved_signature,
                } => self.infer_resolved_new_expression(
                    module,
                    expression_id,
                    &target,
                    dynamic_arguments,
                    resolved_signature,
                    &options,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?,
                CallExpressionSignatureResolution::IndeterminateType(type_id) => type_id,
            }
        };

        // reject managed allocations when managed memory is disabled
        if ctx.options.no_managed
            && !ctx.is_explicit_ownership
            && matches!(module.source, ModuleSource::User)
            && self.type_contains_managed(module, ctx.profile, ty_id, types)
        {
            self.error(AnalyzeError::ManagedMemoryDisabled {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        Ok(ty_id)
    }

    /// Infer and normalize constructor target metadata for new-expression inference.
    #[allow(clippy::too_many_arguments)]
    fn infer_new_expression_target(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<NewExpressionResolution> {
        let callee_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;

        // ensure instance types for constructor references
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            callee_ty_id,
            types,
        )?;

        // resolve the constructor symbol when possible
        let callee_id = self.unwrap_parenthesized_expression(left_id, tree);
        let callee_symbol =
            self.reference_symbol_for_expression(module, callee_id, ctx.profile, tree, symbols);
        let struct_constructor_symbol = callee_symbol.map(|symbol| {
            self.canonical_symbol_id(
                module,
                symbols,
                ctx.profile,
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

    /// Resolve one constructor signature for a new expression or default to unknown.
    #[allow(clippy::too_many_arguments)]
    fn resolve_new_expression_signature(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        target: &NewExpressionResolution,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        construct_signatures: &[LocalTypeId],
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<CallExpressionSignatureResolution> {
        self.resolve_invocation_signature(
            module,
            expression_id,
            target.callee_ty_id,
            target.callee_symbol,
            static_arguments,
            None,
            None,
            construct_signatures,
            dynamic_arguments,
            None,
            ctx.expected_type,
            options,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )
    }

    /// Resolve one invocation signature for call or constructor expressions.
    #[allow(clippy::too_many_arguments)]
    fn resolve_invocation_signature(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id_for_error: LocalTypeId,
        callee_symbol: Option<GlobalSymbolId>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        prefilled_static_arguments: Option<&[StaticArgument]>,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        signature_ty_ids: &[LocalTypeId],
        dynamic_arguments: &[LocalNodeId<Argument>],
        receiver_ty_id_for_signature: Option<LocalTypeId>,
        expected_return_type: Option<LocalTypeId>,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<CallExpressionSignatureResolution> {
        // select the matching overload
        let selection = self.select_call_signature(
            module,
            expression_id,
            callee_symbol,
            static_arguments,
            prefilled_static_arguments,
            bound_substitutions,
            signature_ty_ids,
            dynamic_arguments,
            receiver_ty_id_for_signature,
            SignatureResolutionMode::Synthesize,
            ctx.profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?;
        if let Some((signature_ty_id, resolved)) = selection {
            return Ok(CallExpressionSignatureResolution::Resolved {
                signature_ty_id,
                signature: resolved,
            });
        }

        // report overload errors on ambiguous calls
        if signature_ty_ids.len() > 1 {
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                receiver_ty_id_for_error,
                types,
            );
            self.infer_call_arguments_without_context(
                module,
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            return Ok(CallExpressionSignatureResolution::IndeterminateType(
                self.synthesize_indeterminate_call_result_type(expression_id, types),
            ));
        }

        // attempt direct signature resolution for singleton signatures
        let signature_ty_id = signature_ty_ids[0];
        let resolved = self.resolve_call_signature(
            module,
            expression_id,
            callee_symbol,
            static_arguments,
            prefilled_static_arguments,
            bound_substitutions,
            Some(dynamic_arguments),
            signature_ty_id,
            receiver_ty_id_for_signature,
            expected_return_type,
            SignatureResolutionMode::Synthesize,
            false,
            ctx.profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?;
        if let Some(resolved) = resolved {
            Ok(CallExpressionSignatureResolution::Resolved {
                signature_ty_id,
                signature: resolved,
            })
        } else {
            Ok(CallExpressionSignatureResolution::IndeterminateType(
                self.synthesize_indeterminate_call_result_type(expression_id, types),
            ))
        }
    }

    /// Infer constructor arguments, commit constructor resolution, and return the constructed type.
    #[allow(clippy::too_many_arguments)]
    fn infer_resolved_new_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        target: &NewExpressionResolution,
        dynamic_arguments: &[LocalNodeId<Argument>],
        resolved_signature: ResolvedSignature,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let resolved_dynamic_parameters = &resolved_signature.dynamic_parameters;

        // enforce exact arity for struct constructors
        if target.is_struct_constructor
            && dynamic_arguments.len() != resolved_dynamic_parameters.len()
        {
            self.emit_no_overload_for_receiver_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                target.callee_ty_id,
                types,
            );
            self.infer_call_arguments_without_context(
                module,
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            return Ok(self.synthesize_indeterminate_call_result_type(expression_id, types));
        }

        // infer argument types and constraints
        let argument_ty_ids = self.infer_invocation_arguments(
            module,
            dynamic_arguments,
            resolved_dynamic_parameters,
            ctx.profile,
            None,
            options,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        // check argument assignability against parameters
        self.check_invocation_assignability(
            module,
            ctx.profile,
            expression_id,
            dynamic_arguments,
            &argument_ty_ids,
            resolved_dynamic_parameters,
            tree,
            symbols,
            types,
            options,
        )?;

        let resolved_return_type = resolved_signature.return_type;

        // register the constructor instance when static arguments were resolved
        let constructor_instance_id = if let Some(callee_symbol) = target.callee_symbol {
            let environment = self.instance_environment_for_symbol_arguments(
                module,
                ctx.profile,
                callee_symbol,
                resolved_signature.static_arguments.clone(),
                0,
                tree,
                symbols,
                types,
            );
            if let Some(environment) = environment {
                self.commit_instance_for_node_maybe(
                    expression_id.into_global_any(module.id),
                    callee_symbol,
                    environment,
                    infer,
                    types,
                )?
            } else {
                None
            }
        } else {
            None
        };

        // commit constructor resolution when possible
        if let Some(callee_symbol) = target.callee_symbol {
            self.commit_static_resolution(
                expression_id.into_global_any(module.id),
                None,
                callee_symbol,
                constructor_instance_id,
                Some(resolved_signature),
                types,
            );
        }

        Ok(resolved_return_type.unwrap_or_else(|| {
            self.synthesize_indeterminate_call_result_type(expression_id, types)
        }))
    }

    /// Infer behavior for non-constructable new-expression targets.
    #[allow(clippy::too_many_arguments)]
    fn infer_non_constructable_new_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        target: &NewExpressionResolution,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // reuse the instance type for class and struct constructors without signatures
        if let Some(callee_symbol) = target.callee_symbol
            && matches!(callee_symbol.ty(), SymbolType::Struct | SymbolType::Class)
            && let Some(instance_type_id) = types.get_instance_type_id(callee_symbol)
        {
            self.infer_call_arguments_without_context(
                module,
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            return Ok(instance_type_id);
        }

        // infer arguments and return unknown when no constructor target is available
        self.infer_call_arguments_without_context(
            module,
            dynamic_arguments,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        Ok(self.synthesize_indeterminate_call_result_type(expression_id, types))
    }

    /// Resolve a function type for a call, substituting static parameters when provided.
    pub(crate) fn resolve_function_signature(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        prefilled_static_arguments: Option<&[StaticArgument]>,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        dynamic_arguments: Option<&[LocalNodeId<Argument>]>,
        static_parameters: &[LocalTypeId],
        dynamic_parameters: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
        expected_return_type: Option<LocalTypeId>,
        mode: SignatureResolutionMode,
        allow_missing_value_arguments: bool,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<ResolvedSignature> {
        let resolved = self.resolve_function_static_arguments(
            module,
            node_id,
            owner_symbol,
            static_arguments,
            prefilled_static_arguments,
            bound_substitutions,
            dynamic_arguments,
            static_parameters,
            dynamic_parameters,
            return_type,
            expected_return_type,
            mode,
            allow_missing_value_arguments,
            profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?;

        if let Some(resolved) = resolved {
            return Ok(resolved);
        }

        Ok(ResolvedSignature {
            dynamic_parameters: dynamic_parameters.to_vec(),
            return_type,
            static_arguments: Vec::new(),
        })
    }

    /// Resolve static arguments and substitutions for a function type.
    pub(crate) fn resolve_function_static_arguments(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_argument_ids: Option<&[LocalNodeId<Argument>]>,
        prefilled_static_arguments: Option<&[StaticArgument]>,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        dynamic_argument_ids: Option<&[LocalNodeId<Argument>]>,
        static_parameters: &[LocalTypeId],
        dynamic_parameters: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
        expected_return_type: Option<LocalTypeId>,
        mode: SignatureResolutionMode,
        allow_missing_value_arguments: bool,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<ResolvedSignature>> {
        // handle fast paths when there are no static parameters
        let has_static_arguments = static_argument_ids.is_some_and(|args| !args.is_empty());

        if static_parameters.is_empty() && !has_static_arguments {
            return Ok(None);
        }

        if static_parameters.is_empty() {
            if let Some(argument_ids) = static_argument_ids {
                for argument_id in argument_ids {
                    self.error(AnalyzeError::InvalidStaticArgument {
                        node: argument_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        message: "too many static arguments".to_string(),
                    });
                }
            }

            return Ok(Some(ResolvedSignature {
                dynamic_parameters: dynamic_parameters.to_vec(),
                return_type,
                static_arguments: Vec::new(),
            }));
        }

        let static_parameters = self.collect_signature_static_parameters(
            module,
            node_id,
            static_parameters,
            profile,
            tree,
            symbols,
            types,
        );
        let assigned_arguments = self.assign_signature_static_arguments(
            module,
            node_id,
            static_argument_ids,
            prefilled_static_arguments,
            &static_parameters,
            bound_substitutions,
            profile,
            tree,
            symbols,
        );
        let Some(resolved_state) = self.resolve_signature_static_argument_state(
            module,
            node_id,
            owner_symbol,
            &static_parameters,
            assigned_arguments,
            bound_substitutions,
            dynamic_argument_ids,
            dynamic_parameters,
            return_type,
            expected_return_type,
            mode,
            allow_missing_value_arguments,
            profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?
        else {
            return Ok(None);
        };
        let (resolved_dynamic_parameters, resolved_return_type) = self
            .instantiate_signature_from_resolved_state(
                module,
                node_id,
                owner_symbol,
                dynamic_parameters,
                return_type,
                &resolved_state,
                profile,
                tree,
                symbols,
                types,
            );

        Ok(Some(ResolvedSignature {
            dynamic_parameters: resolved_dynamic_parameters,
            return_type: resolved_return_type,
            static_arguments: resolved_state.resolved_arguments,
        }))
    }

    /// Collect static parameter metadata for one signature.
    fn collect_signature_static_parameters(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        static_parameter_type_ids: &[LocalTypeId],
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Vec<StaticParameter> {
        let static_parameter_symbols =
            self.static_parameter_symbols_for_type_ids(static_parameter_type_ids, types);

        static_parameter_symbols
            .iter()
            .map(|symbol_id| {
                self.resolve_static_parameter(
                    module, *symbol_id, node_id, profile, tree, symbols, types,
                )
            })
            .collect::<Vec<_>>()
    }

    /// Assign call and prefilled static arguments to signature parameters.
    fn assign_signature_static_arguments(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        static_argument_ids: Option<&[LocalNodeId<Argument>]>,
        prefilled_static_arguments: Option<&[StaticArgument]>,
        static_parameters: &[StaticParameter],
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Vec<Option<StaticArgument>> {
        let argument_ids = static_argument_ids.unwrap_or(&[]);
        let argument_values = argument_ids
            .iter()
            .map(|argument_id| StaticArgument::Unevaluated {
                node: argument_id.into_global_any(module.id),
            })
            .collect::<Vec<_>>();
        let prefilled_arguments = prefilled_static_arguments.unwrap_or(&[]);
        let prefilled_count = prefilled_arguments.len().min(static_parameters.len());
        let mut assigned_arguments: Vec<Option<StaticArgument>> =
            vec![None; static_parameters.len()];

        // assign explicitly prefilled arguments first
        for (index, argument) in prefilled_arguments.iter().take(prefilled_count).enumerate() {
            assigned_arguments[index] = Some(argument.clone());
        }

        // reserve already bound substitution slots before assigning call-site arguments
        if let Some(bound_substitutions) = bound_substitutions {
            for (index, parameter) in static_parameters.iter().enumerate() {
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
        for (index, parameter) in static_parameters.iter().enumerate() {
            if assigned_arguments[index].is_some() {
                continue;
            }
            call_slot_indexes.push(index);
            parameters_for_call.push(parameter.clone());
        }
        let assigned_for_call = self.assign_static_argument_values(
            module,
            profile,
            node_id,
            &argument_values,
            &parameters_for_call,
            tree,
            symbols,
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
    #[allow(clippy::too_many_arguments)]
    fn resolve_signature_static_argument_state(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_parameters: &[StaticParameter],
        assigned_arguments: Vec<Option<StaticArgument>>,
        bound_substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        dynamic_argument_ids: Option<&[LocalNodeId<Argument>]>,
        dynamic_parameters: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
        expected_return_type: Option<LocalTypeId>,
        mode: SignatureResolutionMode,
        allow_missing_value_arguments: bool,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<ResolvedStaticArgumentState>> {
        // query expected return type mapping for type parameter backfill
        let expected_return_mapping = self.query_signature_expected_return_mapping(
            module,
            mode,
            return_type,
            expected_return_type,
            profile,
            options,
            tree,
            symbols,
            types,
        )?;

        // resolve one static argument slot at a time
        let mut substitutions = HashMap::new();
        let mut bound_substitutions = bound_substitutions.cloned().unwrap_or_default();
        let mut resolved_arguments = Vec::with_capacity(static_parameters.len());
        let mut has_missing_value_argument = false;
        let mut has_value_substitution = false;
        for (index, static_parameter) in static_parameters.iter().enumerate() {
            let assigned_argument = assigned_arguments.get(index).cloned().flatten();

            // query inferred value arguments from dynamic argument positions
            let inferred_argument = self.query_signature_inferred_static_argument(
                module,
                profile,
                static_parameter,
                assigned_argument.as_ref(),
                dynamic_argument_ids,
                dynamic_parameters,
                tree,
                symbols,
                types,
            )?;

            // query expected argument backfill from return type compatibility
            let expected_argument = self.query_signature_expected_static_argument(
                module,
                profile,
                static_parameter,
                assigned_argument.as_ref(),
                expected_return_mapping.as_ref(),
                tree,
                symbols,
                types,
            )?;

            // resolve one concrete static argument value for this slot
            let Some(mut resolved_argument) = self.resolve_signature_static_argument_value(
                module,
                node_id,
                owner_symbol,
                static_parameter,
                assigned_argument,
                inferred_argument,
                expected_argument,
                mode,
                allow_missing_value_arguments,
                profile,
                tree,
                symbols,
                types,
                infer,
                &mut has_missing_value_argument,
            )?
            else {
                return Ok(None);
            };

            // query the error-node anchor used by type-argument validation
            let error_node = self.query_signature_static_argument_error_node(
                module,
                node_id,
                static_parameter,
                Some(&resolved_argument),
            );

            // validate and collect substitution facts for this argument
            let substitution = self.resolve_signature_static_argument_substitution(
                module,
                profile,
                error_node,
                static_parameter,
                &resolved_argument,
                &bound_substitutions,
                tree,
                symbols,
                types,
                options,
                infer,
            )?;
            if let Some(substitution_ty_id) = substitution {
                substitutions.insert(static_parameter.symbol, substitution_ty_id);
                bound_substitutions.insert(static_parameter.symbol, substitution_ty_id);
                resolved_argument = self.normalize_signature_value_static_argument_substitution(
                    static_parameter,
                    substitution_ty_id,
                    resolved_argument,
                    &mut has_value_substitution,
                    types,
                );
            } else if static_parameter.kind == StaticParameterKind::Type
                && let StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } = &resolved_argument
            {
                substitutions.insert(static_parameter.symbol, *ty);
                bound_substitutions.insert(static_parameter.symbol, *ty);
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

    /// Query expected return mapping used to backfill unresolved type arguments.
    #[allow(clippy::too_many_arguments)]
    fn query_signature_expected_return_mapping(
        &self,
        module: &Module,
        mode: SignatureResolutionMode,
        return_type: Option<LocalTypeId>,
        expected_return_type: Option<LocalTypeId>,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<HashMap<GlobalSymbolId, StaticArgument>>> {
        // only infer from return types in inference mode
        if mode != SignatureResolutionMode::Synthesize {
            return Ok(None);
        }
        let Some(return_type) = return_type else {
            return Ok(None);
        };
        let Some(expected_return_type) = expected_return_type else {
            return Ok(None);
        };

        self.static_arguments_from_expected_return_type(
            module,
            profile,
            return_type,
            expected_return_type,
            options,
            tree,
            symbols,
            types,
        )
    }

    /// Query one inferred static argument from dynamic arguments when possible.
    #[allow(clippy::too_many_arguments)]
    fn query_signature_inferred_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        assigned_argument: Option<&StaticArgument>,
        dynamic_argument_ids: Option<&[LocalNodeId<Argument>]>,
        dynamic_parameters: &[LocalTypeId],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // explicit static arguments always win over inferred ones
        if assigned_argument.is_some() {
            return Ok(None);
        }
        let Some(dynamic_argument_ids) = dynamic_argument_ids else {
            return Ok(None);
        };

        self.infer_static_argument_from_dynamic_arguments(
            module,
            profile,
            static_parameter,
            dynamic_parameters,
            dynamic_argument_ids,
            tree,
            symbols,
            types,
        )
    }

    /// Query one expected static argument from expected return type mapping.
    #[allow(clippy::too_many_arguments)]
    fn query_signature_expected_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        assigned_argument: Option<&StaticArgument>,
        expected_return_mapping: Option<&HashMap<GlobalSymbolId, StaticArgument>>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // only unassigned type parameters can be inherited from expected return types
        if assigned_argument.is_some() || static_parameter.kind != StaticParameterKind::Type {
            return Ok(None);
        }
        let expected_argument = expected_return_mapping
            .and_then(|mapping| mapping.get(&static_parameter.symbol))
            .cloned();
        let Some(expected_argument) = expected_argument else {
            return Ok(None);
        };

        self.resolve_static_argument(
            module,
            profile,
            static_parameter,
            Some(expected_argument),
            true,
            tree,
            symbols,
            types,
        )
    }

    /// Query the error node for one static argument slot.
    fn query_signature_static_argument_error_node(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_argument: Option<&StaticArgument>,
    ) -> GlobalNodeIdAny {
        if let Some(argument) = resolved_argument {
            return match argument {
                StaticArgument::Unevaluated { node } => *node,
                StaticArgument::Evaluated { .. } => node_id.into_global(module.id),
            };
        }
        if let Some(default_expression) = static_parameter.default_expression.as_ref() {
            return default_expression
                .local_id
                .into_global_any(default_expression.module_id);
        }

        node_id.into_global(module.id)
    }

    /// Resolve one static argument value for signature instantiation.
    #[allow(clippy::too_many_arguments)]
    fn resolve_signature_static_argument_value(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_parameter: &StaticParameter,
        assigned_argument: Option<StaticArgument>,
        inferred_argument: Option<StaticArgument>,
        expected_argument: Option<StaticArgument>,
        mode: SignatureResolutionMode,
        allow_missing_value_arguments: bool,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        has_missing_value_argument: &mut bool,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // resolve the explicit argument or default first
        let resolved_argument = self.resolve_static_argument(
            module,
            profile,
            static_parameter,
            assigned_argument,
            true,
            tree,
            symbols,
            types,
        )?;
        if let Some(resolved_argument) = resolved_argument {
            return Ok(Some(resolved_argument));
        }

        // then use inference from dynamic arguments
        if let Some(inferred_argument) = inferred_argument {
            return Ok(Some(inferred_argument));
        }

        // then use expected return type backfill
        if let Some(expected_argument) = expected_argument {
            return Ok(Some(expected_argument));
        }

        // defer value argument failure when the caller allows missing value slots
        if static_parameter.kind == StaticParameterKind::Value
            && mode == SignatureResolutionMode::Synthesize
            && allow_missing_value_arguments
        {
            return Ok(None);
        }

        // record missing value slots to suppress cascading diagnostics
        if static_parameter.kind == StaticParameterKind::Value {
            *has_missing_value_argument = true;
        }
        let missing_argument = self.synthesize_missing_static_argument_for_function(
            module,
            profile,
            node_id,
            owner_symbol,
            static_parameter,
            infer,
            types,
        )?;

        Ok(Some(missing_argument))
    }

    /// Resolve and validate one static argument substitution.
    #[allow(clippy::too_many_arguments)]
    fn resolve_signature_static_argument_substitution(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_argument: &StaticArgument,
        bound_substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // materialize type arguments before validating bound compatibility
        let materialized_substitution = if static_parameter.kind == StaticParameterKind::Type {
            Some(self.materialize_static_type_argument(
                module,
                profile,
                error_node,
                static_parameter,
                resolved_argument,
                tree,
                symbols,
                types,
            )?)
        } else {
            None
        };

        self.validate_static_argument(
            module,
            profile,
            error_node,
            static_parameter,
            resolved_argument,
            materialized_substitution,
            bound_substitutions,
            tree,
            symbols,
            types,
            Some(infer),
            options,
        )
    }

    /// Normalize value static arguments when substitution produced a concrete type.
    fn normalize_signature_value_static_argument_substitution(
        &self,
        static_parameter: &StaticParameter,
        substitution_ty_id: LocalTypeId,
        resolved_argument: StaticArgument,
        has_value_substitution: &mut bool,
        types: &mut TypeTable,
    ) -> StaticArgument {
        if static_parameter.kind != StaticParameterKind::Value {
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
    #[allow(clippy::too_many_arguments)]
    fn instantiate_signature_from_resolved_state(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        dynamic_parameters: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
        state: &ResolvedStaticArgumentState,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> (Vec<LocalTypeId>, Option<LocalTypeId>) {
        // apply substitutions to the dynamic signature
        let mut materialize_cache = HashMap::new();
        let mut substitute_cache = HashMap::new();
        let resolved_dynamic_parameters = if state.has_missing_value_argument {
            let error_ty_id = types.insert_type_from_any(Type::Error, node_id);
            dynamic_parameters
                .iter()
                .map(|_| error_ty_id)
                .collect::<Vec<_>>()
        } else {
            dynamic_parameters
                .iter()
                .map(|parameter| {
                    self.instantiate_signature_type(
                        module,
                        profile,
                        node_id,
                        owner_symbol,
                        &state.substitutions,
                        *parameter,
                        tree,
                        symbols,
                        types,
                        &mut materialize_cache,
                        &mut substitute_cache,
                    )
                })
                .collect::<Vec<_>>()
        };
        let resolved_return_type = if state.has_missing_value_argument {
            Some(types.insert_type_from_any(Type::Error, node_id))
        } else {
            return_type.map(|return_type| {
                self.instantiate_signature_type(
                    module,
                    profile,
                    node_id,
                    owner_symbol,
                    &state.substitutions,
                    return_type,
                    tree,
                    symbols,
                    types,
                    &mut materialize_cache,
                    &mut substitute_cache,
                )
            })
        };

        // normalize substituted value arguments for downstream assignability
        if state.has_value_substitution {
            let mut normalize_cache = HashMap::new();
            let normalized_dynamic_parameters = resolved_dynamic_parameters
                .iter()
                .map(|parameter| {
                    self.materialize_static_arguments_in_type(
                        module,
                        profile,
                        *parameter,
                        tree,
                        symbols,
                        types,
                        &mut normalize_cache,
                    )
                })
                .collect::<Vec<_>>();
            let normalized_return_type = resolved_return_type.map(|return_type| {
                self.materialize_static_arguments_in_type(
                    module,
                    profile,
                    return_type,
                    tree,
                    symbols,
                    types,
                    &mut normalize_cache,
                )
            });
            (normalized_dynamic_parameters, normalized_return_type)
        } else {
            (resolved_dynamic_parameters, resolved_return_type)
        }
    }

    /// Resolve a member function for an operator invocation.
    ///
    /// Handles the common pattern of looking up a member on a receiver type,
    /// applying inherited substitutions, and resolving the function signature.
    pub(crate) fn resolve_member_function(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty_id: Option<LocalTypeId>,
        receiver_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<ResolvedMemberFunction>> {
        // resolve member dispatch for the receiver type
        let member_resolution = self.resolve_member_symbol(
            module,
            receiver_ty,
            member_key,
            profile,
            tree,
            symbols,
            types,
        )?;
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // decide how to filter member lookups for this receiver
        let receiver_context = self.query_member_receiver_context_for_expression(
            module,
            receiver_expression_id,
            receiver_ty_id,
            receiver_ty,
            profile,
            tree,
            symbols,
            types,
        );
        let lookup_mode = receiver_context.lookup_mode;

        // resolve member-call typing context for this receiver
        let resolved_context = self.resolve_member_call_type_context(
            module,
            receiver_expression_id,
            receiver_ty_id,
            receiver_ty,
            member_key,
            member_symbol,
            lookup_mode,
            profile,
            options,
            tree,
            symbols,
            infer,
            types,
        )?;
        let has_member = resolved_context.member_ty_id.is_some();

        // bail if we don't know the member type
        let Some(member_ty_id) = resolved_context.member_ty_id else {
            return Ok(Some(ResolvedMemberFunction {
                signature: ResolvedSignature {
                    dynamic_parameters: Vec::new(),
                    return_type: None,
                    static_arguments: Vec::new(),
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
            static_parameters,
            dynamic_parameters,
            return_type,
            ..
        } = types.get_type(member_ty_id).clone()
        else {
            return Ok(None);
        };
        let signature_static_parameter_symbols =
            self.static_parameter_symbols_for_type_ids(&static_parameters, types);
        let signature = self.resolve_function_signature(
            module,
            expression_id.into_any(),
            member_symbol,
            None,
            None,
            (!resolved_context.substitutions.is_empty()).then_some(&resolved_context.substitutions),
            None,
            &static_parameters,
            &dynamic_parameters,
            return_type,
            None,
            SignatureResolutionMode::Check,
            false,
            profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?;

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

    /// Commit the member-call instance id for a resolved member invocation.
    pub(crate) fn commit_member_call_instance_id(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        resolved: &ResolvedMemberFunction,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let Some(member_symbol) = resolved.member_symbol else {
            return Ok(None);
        };

        let environment = self.compose_member_instance_environment(
            module,
            profile,
            member_symbol,
            &resolved.instance_arguments,
            &resolved.bound_substitutions,
            &resolved.signature.static_arguments,
            &resolved.signature_static_parameter_symbols,
            tree,
            symbols,
            types,
        );
        let Some(environment) = environment else {
            return Ok(None);
        };

        self.commit_instance_for_node_maybe(
            expression_id.into_global_any(module.id),
            member_symbol,
            environment,
            infer,
            types,
        )
    }

    /// Commit resolution facts for a member function invocation.
    ///
    /// Returns the instance ID if one was committed.
    pub(crate) fn commit_member_call_resolution(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        resolved: &ResolvedMemberFunction,
        tree: &NodeTree,
        symbols: &SymbolTable,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalInstanceId>> {
        let instance_id = self.commit_member_call_instance_id(
            module,
            profile,
            expression_id,
            resolved,
            tree,
            symbols,
            infer,
            types,
        )?;

        // record member resolution
        self.commit_member_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            &resolved.member_resolution,
            instance_id,
            Some(resolved.signature.clone()),
            resolved.has_member,
            types,
        );

        Ok(instance_id)
    }
}
