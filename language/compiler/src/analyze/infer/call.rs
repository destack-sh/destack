use std::collections::HashMap;

use super::argument::MissingStaticArgument;
use super::parameter::StaticParameterKind;
use super::resolution::MemberResolution;
use crate::{
    AnalyzeError, AnalyzeResult, Assignability, Compiler, Constraint, InferContext, InferTable,
};
use destack_dir::{
    Argument, Expression, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, NodeTree, StaticArgument, StaticKey, SymbolTable, Type,
    TypeLiteral, TypeTable,
};
use destack_workspace::Module;

/// Resolved static arguments for a function instantiation.
#[derive(Debug, Clone)]
pub(super) struct ResolvedStaticArguments {
    /// Dynamic parameter types after substitution.
    pub(super) dynamic_parameters: Vec<LocalTypeId>,
    /// Return type after substitution.
    pub(super) return_type: Option<LocalTypeId>,
    /// Static arguments in declared order.
    pub(super) static_arguments: Vec<StaticArgument>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
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

        // track static arguments that come from member expressions
        let call_has_static_arguments =
            static_arguments.is_some_and(|arguments| !arguments.is_empty());
        let mut member_instance_arguments = None;
        let mut call_member_resolution = None;
        let mut call_receiver_ty_id = None;
        let unwrapped_left_id = self.unwrap_parenthesized_expression(left_id, tree);

        // resolve inherited static arguments and the callee symbol
        let mut inherited_static_arguments = Vec::new();
        let callee_symbol = match tree.get(unwrapped_left_id) {
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
                        receiver_id.into_any(),
                        *symbol,
                        static_arguments.as_deref(),
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

                if call_has_static_arguments && member_has_static_arguments {
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

                member_symbol
            }
            _ => self.reference_symbol_for_expression(module, unwrapped_left_id, tree, symbols),
        };

        // resolve the callee signature and static arguments
        let ty_id = match types.get_type(callee_ty_id).clone() {
            Type::Function {
                static_parameters,
                dynamic_parameters,
                return_type,
                ..
            } => {
                let resolved = self.resolve_function_type_for_call(
                    module,
                    expression_id.into_any(),
                    callee_symbol,
                    static_arguments,
                    &static_parameters,
                    &dynamic_parameters,
                    return_type,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;

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
                        sub: *argument_ty_id,
                        sup: *param_ty_id,
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
                        && self.check_is_type_assignable(*param_ty_id, *argument_ty_id, types)
                            == Assignability::NotAssignable
                    {
                        let argument_node = dynamic_arguments
                            .get(index)
                            .map(|id| id.into_global_any(module.id))
                            .unwrap_or_else(|| expression_id.into_global_any(module.id));
                        return Err(AnalyzeError::UnassignableType {
                            node: argument_node,
                            expected_ty: GlobalTypeId {
                                module_id: module.id,
                                local_id: *param_ty_id,
                            },
                            actual_ty: GlobalTypeId {
                                module_id: module.id,
                                local_id: *argument_ty_id,
                            },
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
            }
            _ => {
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
                types.insert_type_from(ty, expression_id)
            }
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
    pub(super) fn resolve_function_type_for_call(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        static_parameters: &[LocalTypeId],
        dynamic_parameters: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<ResolvedStaticArguments> {
        let resolved = self.resolve_function_static_arguments(
            module,
            node_id,
            owner_symbol,
            static_arguments,
            static_parameters,
            dynamic_parameters,
            return_type,
            tree,
            symbols,
            types,
            infer,
        )?;

        if let Some(resolved) = resolved {
            return Ok(resolved);
        }

        Ok(ResolvedStaticArguments {
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
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<Option<ResolvedStaticArguments>> {
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

            return Ok(Some(ResolvedStaticArguments {
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

        let static_parameters = self.collect_static_parameters_for_symbols(
            module,
            &static_parameter_symbols,
            tree,
            symbols,
            types,
        );

        let static_parameter_kinds = self.collect_static_parameter_kinds_for_function(
            &static_parameters,
            dynamic_parameters,
            return_type,
            types,
        );

        // map arguments to parameters
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
            // decide which argument and kind apply to the parameter
            let parameter_kind = static_parameter_kinds
                .get(&static_parameter.symbol)
                .copied()
                .unwrap_or(StaticParameterKind::Type);
            let assigned_argument = assigned_arguments.get(index).cloned().flatten();

            let error_node = if let Some(argument) = &assigned_argument {
                match argument {
                    StaticArgument::Unevaluated { node } => node.into_global_any(module.id),
                    StaticArgument::Evaluated { .. } => node_id.into_global(module.id),
                }
            } else if let Some(default_expression) = static_parameter.default_expression.as_ref() {
                GlobalNodeIdAny::new(
                    default_expression.module_id,
                    default_expression.local_id.into_any(),
                )
            } else {
                node_id.into_global(module.id)
            };

            // resolve the static argument value
            let resolved_argument = self.resolve_static_argument_for_parameter(
                module,
                static_parameter,
                parameter_kind,
                assigned_argument,
                tree,
                symbols,
                types,
                MissingStaticArgument::Function {
                    node_id,
                    owner_symbol,
                    infer,
                },
            )?;

            // record substitutions and constraints
            if let Some(substitution_ty_id) = self.validate_static_argument(
                module,
                error_node,
                static_parameter,
                parameter_kind,
                &resolved_argument,
                types,
                Some(infer),
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
                self.substitute_static_parameters_in_type(
                    *parameter,
                    &substitutions,
                    types,
                    &mut cache,
                )
            })
            .collect::<Vec<_>>();
        let resolved_return_type = return_type.map(|return_type| {
            self.substitute_static_parameters_in_type(
                return_type,
                &substitutions,
                types,
                &mut cache,
            )
        });

        Ok(Some(ResolvedStaticArguments {
            dynamic_parameters: resolved_dynamic_parameters,
            return_type: resolved_return_type,
            static_arguments: resolved_arguments,
        }))
    }
}
