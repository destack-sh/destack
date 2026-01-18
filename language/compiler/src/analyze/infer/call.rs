use std::collections::{HashMap, HashSet};

use super::member::{MemberLookupMode, MemberResolution};
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    Argument, Constraint, Declaration, DispatchKey, Expression, FunctionKind, GlobalSymbolId,
    InferTable, LocalInstanceId, LocalNodeId, LocalNodeIdAny, LocalTypeId, NodeTree,
    ResolutionCandidate, ResolvedSignature, StaticArgument, StaticKey, StaticParameterKind,
    SymbolTable, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ModuleSource, ProfileId};

/// Resolved member function for an operator invocation.
#[derive(Debug)]
pub(super) struct ResolvedMemberFunction {
    /// The resolved function signature (parameters and return type).
    pub(super) signature: ResolvedSignature,
    /// The member resolution for dispatch recording.
    pub(super) member_resolution: MemberResolution,
    /// The resolved member symbol (if statically known).
    pub(super) member_symbol: Option<GlobalSymbolId>,
    /// Static arguments used for the member instance.
    pub(super) instance_arguments: Vec<StaticArgument>,
    /// Whether the member was found on the receiver type.
    pub(super) has_member: bool,
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
    /// The static arguments for instancing this member.
    instance_arguments: Vec<StaticArgument>,
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
    /// Static arguments supplied on the member expression.
    member_static_arguments: Option<Vec<LocalNodeId<Argument>>>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect all callable signatures for a type.
    fn call_signatures_for_type(&self, ty_id: LocalTypeId, types: &TypeTable) -> Vec<LocalTypeId> {
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
    pub(super) fn call_signature_for_type(
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
    fn resolve_call_signature(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        callee_symbol: Option<GlobalSymbolId>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        signature_ty_id: LocalTypeId,
        call_receiver_ty_id: Option<LocalTypeId>,
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

        let resolved = self.resolve_function_signature(
            module,
            expression_id.into_any(),
            callee_symbol,
            static_arguments,
            &static_parameters,
            &dynamic_parameters,
            return_type,
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
        signature_ids: &[LocalTypeId],
        dynamic_arguments: &[LocalNodeId<Argument>],
        call_receiver_ty_id: Option<LocalTypeId>,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<(LocalTypeId, ResolvedSignature)>> {
        // only attempt selection when overloads exist
        if signature_ids.len() <= 1 {
            return Ok(None);
        }

        // resolve and filter applicable overloads
        let mut candidates = Vec::new();
        for signature_ty_id in signature_ids {
            let Some(resolved) = self.resolve_call_signature(
                module,
                expression_id,
                callee_symbol,
                static_arguments,
                *signature_ty_id,
                call_receiver_ty_id,
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

        if candidates.is_empty() {
            return Ok(None);
        }
        if candidates.len() == 1 {
            return Ok(Some(candidates.remove(0)));
        }

        // find the most specific signature among candidates
        let mut maximal = Vec::new();
        for (index, candidate) in candidates.iter().enumerate() {
            let mut dominated = false;
            for (other_index, other) in candidates.iter().enumerate() {
                if index == other_index {
                    continue;
                }
                if self.is_signature_more_specific(
                    module,
                    profile,
                    &other.1,
                    &candidate.1,
                    symbols,
                    types,
                    options,
                ) {
                    dominated = true;
                    break;
                }
            }
            if !dominated {
                maximal.push(index);
            }
        }

        if maximal.len() == 1 {
            let index = maximal[0];
            return Ok(Some(candidates.remove(index)));
        }

        let candidates = callee_symbol.into_iter().collect();
        Err(AnalyzeError::AmbiguousOverload {
            node: expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
            candidates,
        })
    }

    /// Infer argument types for a resolved signature and emit subtype constraints.
    fn infer_invocation_arguments(
        &self,
        module: &Module,
        dynamic_arguments: &[LocalNodeId<Argument>],
        parameter_types: &[LocalTypeId],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // infer arguments with contextual parameter types
        let mut argument_ty_ids = Vec::with_capacity(dynamic_arguments.len());
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            let expected_arg_ty_id = parameter_types.get(index).copied();
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

        // add constraints between arguments and parameters
        for (argument_ty_id, param_ty_id) in argument_ty_ids.iter().zip(parameter_types.iter()) {
            infer.push_constraint(Constraint::Subtype {
                sub_type: *argument_ty_id,
                super_type: *param_ty_id,
                variance: None,
            });
        }

        Ok(argument_ty_ids)
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

            if !self.is_infer_var_type(*param_ty_id, types)
                && !self.is_infer_var_type(*argument_ty_id, types)
                && self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *param_ty_id,
                    *argument_ty_id,
                    types,
                    options,
                ) == Assignability::NotAssignable
            {
                let argument_node = dynamic_arguments
                    .get(index)
                    .map(|id| id.into_global_any(module.id))
                    .unwrap_or_else(|| expression_id.into_global_any(module.id));
                return Err(AnalyzeError::UnassignableType {
                    node: argument_node.into_anchored(Some(profile)),
                    expected_ty: param_ty_id.into_global(module.id),
                    actual_ty: argument_ty_id.into_global(module.id),
                });
            }
        }

        Ok(())
    }

    /// Check if a resolved signature is applicable to the argument list.
    fn is_signature_applicable(
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

            let argument = tree.get(*argument_id);
            if matches!(argument, Argument::Spread { .. }) {
                continue;
            }
            let argument_value_id = argument.value();
            let argument_value = tree.get(argument_value_id);

            self.ensure_reference_instance_types_for_type(
                module,
                profile,
                argument_value_id.into_any(),
                param_ty_id,
                types,
            )?;

            if let Expression::Declaration { declaration } = argument_value
                && let Declaration::Function { signature, .. } = tree.get(*declaration)
                && matches!(signature.kind, FunctionKind::Lambda)
            {
                let param_signatures = self.call_signatures_for_type(param_ty_id, types);
                let Some(signature_id) = param_signatures.first().copied() else {
                    continue;
                };
                let Type::Function {
                    dynamic_parameters,
                    return_type,
                    ..
                } = types.get_type(signature_id)
                else {
                    continue;
                };
                let expected_params = dynamic_parameters.as_slice();
                let expected_return = *return_type;

                if signature.dynamic_parameters.len() > expected_params.len() {
                    return Ok(false);
                }

                let expects_predicate = expected_return.is_some_and(|return_ty_id| {
                    matches!(types.get_type(return_ty_id), Type::Predicate { .. })
                });
                if expects_predicate {
                    let has_predicate_return = signature.return_type.is_some_and(|return_id| {
                        matches!(tree.get(return_id), Expression::TypePredicate { .. })
                    });
                    if !has_predicate_return {
                        return Ok(false);
                    }
                }

                continue;
            }

            if let Expression::ScalarLiteral { value } = argument_value {
                let literal_ty = self.infer_scalar_literal(value);
                let ty = Type::TypeLiteral { value: literal_ty };
                let literal_ty_id = types.insert_type_from_any(ty, argument_value_id.into_any());
                if self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    param_ty_id,
                    literal_ty_id,
                    types,
                    options,
                ) == Assignability::NotAssignable
                {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Return true when the left signature is more specific than the right.
    fn is_signature_more_specific(
        &self,
        module: &Module,
        profile: ProfileId,
        left: &ResolvedSignature,
        right: &ResolvedSignature,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        if left.dynamic_parameters.len() != right.dynamic_parameters.len() {
            return false;
        }

        let mut is_strict = false;
        for (left_param, right_param) in left
            .dynamic_parameters
            .iter()
            .zip(right.dynamic_parameters.iter())
        {
            let left_to_right = self.is_type_assignable(
                module,
                profile,
                symbols,
                *right_param,
                *left_param,
                types,
                options,
            );
            if left_to_right == Assignability::NotAssignable {
                return false;
            }

            let right_to_left = self.is_type_assignable(
                module,
                profile,
                symbols,
                *left_param,
                *right_param,
                types,
                options,
            );
            if right_to_left == Assignability::NotAssignable {
                is_strict = true;
            }
        }

        is_strict
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
        // collect candidates across union variants
        let mut candidates = Vec::new();

        // resolve a candidate per union element
        for element_id in element_ids {
            let element_ty = types.get_type(*element_id).clone();

            // locate the member symbol for this union variant
            let mut visited = Vec::new();
            let mut member_symbol = self.resolve_member_symbol_for_type(
                module,
                &element_ty,
                member_key,
                profile,
                tree,
                symbols,
                types,
                &mut visited,
                true,
            )?;
            if member_symbol.is_none() {
                // fall back to instance type owners when possible
                if let Some(instance_symbol) = types.symbol_for_instance_type(*element_id) {
                    member_symbol = self.resolve_member_symbol_for_symbol(
                        module,
                        instance_symbol,
                        member_key,
                        MemberLookupMode::Instance,
                        profile,
                        tree,
                        symbols,
                        types,
                        &mut visited,
                    )?;
                }
            }
            let Some(member_symbol) = member_symbol else {
                self.error(AnalyzeError::MissingMember {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                    receiver_ty: receiver_union_ty_id.into_global(module.id),
                    member_key: *member_key,
                });
                return Ok(None);
            };

            // inherit static arguments and substitutions from the receiver
            let inherited = self.resolve_inherited_static_arguments(
                module,
                profile,
                receiver_expression_id.into_any(),
                &element_ty,
                options,
                tree,
                symbols,
                types,
            )?;

            // resolve extension substitutions for this member symbol
            let extension_context = self.resolve_extension_member_context(
                module,
                profile,
                receiver_expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
                options,
                tree,
                symbols,
                types,
            )?;

            // merge inherited and extension substitutions
            let mut substitutions = inherited.substitutions.clone();
            if let Some(context) = extension_context.as_ref() {
                for (symbol, ty_id) in &context.substitutions {
                    substitutions.insert(*symbol, *ty_id);
                }
            }

            // select instance arguments for member instancing
            let mut instance_arguments = match extension_context.as_ref() {
                Some(context) => context.arguments.clone(),
                None => inherited.arguments.clone(),
            };

            // decide how to filter member lookups for this receiver
            let lookup_mode = self.member_lookup_mode_for_receiver_expression(
                module,
                receiver_expression_id,
                &element_ty,
                profile,
                tree,
                symbols,
            );

            // resolve the member type for this variant
            let mut member_type_visited = Vec::new();
            let member_ty_id = self.infer_member_of_type(
                module,
                profile,
                receiver_expression_id.into_any(),
                &element_ty,
                member_key,
                lookup_mode,
                types,
                &mut member_type_visited,
            )?;
            let Some(member_ty_id) = member_ty_id else {
                self.error(AnalyzeError::MissingMember {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                    receiver_ty: receiver_union_ty_id.into_global(module.id),
                    member_key: *member_key,
                });
                return Ok(None);
            };

            // apply inherited substitutions before resolving call signatures
            let member_ty_id = if substitutions.is_empty() {
                member_ty_id
            } else {
                let mut cache = HashMap::new();
                self.substitute_static_parameters(member_ty_id, &substitutions, types, &mut cache)
            };

            // resolve call signatures for the member type
            let call_signatures = self.call_signatures_for_type(member_ty_id, types);
            let (_signature_ty_id, resolved) = if call_signatures.len() > 1 {
                let selection = self.select_call_signature(
                    module,
                    expression_id,
                    Some(member_symbol),
                    static_arguments,
                    &call_signatures,
                    dynamic_arguments,
                    Some(*element_id),
                    profile,
                    options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;
                let Some(selection) = selection else {
                    self.error(AnalyzeError::NoOverload {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        receiver_ty: receiver_union_ty_id.into_global(module.id),
                    });
                    return Ok(None);
                };
                selection
            } else if let Some(signature_ty_id) = call_signatures.first().copied() {
                let resolved = self.resolve_call_signature(
                    module,
                    expression_id,
                    Some(member_symbol),
                    static_arguments,
                    signature_ty_id,
                    Some(*element_id),
                    profile,
                    options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;
                let Some(resolved) = resolved else {
                    self.error(AnalyzeError::MissingType {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                    return Ok(None);
                };
                (signature_ty_id, resolved)
            } else {
                self.error(AnalyzeError::MissingType {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
                return Ok(None);
            };

            // extend instance arguments with resolved static arguments
            if !resolved.static_arguments.is_empty() {
                instance_arguments.extend(resolved.static_arguments.iter().cloned());
            }

            // keep the candidate signature for union dispatch
            candidates.push(UnionMemberCallCandidate {
                receiver_ty_id: *element_id,
                symbol: member_symbol,
                signature: resolved,
                instance_arguments,
            });
        }

        Ok(Some(candidates))
    }

    /// Infer a call expression.
    pub(super) fn infer_call_expression(
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

        let callee_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
        let options = ctx.options;

        // ensure instance types for callable references
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            callee_ty_id,
            types,
        )?;

        // track static arguments that come from member expressions
        let call_has_static_arguments =
            static_arguments.is_some_and(|arguments| !arguments.is_empty());
        let mut member_instance_arguments = None;
        let mut call_member_resolution = None;
        let mut call_receiver_ty_id = None;
        let mut member_call_context = None;
        let unwrapped_left_id = self.unwrap_parenthesized_expression(left_id, tree);

        // resolve inherited static arguments and the callee symbol
        let mut inherited_static_arguments = Vec::new();
        let mut inherited_substitutions = HashMap::new();
        let (callee_symbol, has_static_argument_conflict) = match tree.get(unwrapped_left_id) {
            Expression::Member {
                left: receiver_id,
                name,
                static_arguments: member_static_arguments,
                ..
            } => {
                let receiver_ty_id = if let Some(receiver_ty_id) =
                    types.get_inferred_type_id(receiver_id.into_global_any(module.id))
                {
                    receiver_ty_id
                } else {
                    self.infer_expression(module, *receiver_id, tree, symbols, types, infer, ctx)?
                };
                let receiver_ty = types.get_type(receiver_ty_id).clone();
                call_receiver_ty_id = Some(receiver_ty_id);

                member_call_context = Some(MemberCallContext {
                    receiver_id: *receiver_id,
                    receiver_ty_id,
                    member_key: StaticKey::Name(*name),
                    member_static_arguments: member_static_arguments.clone(),
                });

                let inherited = self.resolve_inherited_static_arguments(
                    module,
                    ctx.profile,
                    receiver_id.into_any(),
                    &receiver_ty,
                    &options,
                    tree,
                    symbols,
                    types,
                )?;
                inherited_static_arguments = inherited.arguments;
                inherited_substitutions = inherited.substitutions;

                // resolve member dispatch for the receiver type
                let member_key = StaticKey::Name(*name);
                let member_resolution = self.resolve_member_symbol_for_receiver(
                    module,
                    *receiver_id,
                    &receiver_ty,
                    &member_key,
                    ctx.profile,
                    tree,
                    symbols,
                    types,
                )?;
                let member_symbol = match &member_resolution {
                    MemberResolution::Static { symbol } => Some(*symbol),
                    _ => None,
                };
                call_member_resolution = Some(member_resolution);

                // prefer member instance arguments when static arguments live on the member
                let member_has_static_arguments = member_static_arguments
                    .as_ref()
                    .is_some_and(|arguments| !arguments.is_empty());

                // detect and report static argument conflicts
                let has_static_argument_conflict =
                    call_has_static_arguments && member_has_static_arguments;
                if has_static_argument_conflict {
                    self.error(AnalyzeError::ConflictingStaticArguments {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                if !call_has_static_arguments && member_has_static_arguments {
                    member_instance_arguments = self.member_instance_arguments_for_call(
                        module,
                        unwrapped_left_id,
                        member_symbol,
                        types,
                    );
                }

                (member_symbol, has_static_argument_conflict)
            }
            _ => (
                self.reference_symbol_for_expression(
                    module,
                    unwrapped_left_id,
                    ctx.profile,
                    tree,
                    symbols,
                ),
                false,
            ),
        };

        // resolve the callee signature and static arguments
        // (skip call's static arguments if there was a conflict, as the error is already reported)
        let effective_static_arguments = if has_static_argument_conflict {
            None
        } else {
            static_arguments
        };
        let union_static_arguments = if has_static_argument_conflict {
            None
        } else if call_has_static_arguments {
            static_arguments
        } else {
            member_call_context
                .as_ref()
                .and_then(|context| context.member_static_arguments.as_deref())
        };

        // handle union receiver member calls with dynamic resolution
        if let Some(context) = member_call_context.as_ref()
            && let Type::Union { elements } = types.get_type(context.receiver_ty_id)
        {
            // resolve union candidates for the member call
            let element_ids = elements.clone();
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
                &options,
                tree,
                symbols,
                types,
                infer,
            )?;
            let Some(candidates) = candidates else {
                // infer arguments without contextual types
                for argument_id in dynamic_arguments {
                    self.infer_argument(
                        module,
                        *argument_id,
                        None,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }

                // record unresolved union resolution
                self.record_unresolved_resolution(
                    expression_id.into_global_any(module.id),
                    Some(context.receiver_ty_id),
                    Vec::new(),
                    Vec::new(),
                    types,
                );

                // fall back to unknown when union lookup fails
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from(ty, expression_id));
            };

            // compute expected argument types when uniform across candidates
            let mut expected_argument_types = Vec::with_capacity(dynamic_arguments.len());
            for index in 0..dynamic_arguments.len() {
                let mut expected = None;
                let mut is_uniform = true;
                for candidate in &candidates {
                    let Some(param_ty_id) =
                        candidate.signature.dynamic_parameters.get(index).copied()
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

            // infer arguments with contextual types when possible
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
                    self.infer_expression(
                        module,
                        argument_value_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?
                };
                argument_ty_ids.push(argument_ty_id);
            }

            // add constraints between arguments and parameters for each candidate
            for candidate in &candidates {
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

            // check argument assignability against each candidate signature
            let mut is_valid_for_all = true;
            'candidate: for candidate in &candidates {
                for (argument_ty_id, param_ty_id) in argument_ty_ids
                    .iter()
                    .zip(candidate.signature.dynamic_parameters.iter())
                {
                    if !self.is_infer_var_type(*param_ty_id, types)
                        && !self.is_infer_var_type(*argument_ty_id, types)
                        && self.is_type_assignable(
                            module,
                            ctx.profile,
                            symbols,
                            *param_ty_id,
                            *argument_ty_id,
                            types,
                            &options,
                        ) == Assignability::NotAssignable
                    {
                        is_valid_for_all = false;
                        break 'candidate;
                    }
                }
            }

            // report overload mismatch across union candidates
            if !is_valid_for_all {
                self.error(AnalyzeError::NoOverload {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    receiver_ty: context.receiver_ty_id.into_global(module.id),
                });

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from(ty, expression_id));
            }

            // compute union return type from candidate signatures
            let mut void_type_id = None;
            let mut return_type_ids = Vec::new();
            for candidate in &candidates {
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

            // materialize the union return type
            let return_type_id = match return_type_ids.len() {
                0 => {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    types.insert_type_from(ty, expression_id)
                }
                1 => return_type_ids[0],
                _ => self.union_type_from_list(return_type_ids, context.receiver_ty_id, types),
            };

            // record dynamic resolution for union dispatch
            let resolution_candidates = candidates
                .into_iter()
                .map(|candidate| {
                    let instance_id = if candidate.instance_arguments.is_empty() {
                        None
                    } else {
                        Some(self.register_instance_for_symbol(
                            candidate.symbol,
                            candidate.instance_arguments,
                            types,
                        ))
                    };
                    ResolutionCandidate {
                        key: Some(DispatchKey::single(candidate.receiver_ty_id)),
                        target_symbol: candidate.symbol,
                        instance: instance_id,
                        resolved_signature: Some(candidate.signature),
                    }
                })
                .collect();

            self.record_dynamic_resolution(
                expression_id.into_global_any(module.id),
                Some(context.receiver_ty_id),
                resolution_candidates,
                types,
            );

            // short circuit because the union call is handled
            return Ok(return_type_id);
        }

        let call_signatures = self.call_signatures_for_type(callee_ty_id, types);
        let ty_id = if !call_signatures.is_empty() {
            // select the matching overload
            let selection = self.select_call_signature(
                module,
                expression_id,
                callee_symbol,
                effective_static_arguments,
                &call_signatures,
                dynamic_arguments,
                call_receiver_ty_id,
                ctx.profile,
                &options,
                tree,
                symbols,
                types,
                infer,
            )?;
            let (_signature_ty_id, resolved) = if let Some(selection) = selection {
                selection
            } else if call_signatures.len() > 1 {
                self.error(AnalyzeError::NoOverload {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    receiver_ty: callee_ty_id.into_global(module.id),
                });
                for argument_id in dynamic_arguments {
                    self.infer_argument(
                        module,
                        *argument_id,
                        None,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from(ty, expression_id));
            } else {
                let signature_ty_id = call_signatures[0];
                let resolved = self.resolve_call_signature(
                    module,
                    expression_id,
                    callee_symbol,
                    effective_static_arguments,
                    signature_ty_id,
                    call_receiver_ty_id,
                    ctx.profile,
                    &options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;
                match resolved {
                    Some(resolved) => (signature_ty_id, resolved),
                    None => {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        return Ok(types.insert_type_from(ty, expression_id));
                    }
                }
            };

            let mut resolved_signature = resolved;
            if !inherited_substitutions.is_empty() {
                let mut cache = HashMap::new();
                let dynamic_parameters = resolved_signature
                    .dynamic_parameters
                    .iter()
                    .map(|parameter| {
                        self.substitute_static_parameters(
                            *parameter,
                            &inherited_substitutions,
                            types,
                            &mut cache,
                        )
                    })
                    .collect();
                let return_type = resolved_signature.return_type.map(|return_type| {
                    self.substitute_static_parameters(
                        return_type,
                        &inherited_substitutions,
                        types,
                        &mut cache,
                    )
                });
                resolved_signature = ResolvedSignature {
                    dynamic_parameters,
                    return_type,
                    static_arguments: resolved_signature.static_arguments.clone(),
                };
            }
            let resolved_return_type = resolved_signature.return_type;
            let resolved_dynamic_parameters = &resolved_signature.dynamic_parameters;
            let resolved_static_arguments = &resolved_signature.static_arguments;

            // infer argument types and constraints
            let argument_ty_ids = self.infer_invocation_arguments(
                module,
                dynamic_arguments,
                resolved_dynamic_parameters,
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
                &options,
            )?;

            // register the instance if the call is to a symbol
            let mut call_instance_id = None;
            if let Some(callee_symbol) = callee_symbol {
                let instance_arguments = member_instance_arguments.unwrap_or_else(|| {
                    let mut arguments = inherited_static_arguments;
                    arguments.extend(resolved_static_arguments.iter().cloned());
                    arguments
                });

                if !instance_arguments.is_empty() {
                    let instance_id = self.register_instance_for_node(
                        expression_id.into_global_any(module.id),
                        callee_symbol,
                        instance_arguments,
                        types,
                    );
                    call_instance_id = Some(instance_id);
                }
            }

            // record call resolution when possible
            match (call_member_resolution, callee_symbol) {
                (Some(member_resolution), _) => {
                    self.record_member_resolution(
                        expression_id.into_global_any(module.id),
                        call_receiver_ty_id,
                        &member_resolution,
                        call_instance_id,
                        Some(resolved_signature),
                        true,
                        types,
                    );
                }
                (None, Some(callee_symbol)) => {
                    self.record_static_resolution(
                        expression_id.into_global_any(module.id),
                        call_receiver_ty_id,
                        callee_symbol,
                        call_instance_id,
                        Some(resolved_signature),
                        types,
                    );
                }
                _ => {}
            }

            resolved_return_type.unwrap_or_else(|| {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            })
        } else {
            // suppress secondary diagnostics when member lookup already failed
            let has_missing_member = matches!(
                call_member_resolution,
                Some(MemberResolution::None | MemberResolution::Unresolved)
            );

            // report non-callable callee types unless they are dynamic placeholders
            let is_dynamic_callee = has_missing_member
                || matches!(
                    types.get_type(callee_ty_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Any
                    } | Type::InferVar { .. }
                        | Type::Infer { .. }
                        | Type::Error
                );
            if !is_dynamic_callee {
                self.error(AnalyzeError::NonCallable {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }

            // infer dynamic arguments
            for argument_id in dynamic_arguments {
                self.infer_argument(module, *argument_id, None, tree, symbols, types, infer, ctx)?;
            }

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from(ty, expression_id)
        };

        Ok(ty_id)
    }

    /// Infer a constructor call expression.
    pub(super) fn infer_new_expression(
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

        let callee_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
        let options = ctx.options;

        // ensure instance types for constructor references
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            callee_ty_id,
            types,
        )?;

        // resolve the callee symbol when possible
        let callee_id = self.unwrap_parenthesized_expression(left_id, tree);
        let callee_symbol =
            self.reference_symbol_for_expression(module, callee_id, ctx.profile, tree, symbols);

        // resolve construct signatures for the callee type
        let construct_signatures = self.construct_signatures_for_type(callee_ty_id, types);
        let ty_id = if !construct_signatures.is_empty() {
            // select the matching overload
            let selection = self.select_call_signature(
                module,
                expression_id,
                callee_symbol,
                static_arguments,
                &construct_signatures,
                dynamic_arguments,
                None,
                ctx.profile,
                &options,
                tree,
                symbols,
                types,
                infer,
            )?;
            let (_signature_ty_id, resolved) = if let Some(selection) = selection {
                selection
            } else if construct_signatures.len() > 1 {
                self.error(AnalyzeError::NoOverload {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    receiver_ty: callee_ty_id.into_global(module.id),
                });
                for argument_id in dynamic_arguments {
                    self.infer_argument(
                        module,
                        *argument_id,
                        None,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from(ty, expression_id));
            } else {
                let signature_ty_id = construct_signatures[0];
                let resolved = self.resolve_call_signature(
                    module,
                    expression_id,
                    callee_symbol,
                    static_arguments,
                    signature_ty_id,
                    None,
                    ctx.profile,
                    &options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;
                match resolved {
                    Some(resolved) => (signature_ty_id, resolved),
                    None => {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        return Ok(types.insert_type_from(ty, expression_id));
                    }
                }
            };

            let resolved_signature = resolved;
            let resolved_return_type = resolved_signature.return_type;
            let resolved_dynamic_parameters = &resolved_signature.dynamic_parameters;
            let resolved_static_arguments = &resolved_signature.static_arguments;

            // infer argument types and constraints
            let argument_ty_ids = self.infer_invocation_arguments(
                module,
                dynamic_arguments,
                resolved_dynamic_parameters,
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
                &options,
            )?;

            // register the instance when static arguments are resolved
            let mut constructor_instance_id = None;
            if let Some(callee_symbol) = callee_symbol
                && !resolved_static_arguments.is_empty()
            {
                let instance_id = self.register_instance_for_node(
                    expression_id.into_global_any(module.id),
                    callee_symbol,
                    resolved_static_arguments.clone(),
                    types,
                );
                constructor_instance_id = Some(instance_id);
            }

            // record constructor resolution when possible
            if let Some(callee_symbol) = callee_symbol {
                self.record_static_resolution(
                    expression_id.into_global_any(module.id),
                    None,
                    callee_symbol,
                    constructor_instance_id,
                    Some(resolved_signature),
                    types,
                );
            }

            resolved_return_type.unwrap_or_else(|| {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            })
        } else {
            // infer dynamic arguments
            for argument_id in dynamic_arguments {
                self.infer_argument(module, *argument_id, None, tree, symbols, types, infer, ctx)?;
            }

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from(ty, expression_id)
        };

        // reject managed allocations when managed memory is disabled
        if ctx.options.no_managed
            && !ctx.is_explicit_ownership
            && matches!(module.source, ModuleSource::User)
        {
            let value_ty = types.get_type(ty_id);
            if self.type_contains_managed(module, ctx.profile, value_ty, types) {
                self.error(AnalyzeError::ManagedMemoryDisabled {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        Ok(ty_id)
    }

    /// Resolve a function type for a call, substituting static parameters when provided.
    pub(super) fn resolve_function_signature(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        static_parameters: &[LocalTypeId],
        dynamic_parameters: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
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
            static_parameters,
            dynamic_parameters,
            return_type,
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
    pub(super) fn resolve_function_static_arguments(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_argument_ids: Option<&[LocalNodeId<Argument>]>,
        static_parameters: &[LocalTypeId],
        dynamic_parameters: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
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
                    self.error(AnalyzeError::MissingType {
                        node: argument_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
            }

            return Ok(Some(ResolvedSignature {
                dynamic_parameters: dynamic_parameters.to_vec(),
                return_type,
                static_arguments: Vec::new(),
            }));
        }

        // collect static parameter symbols from the function type
        let static_parameter_symbols = static_parameters
            .iter()
            .filter_map(|parameter_id| match types.get_type(*parameter_id) {
                Type::Reference { symbol, .. } => Some(*symbol),
                _ => None,
            })
            .collect::<Vec<_>>();

        // detect whether any static parameter kinds need inference
        let mut needs_inference = false;
        for symbol_id in static_parameter_symbols.iter().copied() {
            if types.get_static_parameter_kind(symbol_id).is_none() {
                needs_inference = true;
                break;
            }
        }

        // collect referenced symbols from the dynamic signature when needed
        let mut referenced_symbols = HashSet::new();
        if needs_inference {
            let mut visited = HashSet::new();
            for ty_id in dynamic_parameters.iter().copied().chain(return_type) {
                self.collect_type_reference_symbols(
                    ty_id,
                    types,
                    &mut referenced_symbols,
                    &mut visited,
                );
            }

            self.ensure_static_parameter_kinds_for_signature(
                module,
                profile,
                node_id,
                &static_parameter_symbols,
                &referenced_symbols,
                tree,
                symbols,
                types,
            )?;
        }

        // collect static parameters with cached kinds
        let static_parameters = static_parameter_symbols
            .iter()
            .map(|symbol_id| {
                let kind = types
                    .get_static_parameter_kind(*symbol_id)
                    .unwrap_or(StaticParameterKind::Type);
                self.collect_static_parameter(
                    module, *symbol_id, kind, node_id, profile, tree, symbols, types,
                )
            })
            .collect::<Vec<_>>();

        // assign static arguments to static parameters
        let argument_ids = static_argument_ids.unwrap_or(&[]);
        let argument_values = argument_ids
            .iter()
            .map(|argument_id| StaticArgument::Unevaluated { node: *argument_id })
            .collect::<Vec<_>>();
        let assigned_arguments = self.assign_static_argument_values(
            module.id,
            profile,
            node_id,
            &argument_values,
            &static_parameters,
            tree,
        );

        // resolve each parameter and build substitutions
        let mut substitutions = HashMap::new();
        let mut resolved_arguments = Vec::with_capacity(static_parameters.len());
        for (index, static_parameter) in static_parameters.iter().enumerate() {
            let assigned_argument = assigned_arguments.get(index).cloned().flatten();

            let error_node = if let Some(argument) = &assigned_argument {
                match argument {
                    StaticArgument::Unevaluated { node } => node.into_global_any(module.id),
                    StaticArgument::Evaluated { .. } => node_id.into_global(module.id),
                }
            } else if let Some(default_expression) = static_parameter.default_expression.as_ref() {
                default_expression.local_id.into_global_any(module.id)
            } else {
                node_id.into_global(module.id)
            };

            // resolve the static argument value
            let resolved_argument = match self.resolve_static_argument(
                module,
                profile,
                static_parameter,
                assigned_argument,
                true,
                tree,
                symbols,
                types,
            )? {
                Some(argument) => argument,
                None => self.missing_static_argument_for_function(
                    module,
                    profile,
                    node_id,
                    owner_symbol,
                    static_parameter,
                    infer,
                    types,
                )?,
            };

            // materialized type argument validation when needed
            let materialized_substitution = if static_parameter.kind == StaticParameterKind::Type {
                Some(self.materialize_static_type_argument(
                    module,
                    profile,
                    error_node,
                    static_parameter,
                    &resolved_argument,
                    tree,
                    symbols,
                    types,
                )?)
            } else {
                None
            };

            // record substitutions and constraints
            if let Some(substitution_ty_id) = self.validate_static_argument(
                module,
                profile,
                error_node,
                static_parameter,
                &resolved_argument,
                materialized_substitution,
                symbols,
                types,
                Some(infer),
                options,
            )? {
                substitutions.insert(static_parameter.symbol, substitution_ty_id);
            }

            resolved_arguments.push(resolved_argument);
        }

        // apply substitutions to the dynamic signature
        let mut cache = HashMap::new();
        let resolved_dynamic_parameters = dynamic_parameters
            .iter()
            .map(|parameter| {
                self.substitute_static_parameters(*parameter, &substitutions, types, &mut cache)
            })
            .collect::<Vec<_>>();
        let resolved_return_type = return_type.map(|return_type| {
            self.substitute_static_parameters(return_type, &substitutions, types, &mut cache)
        });

        Ok(Some(ResolvedSignature {
            dynamic_parameters: resolved_dynamic_parameters,
            return_type: resolved_return_type,
            static_arguments: resolved_arguments,
        }))
    }

    /// Resolve a member function for an operator invocation.
    ///
    /// Handles the common pattern of looking up a member on a receiver type,
    /// applying inherited substitutions, and resolving the function signature.
    pub(super) fn resolve_member_function(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<ResolvedMemberFunction>> {
        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            profile,
            receiver_expression_id.into_any(),
            receiver_ty,
            options,
            tree,
            symbols,
            types,
        )?;

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
        let mut substitutions = inherited.substitutions.clone();
        if let Some(context) = extension_context.as_ref() {
            for (symbol, ty_id) in &context.substitutions {
                substitutions.insert(*symbol, *ty_id);
            }
        }

        // select instance arguments for member instancing
        let instance_arguments = match extension_context.as_ref() {
            Some(context) => context.arguments.clone(),
            None => inherited.arguments.clone(),
        };

        // decide how to filter member lookups for this receiver
        let lookup_mode = self.member_lookup_mode_for_receiver_expression(
            module,
            receiver_expression_id,
            receiver_ty,
            profile,
            tree,
            symbols,
        );

        // infer the member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            profile,
            receiver_expression_id.into_any(),
            receiver_ty,
            member_key,
            lookup_mode,
            types,
            &mut member_type_visited,
        )?;
        let has_member = member_ty_id.is_some();

        // bail of we don't know the member type
        let Some(member_ty_id) = member_ty_id else {
            return Ok(Some(ResolvedMemberFunction {
                signature: ResolvedSignature {
                    dynamic_parameters: Vec::new(),
                    return_type: None,
                    static_arguments: Vec::new(),
                },
                member_resolution,
                member_symbol,
                instance_arguments,
                has_member: false,
            }));
        };

        // apply static substitutions
        let member_ty_id = if !substitutions.is_empty() {
            let mut cache = HashMap::new();
            self.substitute_static_parameters(member_ty_id, &substitutions, types, &mut cache)
        } else {
            member_ty_id
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
        let signature = self.resolve_function_signature(
            module,
            expression_id.into_any(),
            member_symbol,
            None,
            &static_parameters,
            &dynamic_parameters,
            return_type,
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
            instance_arguments,
            has_member,
        }))
    }

    /// Register an instance and record resolution for a member function invocation.
    ///
    /// Returns the instance ID if one was registered.
    pub(super) fn record_member_call_resolution(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        resolved: &ResolvedMemberFunction,
        types: &mut TypeTable,
    ) -> Option<LocalInstanceId> {
        let mut instance_id = None;

        // register instance if needed
        if let Some(member_symbol) = resolved.member_symbol {
            let mut instance_arguments = resolved.instance_arguments.clone();
            instance_arguments.extend(resolved.signature.static_arguments.clone());
            if !instance_arguments.is_empty() {
                instance_id = Some(self.register_instance_for_node(
                    expression_id.into_global_any(module.id),
                    member_symbol,
                    instance_arguments,
                    types,
                ));
            }
        }

        // record member resolution
        self.record_member_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            &resolved.member_resolution,
            instance_id,
            Some(resolved.signature.clone()),
            resolved.has_member,
            types,
        );

        instance_id
    }
}
