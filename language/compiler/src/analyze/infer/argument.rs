use std::collections::{HashMap, HashSet};

use super::parameter::{StaticParameter, StaticParameterKind};
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, Constraint, InferContext,
    InferOrigin, InferScope, InferTable,
};
use destack_dir::{
    Argument, Expression, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, NodeTree, StaticArgument, StaticExpression, StaticProperty,
    StringId, SymbolTable, Type, TypeField, TypeLiteral, TypeMappedParameter, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Inherited static arguments and substitutions for a type reference.
#[derive(Debug, Clone)]
pub(super) struct InheritedStaticArguments {
    /// Static arguments inherited from the receiver.
    pub(super) arguments: Vec<StaticArgument>,
    /// Substitutions for type parameters in inherited arguments.
    pub(super) substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Map static argument values to parameters by name and position.
    pub(super) fn assign_static_argument_values(
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

    /// Resolve inherited static arguments and substitutions for a receiver type.
    pub(super) fn resolve_inherited_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_id: LocalNodeIdAny,
        receiver_ty: &Type,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<InheritedStaticArguments> {
        let Type::Reference {
            symbol,
            static_arguments,
        } = receiver_ty
        else {
            return Ok(InheritedStaticArguments {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            });
        };

        let resolved = self.resolve_type_reference_static_arguments(
            module,
            profile,
            receiver_id,
            *symbol,
            static_arguments.as_deref(),
            options,
            tree,
            symbols,
            types,
        )?;
        let Some(resolved_arguments) = resolved else {
            return Ok(InheritedStaticArguments {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            });
        };

        // build type parameter substitutions
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            module,
            profile,
            *symbol,
            &resolved_arguments,
            tree,
            symbols,
            types,
        );

        Ok(InheritedStaticArguments {
            arguments: resolved_arguments,
            substitutions,
        })
    }

    /// Infer a dynamic argument value with contextual typing.
    pub(super) fn infer_argument(
        &self,
        module: &Module,
        argument_id: LocalNodeId<Argument>,
        expected_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let argument = tree.get(argument_id);

        // apply the expected type to the argument value
        let mut argument_ctx = ctx.fork().with_expected_type(expected_ty_id);

        match argument {
            Argument::Positional { value } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Named { name: _, value } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Labeled { label: _, value } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Spread { value } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
        }

        Ok(())
    }

    /// Resolve a static argument for a parameter.
    /// Returns `None` when no argument is provided and no default exists.
    pub(super) fn resolve_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        assigned_argument: Option<StaticArgument>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // resolve explicit argument when provided
        if let Some(argument) = assigned_argument {
            let resolved_argument = match (static_parameter.kind, argument) {
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

            return Ok(Some(resolved_argument));
        }

        // apply default expression when present
        if let Some(default_expression) = static_parameter.default_expression.as_ref() {
            return self
                .evaluate_static_default_argument(
                    profile,
                    static_parameter.kind,
                    static_parameter.name,
                    default_expression,
                    types,
                )
                .map(Some);
        }

        // no argument and no default - caller handles fallback
        Ok(None)
    }

    /// Validate a static argument against its declared type.
    pub(super) fn validate_static_argument(
        &self,
        module: &Module,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_static_argument: &StaticArgument,
        types: &mut TypeTable,
        infer: Option<&mut InferTable>,
        options: &AnalyzeOptions,
    ) -> Option<LocalTypeId> {
        // validate type arguments against the declared bound
        if static_parameter.kind == StaticParameterKind::Type {
            let substitution_ty_id =
                self.convert_static_argument_to_type_id(resolved_static_argument, types);

            if let Some(infer) = infer {
                infer.push_constraint(Constraint::Subtype {
                    sub_type: substitution_ty_id,
                    super_type: static_parameter.declared_type_id,
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
                    options,
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
        if let StaticArgument::Evaluated { value, .. } = resolved_static_argument {
            let ty = match value {
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
            let value_ty_id = types.insert_type(ty);
            if !self.is_infer_var_type(static_parameter.declared_type_id, types)
                && self.check_is_type_assignable(
                    static_parameter.declared_type_id,
                    value_ty_id,
                    types,
                    options,
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
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // skip non instantiable symbols
        if !self.is_instantiable_symbol(symbol) {
            return Ok(None);
        }

        // collect parameter symbols for the declaration
        let parameter_symbols =
            self.collect_static_parameter_symbols(module, symbol, profile, tree, symbols);
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

        // collect referenced symbols from the instance type
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        if let Some(ty_id) = types.get_instance_type_id(symbol) {
            self.collect_type_reference_symbols(
                ty_id,
                types,
                &mut referenced_symbols,
                &mut visited,
            );
        }

        // gather static parameter metadata with kinds
        let static_parameters: Vec<_> = parameter_symbols
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
            let resolved_argument = self
                .resolve_static_argument(
                    module,
                    profile,
                    static_parameter,
                    assigned_argument,
                    tree,
                    symbols,
                    types,
                )?
                .unwrap_or_else(|| {
                    // fallback for type references: unknown type
                    let fallback_value = match static_parameter.kind {
                        StaticParameterKind::Type => {
                            let unknown_ty_id = types.insert_type(Type::TypeLiteral {
                                value: TypeLiteral::Unknown,
                            });
                            StaticExpression::Type { ty: unknown_ty_id }
                        }
                        StaticParameterKind::Value => node_id
                            .try_into_typed::<Expression>()
                            .map(|expression_id| StaticExpression::Unevaluated {
                                node: expression_id,
                            })
                            .unwrap_or(StaticExpression::TypeLiteral {
                                value: TypeLiteral::Unknown,
                            }),
                    };
                    StaticArgument::Evaluated {
                        name: static_parameter.name,
                        value: fallback_value,
                    }
                });

            // validate type and value arguments against declared bounds
            self.validate_static_argument(
                module,
                error_node,
                static_parameter,
                &resolved_argument,
                types,
                None,
                options,
            );

            resolved_arguments.push(resolved_argument);
        }

        Ok(Some(resolved_arguments))
    }

    /// Build type parameter substitutions for a type symbol.
    pub(super) fn build_type_parameter_substitutions_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        resolved_arguments: &[StaticArgument],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> HashMap<GlobalSymbolId, LocalTypeId> {
        // collect static parameter symbols for the declaration
        let parameter_symbols =
            self.collect_static_parameter_symbols(module, symbol, profile, tree, symbols);
        let Some(parameter_symbols) = parameter_symbols else {
            return HashMap::new();
        };
        if parameter_symbols.is_empty() {
            return HashMap::new();
        }

        // collect referenced symbols from the instance type
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        if let Some(ty_id) = types.get_instance_type_id(symbol) {
            self.collect_type_reference_symbols(
                ty_id,
                types,
                &mut referenced_symbols,
                &mut visited,
            );
        }

        // collect static parameters with kinds
        let static_parameters: Vec<_> = parameter_symbols
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

        // build substitutions for type parameters only
        let mut substitutions = HashMap::new();
        for (static_parameter, argument) in static_parameters.iter().zip(resolved_arguments.iter())
        {
            if static_parameter.kind == StaticParameterKind::Type {
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

    /// Create a fallback static argument for function instantiation.
    pub(super) fn missing_static_argument_for_function(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: Option<GlobalSymbolId>,
        static_parameter: &StaticParameter,
        infer: &mut InferTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        match static_parameter.kind {
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
        profile: ProfileId,
        parameter_kind: StaticParameterKind,
        name: Option<StringId>,
        default_expression: &GlobalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        let module = self.program.modules.get(default_expression.module_id);
        let module = module.read();
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();

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
    pub(super) fn substitute_static_parameters(
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
            Type::This => ty_id,
            Type::Value { value } => {
                let mapped_value =
                    self.substitute_static_parameters(value, substitutions, types, cache);
                if mapped_value == value {
                    ty_id
                } else {
                    types.insert_type(Type::Value {
                        value: mapped_value,
                    })
                }
            }
            Type::Unary { operator, right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
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
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type(Type::Binary {
                        left: mapped_left,
                        operator,
                        right: mapped_right,
                    })
                }
            }
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                let mapped_then =
                    self.substitute_static_parameters(then_type, substitutions, types, cache);
                let mapped_else =
                    self.substitute_static_parameters(else_type, substitutions, types, cache);
                if mapped_left == left
                    && mapped_right == right
                    && mapped_then == then_type
                    && mapped_else == else_type
                {
                    ty_id
                } else {
                    types.insert_type(Type::Conditional {
                        left: mapped_left,
                        right: mapped_right,
                        then_type: mapped_then,
                        else_type: mapped_else,
                    })
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint = self.substitute_static_parameters(
                    parameter.constraint,
                    substitutions,
                    types,
                    cache,
                );
                let mapped_key_remap = parameter.key_remap.map(|key_remap| {
                    self.substitute_static_parameters(key_remap, substitutions, types, cache)
                });
                let mapped_value =
                    self.substitute_static_parameters(value, substitutions, types, cache);
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    ty_id
                } else {
                    let parameter = TypeMappedParameter {
                        name: parameter.name,
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type(Type::Mapped {
                        parameter,
                        modifiers,
                        value: mapped_value,
                    })
                }
            }
            Type::Index { left, index } => {
                let mapped_left =
                    self.substitute_static_parameters(left, substitutions, types, cache);
                let mapped_index =
                    self.substitute_static_parameters(index, substitutions, types, cache);
                if mapped_left == left && mapped_index == index {
                    ty_id
                } else {
                    types.insert_type(Type::Index {
                        left: mapped_left,
                        index: mapped_index,
                    })
                }
            }
            Type::TemplateLiteral { strings, spans } => {
                let mut changed = false;
                let mapped_spans = spans
                    .iter()
                    .map(|span| {
                        let mapped =
                            self.substitute_static_parameters(*span, substitutions, types, cache);
                        if mapped != *span {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type(Type::TemplateLiteral {
                        strings,
                        spans: mapped_spans,
                    })
                } else {
                    ty_id
                }
            }
            Type::Import { .. } => ty_id,
            Type::Infer { name, constraint } => {
                let mapped_constraint = constraint.map(|constraint| {
                    self.substitute_static_parameters(constraint, substitutions, types, cache)
                });
                if mapped_constraint == constraint {
                    ty_id
                } else {
                    types.insert_type(Type::Infer {
                        name,
                        constraint: mapped_constraint,
                    })
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let mapped_target = target.map(|target| {
                    self.substitute_static_parameters(target, substitutions, types, cache)
                });
                if mapped_target == target {
                    ty_id
                } else {
                    types.insert_type(Type::Predicate {
                        asserts,
                        subject,
                        target: mapped_target,
                    })
                }
            }
            Type::Mutable { mutability, right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
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
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
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
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
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
                let mapped_element =
                    self.substitute_static_parameters(element, substitutions, types, cache);
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type(Type::ArraySized {
                        element: mapped_element,
                        count,
                    })
                }
            }
            Type::Array { element } => {
                let mapped_element = element.map(|element| {
                    self.substitute_static_parameters(element, substitutions, types, cache)
                });
                if mapped_element == element {
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
                        let mapped = self.substitute_static_parameters(
                            element.ty,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != element.ty {
                            changed = true;
                        }
                        let mut element = element.clone();
                        element.ty = mapped;
                        element
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
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mut changed = false;
                let mapped_fields = fields
                    .iter()
                    .map(|field| {
                        let mapped = self.substitute_static_parameters(
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
                let mapped_call_signatures = call_signatures
                    .iter()
                    .map(|signature| {
                        let mapped = self.substitute_static_parameters(
                            *signature,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_construct_signatures = construct_signatures
                    .iter()
                    .map(|signature| {
                        let mapped = self.substitute_static_parameters(
                            *signature,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_index_signatures = index_signatures
                    .iter()
                    .map(|signature| {
                        let mapped_key = self.substitute_static_parameters(
                            signature.key_type,
                            substitutions,
                            types,
                            cache,
                        );
                        let mapped_value = self.substitute_static_parameters(
                            signature.value_type,
                            substitutions,
                            types,
                            cache,
                        );
                        if mapped_key != signature.key_type || mapped_value != signature.value_type
                        {
                            changed = true;
                        }
                        let mut signature = signature.clone();
                        signature.key_type = mapped_key;
                        signature.value_type = mapped_value;
                        signature
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type(Type::Object {
                        fields: mapped_fields,
                        call_signatures: mapped_call_signatures,
                        construct_signatures: mapped_construct_signatures,
                        index_signatures: mapped_index_signatures,
                    })
                } else {
                    ty_id
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                let mut changed = false;
                let mapped_this = this_parameter.map(|this_parameter| {
                    let mapped = self.substitute_static_parameters(
                        this_parameter,
                        substitutions,
                        types,
                        cache,
                    );
                    if mapped != this_parameter {
                        changed = true;
                    }
                    mapped
                });
                let mapped_parameters = dynamic_parameters
                    .iter()
                    .map(|parameter| {
                        let mapped = self.substitute_static_parameters(
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
                    let mapped =
                        self.substitute_static_parameters(return_type, substitutions, types, cache);
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
                        this_parameter: mapped_this,
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
                        let mapped = self.substitute_static_parameters(
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
                        let mapped = self.substitute_static_parameters(
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
                ty: self.substitute_static_parameters(*ty, substitutions, types, cache),
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
