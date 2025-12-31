use std::collections::{HashMap, HashSet};

use super::parameter::StaticParameterKind;
use super::resolve::MemberResolution;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    Argument, Constraint, Expression, GlobalSymbolId, InferTable, LocalInstanceId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, NodeTree, StaticArgument, StaticKey, SymbolTable, Type,
    TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Resolved function signature after static argument substitution.
#[derive(Debug, Clone)]
pub(super) struct ResolvedFunctionSignature {
    /// Dynamic parameter types after substitution.
    pub(super) dynamic_parameters: Vec<LocalTypeId>,
    /// Return type after substitution.
    pub(super) return_type: Option<LocalTypeId>,
    /// Static arguments in declared order.
    pub(super) static_arguments: Vec<StaticArgument>,
}

/// Resolved member function for an operator invocation.
#[derive(Debug)]
pub(super) struct ResolvedMemberFunction {
    /// The resolved function signature (parameters and return type).
    pub(super) signature: ResolvedFunctionSignature,
    /// The member resolution for dispatch recording.
    pub(super) member_resolution: MemberResolution,
    /// The resolved member symbol (if statically known).
    pub(super) member_symbol: Option<GlobalSymbolId>,
    /// Inherited static arguments from the receiver type.
    pub(super) inherited_arguments: Vec<StaticArgument>,
    /// Whether the member was found on the receiver type.
    pub(super) has_member: bool,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect all callable signatures for a type (for #Overloads support).
    fn call_signatures_for_type(&self, ty_id: LocalTypeId, types: &TypeTable) -> Vec<LocalTypeId> {
        // use direct function types as call signatures
        if matches!(types.get_type(ty_id), Type::Function { .. }) {
            return vec![ty_id];
        }

        // collect call signatures from object types
        if let Type::Object {
            call_signatures, ..
        } = types.get_type(ty_id)
        {
            return call_signatures.clone();
        }

        // follow nominal references into their instance types
        if let Type::Reference { symbol, .. } = types.get_type(ty_id)
            && let Some(instance_id) = types.get_instance_type_id(*symbol)
            && let Type::Object {
                call_signatures, ..
            } = types.get_type(instance_id)
        {
            return call_signatures.clone();
        }

        // unwrap value types to their underlying type
        if let Type::Value { value } = types.get_type(ty_id) {
            return self.call_signatures_for_type(*value, types);
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
    ) -> AnalyzeResult<Option<ResolvedFunctionSignature>> {
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

            return Ok(Some(ResolvedFunctionSignature {
                dynamic_parameters: mapped_parameters,
                return_type: mapped_return,
                static_arguments: resolved.static_arguments,
            }));
        }

        Ok(Some(resolved))
    }

    /// Resolve the literal argument types when every argument is a scalar literal.
    /// NOTE #Cleanup #Overloads: literal only selection avoids contextual widening
    fn literal_argument_types(
        &self,
        arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Option<Vec<LocalTypeId>> {
        let mut literal_types = Vec::with_capacity(arguments.len());

        // bail out if any argument is non-literal
        for argument_id in arguments {
            let argument = tree.get(*argument_id);
            if matches!(argument, Argument::Spread { .. }) {
                return None;
            }

            let value_id = argument.value();
            let Expression::ScalarLiteral { value } = tree.get(value_id) else {
                return None;
            };

            let literal_ty = self.infer_scalar_literal(value);
            let ty = Type::TypeLiteral { value: literal_ty };
            let ty_id = types.insert_type_from_any(ty, value_id.into_any());
            literal_types.push(ty_id);
        }

        Some(literal_types)
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
    ) -> AnalyzeResult<Option<(LocalTypeId, ResolvedFunctionSignature)>> {
        // only attempt selection when overloads exist
        if signature_ids.len() <= 1 {
            return Ok(None);
        }

        // collect the argument types for literal only selection (until full #Overloads support)
        let Some(argument_types) = self.literal_argument_types(dynamic_arguments, tree, types)
        else {
            return Ok(None);
        };

        // select the matching overload
        // FUGU #Overloads: replace literal only heuristic with full overload selection
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

            let mut matches = true;
            for (argument_ty_id, param_ty_id) in argument_types
                .iter()
                .zip(resolved.dynamic_parameters.iter())
            {
                if self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *param_ty_id,
                    *argument_ty_id,
                    types,
                    options,
                ) == Assignability::NotAssignable
                {
                    matches = false;
                    break;
                }
            }

            if matches {
                return Ok(Some((*signature_ty_id, resolved)));
            }
        }

        Ok(None)
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
        let unwrapped_left_id = self.unwrap_parenthesized_expression(left_id, tree);

        // resolve inherited static arguments and the callee symbol
        let mut inherited_static_arguments = Vec::new();
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

                if let Type::Reference {
                    symbol,
                    static_arguments,
                } = &receiver_ty
                    && let Some(resolved) = self.resolve_type_reference_static_arguments(
                        module,
                        ctx.profile,
                        receiver_id.into_any(),
                        *symbol,
                        static_arguments.as_deref(),
                        &options,
                        tree,
                        symbols,
                        types,
                    )?
                {
                    inherited_static_arguments = resolved;
                }

                // resolve member dispatch for the receiver type
                let member_key = StaticKey::Name(*name);
                let member_resolution = self.resolve_member_resolution(
                    module,
                    &receiver_ty,
                    &member_key,
                    ctx.profile,
                    tree,
                    symbols,
                    types,
                );
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
                        node: expression_id.into_global_any(module.id),
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

            let resolved_dynamic_parameters = resolved.dynamic_parameters;
            let resolved_return_type = resolved.return_type;
            let resolved_static_arguments = resolved.static_arguments;

            // analyze arguments with contextual parameter types
            let mut argument_ty_ids = Vec::with_capacity(dynamic_arguments.len());
            for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                let expected_arg_ty_id = resolved_dynamic_parameters.get(index).copied();
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
                let argument_ty_id = self.infer_expression(
                    module,
                    argument_value_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
                argument_ty_ids.push(argument_ty_id);
            }

            // add constraints between arguments and parameters
            for (argument_ty_id, param_ty_id) in argument_ty_ids
                .iter()
                .zip(resolved_dynamic_parameters.iter())
            {
                infer.push_constraint(Constraint::Subtype {
                    sub_type: *argument_ty_id,
                    super_type: *param_ty_id,
                    variance: None,
                });
            }

            // check argument assignability against parameters
            for (index, (argument_ty_id, param_ty_id)) in argument_ty_ids
                .iter()
                .zip(resolved_dynamic_parameters.iter())
                .enumerate()
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
                    let argument_node = dynamic_arguments
                        .get(index)
                        .map(|id| id.into_global_any(module.id))
                        .unwrap_or_else(|| expression_id.into_global_any(module.id));
                    return Err(AnalyzeError::UnassignableType {
                        node: argument_node,
                        expected_ty: param_ty_id.into_global(module.id),
                        actual_ty: argument_ty_id.into_global(module.id),
                    });
                }
            }

            // register the instance if the call is to a symbol
            let mut call_instance_id = None;
            if let Some(callee_symbol) = callee_symbol {
                let instance_arguments = member_instance_arguments.unwrap_or_else(|| {
                    let mut arguments = inherited_static_arguments;
                    arguments.extend(resolved_static_arguments);
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
            if let Some(member_resolution) = call_member_resolution {
                self.record_member_resolution(
                    expression_id.into_global_any(module.id),
                    call_receiver_ty_id,
                    &member_resolution,
                    call_instance_id,
                    true,
                    types,
                );
            } else if let Some(callee_symbol) = callee_symbol {
                self.record_static_resolution(
                    expression_id.into_global_any(module.id),
                    call_receiver_ty_id,
                    callee_symbol,
                    call_instance_id,
                    types,
                );
            }

            resolved_return_type.unwrap_or_else(|| {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
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

        Ok(ty_id)
    }

    /// Infer a constructor call expression.
    pub(super) fn infer_new_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let _left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;

        for argument_id in dynamic_arguments {
            self.infer_argument(module, *argument_id, None, tree, symbols, types, infer, ctx)?;
        }

        // NOTE #Incomplete: resolve instance type from constructor
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        };
        Ok(types.insert_type_from(ty, expression_id))
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
    ) -> AnalyzeResult<ResolvedFunctionSignature> {
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

        Ok(ResolvedFunctionSignature {
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
    ) -> AnalyzeResult<Option<ResolvedFunctionSignature>> {
        // handle fast paths when there are no static parameters
        let has_static_arguments = static_argument_ids.is_some_and(|args| !args.is_empty());

        if static_parameters.is_empty() && !has_static_arguments {
            return Ok(None);
        }

        if static_parameters.is_empty() {
            if let Some(argument_ids) = static_argument_ids {
                for argument_id in argument_ids {
                    self.error(AnalyzeError::MissingType {
                        node: argument_id.into_global_any(module.id),
                    });
                }
            }

            return Ok(Some(ResolvedFunctionSignature {
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

        // collect referenced symbols from dynamic signature
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        for ty_id in dynamic_parameters.iter().copied().chain(return_type) {
            self.collect_type_reference_symbols(
                ty_id,
                types,
                &mut referenced_symbols,
                &mut visited,
            );
        }

        // collect static parameters
        let static_parameters: Vec<_> = static_parameter_symbols
            .iter()
            .map(|symbol_id| {
                let kind = if referenced_symbols.contains(symbol_id) {
                    StaticParameterKind::Type
                } else {
                    StaticParameterKind::Value
                };
                self.collect_static_parameter(
                    module, *symbol_id, kind, profile, tree, symbols, types,
                )
            })
            .collect();

        // assign static arguments to static parameters
        let argument_ids = static_argument_ids.unwrap_or(&[]);
        let argument_values = argument_ids
            .iter()
            .map(|argument_id| StaticArgument::Unevaluated { node: *argument_id })
            .collect::<Vec<_>>();
        let assigned_arguments = self.assign_static_argument_values(
            module.id,
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
                tree,
                symbols,
                types,
            )? {
                Some(argument) => argument,
                None => self.missing_static_argument_for_function(
                    module,
                    node_id,
                    owner_symbol,
                    static_parameter,
                    infer,
                    types,
                )?,
            };

            // record substitutions and constraints
            if let Some(substitution_ty_id) = self.validate_static_argument(
                module,
                profile,
                error_node,
                static_parameter,
                &resolved_argument,
                symbols,
                types,
                Some(infer),
                options,
            ) {
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

        Ok(Some(ResolvedFunctionSignature {
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
        let member_resolution = self.resolve_member_resolution(
            module,
            receiver_ty,
            member_key,
            profile,
            tree,
            symbols,
            types,
        );
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // infer the member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            profile,
            receiver_expression_id.into_any(),
            receiver_ty,
            member_key,
            types,
            &mut member_type_visited,
        )?;
        let has_member = member_ty_id.is_some();

        // bail of we don't know the member type
        let Some(member_ty_id) = member_ty_id else {
            return Ok(Some(ResolvedMemberFunction {
                signature: ResolvedFunctionSignature {
                    dynamic_parameters: Vec::new(),
                    return_type: None,
                    static_arguments: Vec::new(),
                },
                member_resolution,
                member_symbol,
                inherited_arguments: inherited.arguments,
                has_member: false,
            }));
        };

        // apply inherited substitutions
        let member_ty_id = if !inherited.substitutions.is_empty() {
            let mut cache = HashMap::new();
            self.substitute_static_parameters(
                member_ty_id,
                &inherited.substitutions,
                types,
                &mut cache,
            )
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
            inherited_arguments: inherited.arguments,
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
            let mut instance_arguments = resolved.inherited_arguments.clone();
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
            resolved.has_member,
            types,
        );

        instance_id
    }
}
