use std::collections::HashMap;

use super::parameter::{StaticParameter, StaticParameterKind};
use crate::{
    AnalyzeError, AnalyzeResult, Assignability, Compiler, Constraint, InferOrigin, InferScope,
    InferTable,
};
use destack_dir::{
    Argument, Expression, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, NodeTree, StaticArgument, StaticExpression, StaticProperty,
    StringId, SymbolTable, Type, TypeField, TypeLiteral, TypeTable,
};
use destack_workspace::Module;

/// Describe how to handle missing static arguments.
pub(super) enum MissingStaticArgument<'a> {
    /// Use type reference defaults and fallbacks.
    TypeReference {
        /// The node that triggered the lookup.
        node_id: LocalNodeIdAny,
    },
    /// Infer missing arguments for a function call.
    Function {
        /// The node that triggered the call.
        node_id: LocalNodeIdAny,
        /// The owning symbol when available.
        owner_symbol: Option<GlobalSymbolId>,
        /// Use the inference table to allocate variables.
        infer: &'a mut InferTable,
    },
}

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
    /// Map static argument values to parameters by name and position.
    fn assign_static_argument_values(
        &self,
        module_id: destack_source::ModuleId,
        node_id: LocalNodeIdAny,
        static_arguments: &[StaticArgument],
        parameters: &[StaticParameter],
        tree: &NodeTree,
    ) -> Vec<Option<StaticArgument>> {
        // track assignments by parameter index
        let mut assigned: Vec<Option<StaticArgument>> = vec![None; parameters.len()];
        let mut next_index = 0;

        for argument in static_arguments {
            // resolve argument name for named mapping
            let (argument_name, is_spread) = match argument {
                StaticArgument::Evaluated { name, .. } => (*name, false),
                StaticArgument::Unevaluated { node } => match tree.get(*node) {
                    Argument::Named { name, .. } => (Some(*name), false),
                    Argument::Spread { .. } => (None, true),
                    _ => (None, false),
                },
            };

            // report unsupported spread arguments
            if is_spread {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => node.into_global_any(module_id),
                    _ => node_id.into_global(module_id),
                };
                self.error(AnalyzeError::MissingType { node: error_node });
                continue;
            }

            // select the target parameter index
            let target_index = match argument_name {
                Some(name) => parameters
                    .iter()
                    .position(|parameter| parameter.name == Some(name)),
                None => {
                    let mut index = next_index;
                    while index < parameters.len() && assigned[index].is_some() {
                        index += 1;
                    }
                    next_index = index + 1;
                    if index < parameters.len() {
                        Some(index)
                    } else {
                        None
                    }
                }
            };

            // report unknown or overflowed argument positions
            let Some(target_index) = target_index else {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => node.into_global_any(module_id),
                    _ => node_id.into_global(module_id),
                };
                self.error(AnalyzeError::MissingType { node: error_node });
                continue;
            };

            // reject duplicate assignments
            if assigned[target_index].is_some() {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => node.into_global_any(module_id),
                    _ => node_id.into_global(module_id),
                };
                self.error(AnalyzeError::MissingType { node: error_node });
                continue;
            }

            assigned[target_index] = Some(argument.clone());
        }

        assigned
    }

    /// Resolve a static argument for a parameter.
    pub(super) fn resolve_static_argument_for_parameter(
        &self,
        module: &Module,
        static_parameter: &StaticParameter,
        parameter_kind: StaticParameterKind,
        assigned_argument: Option<StaticArgument>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        missing_argument: MissingStaticArgument<'_>,
    ) -> AnalyzeResult<StaticArgument> {
        // resolve explicit argument when provided
        if let Some(argument) = assigned_argument {
            let resolved_argument = match (parameter_kind, argument) {
                (StaticParameterKind::Type, StaticArgument::Unevaluated { node }) => {
                    let resolved =
                        self.evaluate_static_argument_as_type(module, node, tree, symbols, types)?;
                    resolved.unwrap_or(StaticArgument::Unevaluated { node })
                }
                (StaticParameterKind::Value, StaticArgument::Unevaluated { node }) => self
                    .evaluate_static_argument_as_value(node, tree)
                    .unwrap_or(StaticArgument::Unevaluated { node }),
                (StaticParameterKind::Type, StaticArgument::Evaluated { name, value }) => {
                    let argument = StaticArgument::Evaluated {
                        name,
                        value: value.clone(),
                    };
                    let ty_id = self.convert_static_argument_to_type_id(&argument, types);

                    StaticArgument::Evaluated {
                        name,
                        value: StaticExpression::Type { ty: ty_id },
                    }
                }
                (_, argument) => argument,
            };

            return Ok(resolved_argument);
        }

        // apply default expression when present
        if let Some(default_expression) = static_parameter.default_expression.as_ref() {
            return self.evaluate_static_default_argument(
                parameter_kind,
                static_parameter.name,
                default_expression,
                types,
            );
        }

        // synthesize a fallback when no argument is available
        match missing_argument {
            MissingStaticArgument::TypeReference { node_id } => match parameter_kind {
                StaticParameterKind::Type => {
                    let unknown_ty_id = types.insert_type(Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    });
                    Ok(StaticArgument::Evaluated {
                        name: static_parameter.name,
                        value: StaticExpression::Type { ty: unknown_ty_id },
                    })
                }
                StaticParameterKind::Value => {
                    let fallback_expression = node_id
                        .try_into_typed::<Expression>()
                        .map(|expression_id| StaticExpression::Unevaluated {
                            node: expression_id,
                        })
                        .unwrap_or(StaticExpression::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        });

                    Ok(StaticArgument::Evaluated {
                        name: static_parameter.name,
                        value: fallback_expression,
                    })
                }
            },
            MissingStaticArgument::Function {
                node_id,
                owner_symbol,
                infer,
            } => self.missing_static_argument_for_function(
                module,
                node_id,
                owner_symbol,
                static_parameter,
                parameter_kind,
                infer,
                types,
            ),
        }
    }

    /// Validate a static argument against its declared type.
    pub(super) fn validate_static_argument(
        &self,
        module: &Module,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        parameter_kind: StaticParameterKind,
        resolved_argument: &StaticArgument,
        types: &mut TypeTable,
        infer: Option<&mut InferTable>,
    ) -> Option<LocalTypeId> {
        // validate type arguments against the declared bound
        if parameter_kind == StaticParameterKind::Type {
            let substitution_ty_id =
                self.convert_static_argument_to_type_id(resolved_argument, types);

            if let Some(infer) = infer {
                infer.push_constraint(Constraint::Subtype {
                    sub: substitution_ty_id,
                    sup: static_parameter.declared_type_id,
                    variance: None,
                });
            }

            // report unassignable type
            if !self.is_infer_var_type(static_parameter.declared_type_id, types)
                && !self.is_infer_var_type(substitution_ty_id, types)
                && self.check_is_type_assignable(
                    static_parameter.declared_type_id,
                    substitution_ty_id,
                    types,
                ) == Assignability::NotAssignable
            {
                self.error(AnalyzeError::UnassignableType {
                    node: error_node,
                    expected_ty: static_parameter.declared_type_id.into_global(module.id),
                    actual_ty: substitution_ty_id.into_global(module.id),
                });
            }

            return Some(substitution_ty_id);
        }

        // validate value arguments against the declared type
        if let StaticArgument::Evaluated { value, .. } = resolved_argument {
            let value_ty_id = self.convert_static_expression_to_type_id(value, types);
            if !self.is_infer_var_type(static_parameter.declared_type_id, types)
                && self.check_is_type_assignable(
                    static_parameter.declared_type_id,
                    value_ty_id,
                    types,
                ) == Assignability::NotAssignable
            {
                self.error(AnalyzeError::UnassignableType {
                    node: error_node,
                    expected_ty: static_parameter.declared_type_id.into_global(module.id),
                    actual_ty: value_ty_id.into_global(module.id),
                });
            }
        }

        None
    }

    /// Resolve static arguments for a type reference.
    pub(super) fn resolve_type_reference_static_arguments(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // skip non instantiable symbols
        if !self.is_instantiable_type_symbol(symbol) {
            return Ok(None);
        }

        // collect parameter symbols for the declaration
        let parameter_symbols =
            self.collect_static_parameter_symbols_for_symbol(module, symbol, tree, symbols);
        let parameter_symbols = match parameter_symbols {
            Some(parameter_symbols) => parameter_symbols,
            None => {
                // keep explicit arguments when the declaration is unavailable
                if let Some(static_arguments) = static_arguments
                    && !static_arguments.is_empty()
                {
                    return Ok(Some(static_arguments.to_vec()));
                }
                return Ok(None);
            }
        };
        if parameter_symbols.is_empty() {
            // keep explicit arguments when no parameters exist
            if let Some(static_arguments) = static_arguments
                && !static_arguments.is_empty()
            {
                return Ok(Some(static_arguments.to_vec()));
            }

            return Ok(None);
        }

        // gather static parameter metadata and kinds
        let static_parameters = self.collect_static_parameters_for_symbols(
            module,
            &parameter_symbols,
            tree,
            symbols,
            types,
        );
        let static_parameter_kinds =
            self.collect_static_parameter_kinds_for_symbol(symbol, &static_parameters, types);

        // map arguments to parameter slots
        let argument_values = static_arguments.unwrap_or(&[]);
        let assigned_arguments = self.assign_static_argument_values(
            module.id,
            node_id,
            argument_values,
            &static_parameters,
            tree,
        );

        // resolve arguments with defaults and fallbacks
        let mut resolved_arguments = Vec::with_capacity(static_parameters.len());
        for (index, static_parameter) in static_parameters.iter().enumerate() {
            // pick the parameter kind and assigned argument
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

            // resolve the argument value or synthesize a fallback
            let resolved_argument = self.resolve_static_argument_for_parameter(
                module,
                static_parameter,
                parameter_kind,
                assigned_argument,
                tree,
                symbols,
                types,
                MissingStaticArgument::TypeReference { node_id },
            )?;

            // validate type and value arguments against declared bounds
            self.validate_static_argument(
                module,
                error_node,
                static_parameter,
                parameter_kind,
                &resolved_argument,
                types,
                None,
            );

            resolved_arguments.push(resolved_argument);
        }

        Ok(Some(resolved_arguments))
    }

    /// Build type parameter substitutions for a type symbol.
    pub(super) fn build_type_parameter_substitutions_for_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        resolved_arguments: &[StaticArgument],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> HashMap<GlobalSymbolId, LocalTypeId> {
        // load static parameter symbols for the declaration
        let parameter_symbols =
            self.collect_static_parameter_symbols_for_symbol(module, symbol, tree, symbols);
        let Some(parameter_symbols) = parameter_symbols else {
            return HashMap::new();
        };
        if parameter_symbols.is_empty() {
            return HashMap::new();
        }

        // classify static parameters by usage
        let static_parameters = self.collect_static_parameters_for_symbols(
            module,
            &parameter_symbols,
            tree,
            symbols,
            types,
        );
        let static_parameter_kinds =
            self.collect_static_parameter_kinds_for_symbol(symbol, &static_parameters, types);

        // build substitutions for type parameters only
        let mut substitutions = HashMap::new();
        for (static_parameter, argument) in static_parameters.iter().zip(resolved_arguments.iter())
        {
            let parameter_kind = static_parameter_kinds
                .get(&static_parameter.symbol)
                .copied()
                .unwrap_or(StaticParameterKind::Type);
            if parameter_kind == StaticParameterKind::Type {
                let ty_id = self.convert_static_argument_to_type_id(argument, types);
                substitutions.insert(static_parameter.symbol, ty_id);
            }
        }

        substitutions
    }

    /// Evaluate a static argument as a type.
    pub(super) fn evaluate_static_argument_as_type(
        &self,
        module: &Module,
        argument_id: LocalNodeId<Argument>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        let argument = tree.get(argument_id);
        let argument_name = match argument {
            Argument::Named { name, .. } => Some(*name),
            _ => None,
        };

        let expression_id = argument.value();
        let ty_id =
            self.try_evaluate_expression_to_type(module, expression_id, tree, symbols, types)?;

        if matches!(types.get_type(ty_id), Type::Unevaluated { .. }) {
            return Ok(None);
        }

        Ok(Some(StaticArgument::Evaluated {
            name: argument_name,
            value: StaticExpression::Type { ty: ty_id },
        }))
    }

    /// Evaluate a static argument as a value.
    pub(super) fn evaluate_static_argument_as_value(
        &self,
        argument_id: LocalNodeId<Argument>,
        tree: &NodeTree,
    ) -> Option<StaticArgument> {
        let argument = tree.get(argument_id);
        let argument_name = match argument {
            Argument::Named { name, .. } => Some(*name),
            _ => None,
        };

        let expression_id = argument.value();
        let value = self.evaluate_static_expression_value(expression_id, tree)?;

        Some(StaticArgument::Evaluated {
            name: argument_name,
            value,
        })
    }

    /// Evaluate an expression into a static value expression.
    pub(super) fn evaluate_static_expression_value(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<StaticExpression> {
        let expression = tree.get(expression_id);
        match expression {
            Expression::ScalarLiteral { value } => Some(StaticExpression::ScalarLiteral {
                value: value.clone(),
            }),
            Expression::TypeLiteral { value } => Some(StaticExpression::TypeLiteral {
                value: value.clone(),
            }),
            Expression::Type { value } => Some(StaticExpression::Type { ty: *value }),
            Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let start_value = self.evaluate_static_expression_value(*start, tree)?;
                let end_value = self.evaluate_static_expression_value(*end, tree)?;
                Some(StaticExpression::RangeExpression {
                    start: Box::new(start_value),
                    end: Box::new(end_value),
                    is_inclusive: *is_inclusive,
                })
            }
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value(element.value(), tree)?;
                    values.push(value);
                }

                Some(StaticExpression::ArrayExpression { elements: values })
            }
            Expression::TupleExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value(element.value(), tree)?;
                    values.push(value);
                }

                Some(StaticExpression::TupleExpression { elements: values })
            }
            _ => None,
        }
    }

    /// Convert a static argument into a type id for substitution.
    pub(super) fn convert_static_argument_to_type_id(
        &self,
        argument: &StaticArgument,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let ty = match argument {
            StaticArgument::Evaluated { value, .. } => match value {
                StaticExpression::Type { ty } => return *ty,
                StaticExpression::TypeLiteral { value } => Type::TypeLiteral {
                    value: value.clone(),
                },
                StaticExpression::ScalarLiteral { value } => Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(value.clone()),
                },
                _ => Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
            },
            StaticArgument::Unevaluated { .. } => Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
        };

        types.insert_type(ty)
    }

    /// Convert a static expression into a type id for value checking.
    pub(super) fn convert_static_expression_to_type_id(
        &self,
        expression: &StaticExpression,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let ty = match expression {
            StaticExpression::ScalarLiteral { value } => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value.clone()),
            },
            StaticExpression::TypeLiteral { value } => Type::TypeLiteral {
                value: value.clone(),
            },
            _ => Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
        };

        types.insert_type(ty)
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

        // collect static parameter metadata
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

    /// Create a fallback static argument for function instantiation.
    pub(super) fn missing_static_argument_for_function(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_parameter: &StaticParameter,
        parameter_kind: StaticParameterKind,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        match parameter_kind {
            StaticParameterKind::Type => {
                let inferred_ty_id = if let Some(owner_symbol) = owner_symbol {
                    let scope = InferScope {
                        owner: owner_symbol,
                        function_id: Some(node_id.into_global(module.id)),
                    };

                    let infer_var_id =
                        infer.new_var(InferOrigin::TypeParameter(static_parameter.symbol), scope);
                    let infer_ty_id = types.insert_type(Type::InferVar { id: infer_var_id });
                    infer.bind_type(infer_var_id, infer_ty_id);

                    infer_ty_id
                } else {
                    self.error(AnalyzeError::MissingType {
                        node: node_id.into_global(module.id),
                    });
                    types.insert_type(Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    })
                };

                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: inferred_ty_id },
                })
            }
            StaticParameterKind::Value => {
                self.error(AnalyzeError::MissingType {
                    node: node_id.into_global(module.id),
                });

                let fallback_expression = node_id
                    .try_into_typed::<Expression>()
                    .map(|expression_id| StaticExpression::Unevaluated {
                        node: expression_id,
                    })
                    .unwrap_or(StaticExpression::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    });

                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: fallback_expression,
                })
            }
        }
    }

    /// Evaluate a static default expression for a parameter.
    pub(super) fn evaluate_static_default_argument(
        &self,
        parameter_kind: StaticParameterKind,
        name: Option<StringId>,
        default_expression: &GlobalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        let module = self.program.modules.get(default_expression.module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let symbols = module.dir().symbols.read();

        let value = match parameter_kind {
            StaticParameterKind::Type => {
                let ty_id = self.try_evaluate_expression_to_type(
                    &module,
                    default_expression.local_id,
                    &tree,
                    &symbols,
                    types,
                )?;

                if matches!(types.get_type(ty_id), Type::Unevaluated { .. }) {
                    StaticExpression::Unevaluated {
                        node: default_expression.local_id,
                    }
                } else {
                    StaticExpression::Type { ty: ty_id }
                }
            }
            StaticParameterKind::Value => {
                if let Some(value) =
                    self.evaluate_static_expression_value(default_expression.local_id, &tree)
                {
                    value
                } else {
                    StaticExpression::Unevaluated {
                        node: default_expression.local_id,
                    }
                }
            }
        };

        Ok(StaticArgument::Evaluated { name, value })
    }

    /// Substitute static parameter references in a type.
    pub(super) fn substitute_static_parameters_in_type(
        &self,
        ty_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        if let Some(mapped) = cache.get(&ty_id).copied() {
            return mapped;
        }

        let ty = types.get_type(ty_id).clone();
        let mapped = match ty {
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if let Some(mapped) = substitutions.get(&symbol).copied() {
                    mapped
                } else if let Some(static_arguments) = static_arguments {
                    let mut changed = false;
                    let mapped_arguments = static_arguments
                        .iter()
                        .map(|argument| {
                            let mapped = self.substitute_static_argument(
                                argument,
                                substitutions,
                                types,
                                cache,
                            );
                            if mapped != *argument {
                                changed = true;
                            }
                            mapped
                        })
                        .collect::<Vec<_>>();

                    if changed {
                        if !mapped_arguments.is_empty() {
                            self.register_instance_for_symbol(
                                symbol,
                                mapped_arguments.clone(),
                                types,
                            );
                        }

                        types.insert_type(Type::Reference {
                            symbol,
                            static_arguments: Some(mapped_arguments),
                        })
                    } else {
                        ty_id
                    }
                } else {
                    ty_id
                }
            }
            Type::Value { value } => {
                let original_value = value;
                let mapped_value = self.substitute_static_parameters_in_type(
                    original_value,
                    substitutions,
                    types,
                    cache,
                );
                if mapped_value == original_value {
                    ty_id
                } else {
                    types.insert_type(Type::Value {
                        value: mapped_value,
                    })
                }
            }
            Type::Unary { operator, right } => {
                let original_right = right;
                let mapped_right = self.substitute_static_parameters_in_type(
                    original_right,
                    substitutions,
                    types,
                    cache,
                );
                if mapped_right == original_right {
                    ty_id
                } else {
                    types.insert_type(Type::Unary {
                        operator,
                        right: mapped_right,
                    })
                }
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let original_left = left;
                let original_right = right;
                let mapped_left = self.substitute_static_parameters_in_type(
                    original_left,
                    substitutions,
                    types,
                    cache,
                );
                let mapped_right = self.substitute_static_parameters_in_type(
                    original_right,
                    substitutions,
                    types,
                    cache,
                );
                if mapped_left == original_left && mapped_right == original_right {
                    ty_id
                } else {
                    types.insert_type(Type::Binary {
                        left: mapped_left,
                        operator,
                        right: mapped_right,
                    })
                }
            }
            Type::Mutable { mutability, right } => {
                let original_right = right;
                let mapped_right = self.substitute_static_parameters_in_type(
                    original_right,
                    substitutions,
                    types,
                    cache,
                );
                if mapped_right == original_right {
                    ty_id
                } else {
                    types.insert_type(Type::Mutable {
                        mutability,
                        right: mapped_right,
                    })
                }
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let original_right = right;
                let mapped_right = self.substitute_static_parameters_in_type(
                    original_right,
                    substitutions,
                    types,
                    cache,
                );
                if mapped_right == original_right {
                    ty_id
                } else {
                    types.insert_type(Type::ValueOf {
                        mutability,
                        variance,
                        right: mapped_right,
                    })
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let original_right = right;
                let mapped_right = self.substitute_static_parameters_in_type(
                    original_right,
                    substitutions,
                    types,
                    cache,
                );
                if mapped_right == original_right {
                    ty_id
                } else {
                    types.insert_type(Type::ReferenceOf {
                        mutability,
                        variance,
                        right: mapped_right,
                    })
                }
            }
            Type::ArraySized { element, count } => {
                let original_element = element;
                let mapped_element = self.substitute_static_parameters_in_type(
                    original_element,
                    substitutions,
                    types,
                    cache,
                );
                if mapped_element == original_element {
                    ty_id
                } else {
                    types.insert_type(Type::ArraySized {
                        element: mapped_element,
                        count,
                    })
                }
            }
            Type::Array { element } => {
                let original_element = element;
                let mapped_element = original_element.map(|element| {
                    self.substitute_static_parameters_in_type(element, substitutions, types, cache)
                });
                if mapped_element == original_element {
                    ty_id
                } else {
                    types.insert_type(Type::Array {
                        element: mapped_element,
                    })
                }
            }
            Type::Tuple { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_static_parameters_in_type(
                            *element,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type(Type::Tuple {
                        elements: mapped_elements,
                    })
                } else {
                    ty_id
                }
            }
            Type::Object { fields } => {
                let mut changed = false;
                let mapped_fields = fields
                    .iter()
                    .map(|field| {
                        let mapped = self.substitute_static_parameters_in_type(
                            field.ty,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != field.ty {
                            changed = true;
                        }
                        TypeField {
                            key: field.key,
                            ty: mapped,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type(Type::Object {
                        fields: mapped_fields,
                    })
                } else {
                    ty_id
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                dynamic_parameters,
                return_type,
            } => {
                let mut changed = false;
                let mapped_parameters = dynamic_parameters
                    .iter()
                    .map(|parameter| {
                        let mapped = self.substitute_static_parameters_in_type(
                            *parameter,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *parameter {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_return = return_type.map(|return_type| {
                    let mapped = self.substitute_static_parameters_in_type(
                        return_type,
                        substitutions,
                        types,
                        cache,
                    );
                    if mapped != return_type {
                        changed = true;
                    }
                    mapped
                });
                if changed {
                    types.insert_type(Type::Function {
                        asynchrony,
                        cardinality,
                        static_parameters,
                        dynamic_parameters: mapped_parameters,
                        return_type: mapped_return,
                    })
                } else {
                    ty_id
                }
            }
            Type::Union { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_static_parameters_in_type(
                            *element,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type(Type::Union {
                        elements: mapped_elements,
                    })
                } else {
                    ty_id
                }
            }
            Type::Intersection { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_static_parameters_in_type(
                            *element,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type(Type::Intersection {
                        elements: mapped_elements,
                    })
                } else {
                    ty_id
                }
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::Error => ty_id,
        };

        cache.insert(ty_id, mapped);
        mapped
    }

    /// Substitute static parameters in a static argument.
    pub(super) fn substitute_static_argument(
        &self,
        argument: &StaticArgument,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped_value =
                    self.substitute_static_expression(value, substitutions, types, cache);
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped_value,
                }
            }
        }
    }

    /// Substitute static parameters in a static expression.
    pub(super) fn substitute_static_expression(
        &self,
        expression: &StaticExpression,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticExpression {
        match expression {
            StaticExpression::Unevaluated { .. } => expression.clone(),
            StaticExpression::ScalarLiteral { .. } => expression.clone(),
            StaticExpression::TypeLiteral { .. } => expression.clone(),
            StaticExpression::Type { ty } => StaticExpression::Type {
                ty: self.substitute_static_parameters_in_type(*ty, substitutions, types, cache),
            },
            StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                let mapped_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.substitute_static_argument(argument, substitutions, types, cache)
                        })
                        .collect::<Vec<_>>()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    static_arguments: mapped_arguments,
                }
            }
            StaticExpression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let mapped_start =
                    self.substitute_static_expression(start, substitutions, types, cache);
                let mapped_end =
                    self.substitute_static_expression(end, substitutions, types, cache);
                StaticExpression::RangeExpression {
                    start: Box::new(mapped_start),
                    end: Box::new(mapped_end),
                    is_inclusive: *is_inclusive,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_static_expression(element, substitutions, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ArrayExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::TupleExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_static_expression(element, substitutions, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::TupleExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                let mapped_properties = properties
                    .iter()
                    .map(|property| {
                        self.substitute_static_property(property, substitutions, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
        }
    }

    /// Substitute static parameters in a static property.
    pub(super) fn substitute_static_property(
        &self,
        property: &StaticProperty,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticProperty {
        match property {
            StaticProperty::Unevaluated { .. } => property.clone(),
            StaticProperty::Field {
                modifiers,
                key,
                value,
                default,
                symbol,
            } => {
                let mapped_value =
                    self.substitute_static_expression(value, substitutions, types, cache);
                let mapped_default = default.as_ref().map(|default| {
                    self.substitute_static_expression(default, substitutions, types, cache)
                });
                StaticProperty::Field {
                    modifiers: *modifiers,
                    key: *key,
                    value: mapped_value,
                    default: mapped_default,
                    symbol: *symbol,
                }
            }
            StaticProperty::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
            } => {
                let mapped_body =
                    self.substitute_static_expression(body, substitutions, types, cache);
                StaticProperty::Method {
                    modifiers: *modifiers,
                    key: *key,
                    signature: signature.clone(),
                    body: mapped_body,
                    symbol: *symbol,
                }
            }
        }
    }
}
