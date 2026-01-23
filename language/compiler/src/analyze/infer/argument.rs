use std::collections::{HashMap, HashSet};

use crate::analyze::common::CanonicalSymbolMode;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    AnchoredGlobalNodeId, Argument, BindingKind, Constraint, Declaration, DynamicKey,
    EnumFieldValue, Expression, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId, InferOrigin,
    InferScope, InferTable, LocalNodeId, LocalNodeIdAny, LocalTypeId, Mutability, NodeTree,
    ScalarLiteral, StaticArgument, StaticExpression, StaticKey, StaticParameter,
    StaticParameterKind, StaticProperty, StringId, SymbolTable, SymbolType, Type, TypeElement,
    TypeField, TypeLiteral, TypeMappedParameter, TypeTable,
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
        profile_id: ProfileId,
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
                self.error(AnalyzeError::MissingType {
                    node: error_node.into_anchored(Some(profile_id)),
                });
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
                self.error(AnalyzeError::MissingType {
                    node: error_node.into_anchored(Some(profile_id)),
                });
                continue;
            };

            // reject duplicate assignments
            if assigned[target_index].is_some() {
                let error_node = match argument {
                    StaticArgument::Unevaluated { node } => node.into_global_any(module_id),
                    _ => node_id.into_global(module_id),
                };
                self.error(AnalyzeError::MissingType {
                    node: error_node.into_anchored(Some(profile_id)),
                });
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
        // resolve the receiver into a symbol and static arguments
        let Some((symbol, static_arguments)) = (match receiver_ty {
            // explicit reference
            Type::Reference {
                symbol,
                static_arguments,
            } => Some((*symbol, static_arguments.clone())),
            // possibly implicit reference via well known type
            _ => self
                .well_known_type(profile, receiver_ty, types)
                .and_then(|reference_ty| match reference_ty {
                    Type::Reference {
                        symbol,
                        static_arguments,
                    } => Some((symbol, static_arguments)),
                    _ => None,
                }),
        }) else {
            return Ok(InheritedStaticArguments {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            });
        };

        // resolve static arguments for the type reference
        let resolved = self.resolve_type_reference_static_arguments(
            module,
            profile,
            receiver_id,
            symbol,
            static_arguments.as_deref(),
            true,
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
            symbol,
            receiver_id,
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
            Argument::Positional { value, .. } => {
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
            Argument::Named { name: _, value, .. } => {
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
            Argument::Labeled {
                label: _, value, ..
            } => {
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
            Argument::Spread {
                label: _, value, ..
            } => {
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
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // resolve explicit argument when provided
        if let Some(argument) = assigned_argument {
            let resolved_argument = self.resolve_explicit_static_argument(
                module,
                profile,
                static_parameter,
                argument,
                treat_type_arguments_as_types,
                tree,
                symbols,
                types,
            )?;

            // normalize value arguments back into value expressions
            let resolved_argument = if static_parameter.kind == StaticParameterKind::Value {
                self.normalize_value_static_argument(resolved_argument, types)
            } else {
                resolved_argument
            };

            return Ok(Some(resolved_argument));
        }

        // default expression
        if let Some(default_expression) = static_parameter.default_expression.as_ref() {
            let resolved_argument = self.resolve_default_static_argument(
                module,
                profile,
                static_parameter,
                default_expression,
                treat_type_arguments_as_types,
                tree,
                symbols,
                types,
            )?;

            // normalize value defaults back into value expressions
            let resolved_argument = if static_parameter.kind == StaticParameterKind::Value {
                self.normalize_value_static_argument(resolved_argument, types)
            } else {
                resolved_argument
            };

            return Ok(Some(resolved_argument));
        }

        // no argument and no default (caller handles fallback)
        Ok(None)
    }

    /// Resolve an explicit static argument for a parameter.
    fn resolve_explicit_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        argument: StaticArgument,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        // resolve explicit arguments based on parameter kind
        let resolved_argument = match (static_parameter.kind, argument) {
            (StaticParameterKind::Type, StaticArgument::Unevaluated { node }) => {
                // prefer value literals when type arguments stay unconverted
                if !treat_type_arguments_as_types
                    && let Some(value) = self.evaluate_static_argument_as_value(
                        module, profile, node, tree, symbols, types,
                    )?
                {
                    return Ok(value);
                }

                let resolved = self.evaluate_static_argument_as_type(
                    module, profile, node, tree, symbols, types,
                )?;
                resolved.unwrap_or(StaticArgument::Unevaluated { node })
            }
            (StaticParameterKind::Value, StaticArgument::Unevaluated { node }) => {
                self.resolve_value_static_argument(module, profile, node, tree, symbols, types)?
            }
            (StaticParameterKind::Type, StaticArgument::Evaluated { name, value }) => {
                // preserve explicit values when type arguments stay unconverted
                if !treat_type_arguments_as_types {
                    StaticArgument::Evaluated {
                        name,
                        value: value.clone(),
                    }
                } else {
                    let argument = StaticArgument::Evaluated {
                        name,
                        value: value.clone(),
                    };
                    let ty_id = self.convert_static_argument_type(
                        &argument,
                        types.get_type_source(static_parameter.declared_type_id),
                        types,
                    );

                    StaticArgument::Evaluated {
                        name,
                        value: StaticExpression::Type { ty: ty_id },
                    }
                }
            }
            (_, argument) => argument,
        };

        Ok(resolved_argument)
    }

    /// Resolve a value-kind static argument to a concrete value or reference.
    fn resolve_value_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        node: LocalNodeId<Argument>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        // prefer static value evaluation first
        if let Some(value) = self
            .evaluate_static_argument_as_value_inner(module, profile, node, tree, symbols, types)?
        {
            return Ok(value);
        }

        // preserve static parameter references in value slots
        if let Some(StaticArgument::Evaluated { name, value }) =
            self.evaluate_static_argument_as_type(module, profile, node, tree, symbols, types)?
            && let StaticExpression::Type { ty } = value
            && let Type::Reference { symbol, .. } = types.get_type(ty)
            && self.symbol_is_static_parameter(module, profile, *symbol, symbols, types)
        {
            return Ok(StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty },
            });
        }

        // reject non-static values when no reference is available
        Ok(StaticArgument::Unevaluated { node })
    }

    /// Coerce value static arguments into literal value expressions when possible.
    pub(super) fn normalize_value_static_argument(
        &self,
        argument: StaticArgument,
        types: &TypeTable,
    ) -> StaticArgument {
        // normalize type-backed literal arguments into value expressions
        match argument {
            StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty },
            } => match types.get_type(ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(value),
                } => StaticArgument::Evaluated {
                    name,
                    value: StaticExpression::ScalarLiteral {
                        value: value.clone(),
                    },
                },
                Type::TypeLiteral { value } => StaticArgument::Evaluated {
                    name,
                    value: StaticExpression::TypeLiteral {
                        value: value.clone(),
                    },
                },
                _ => StaticArgument::Evaluated {
                    name,
                    value: StaticExpression::Type { ty },
                },
            },
            StaticArgument::Evaluated {
                name,
                value:
                    StaticExpression::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value),
                    },
            } => StaticArgument::Evaluated {
                name,
                value: StaticExpression::ScalarLiteral {
                    value: value.clone(),
                },
            },
            _ => argument,
        }
    }

    /// Replace value defaults that reference earlier value parameters.
    fn substitute_value_parameter_reference(
        &self,
        argument: StaticArgument,
        resolved_arguments: &HashMap<GlobalSymbolId, StaticArgument>,
        types: &TypeTable,
        error_node: AnchoredGlobalNodeId,
    ) -> AnalyzeResult<StaticArgument> {
        // walk through value references until a concrete value appears
        let mut current = argument;
        let mut visited = HashSet::new();

        loop {
            let StaticArgument::Evaluated { name, value } = current else {
                break;
            };
            let StaticExpression::Type { ty } = value else {
                // preserve non-reference values as-is
                current = StaticArgument::Evaluated { name, value };
                break;
            };
            let Type::Reference { symbol, .. } = types.get_type(ty) else {
                // stop once the value is not a static parameter reference
                current = StaticArgument::Evaluated { name, value };
                break;
            };
            // guard against circular static defaults
            if !visited.insert(*symbol) {
                return Err(AnalyzeError::CircularStaticArgument { node: error_node });
            }
            let Some(StaticArgument::Evaluated { value, .. }) = resolved_arguments.get(symbol)
            else {
                // keep the reference if no replacement is available yet
                current = StaticArgument::Evaluated { name, value };
                break;
            };

            current = StaticArgument::Evaluated {
                name,
                value: value.clone(),
            };
        }

        Ok(self.normalize_value_static_argument(current, types))
    }

    /// Resolve a default static argument for a parameter.
    fn resolve_default_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        default_expression: &GlobalNodeId<Expression>,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        // select the module context for the default expression
        if default_expression.module_id == module.id {
            return self.evaluate_static_default_argument(
                module,
                profile,
                static_parameter.kind,
                static_parameter.name,
                default_expression.local_id,
                treat_type_arguments_as_types,
                tree,
                symbols,
                types,
            );
        }

        // load the remote module context for the default expression
        let default_module = self.program.modules.get(default_expression.module_id);
        let default_module = default_module.read();
        let default_tree = default_module.dir(profile).tree.read();
        let default_symbols = default_module.dir(profile).symbols.read();
        self.evaluate_static_default_argument(
            &default_module,
            profile,
            static_parameter.kind,
            static_parameter.name,
            default_expression.local_id,
            treat_type_arguments_as_types,
            &default_tree,
            &default_symbols,
            types,
        )
    }

    /// Materialize a static type argument for validation.
    pub(super) fn materialize_static_type_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_static_argument: &StaticArgument,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // pre-evaluate local type aliases used as bounds
        if let Type::Reference { symbol, .. } = types.get_type(static_parameter.declared_type_id)
            && symbol.ty() == SymbolType::TypeAlias
        {
            self.unwrap_type_alias_reference(
                module,
                profile,
                static_parameter.declared_type_id,
                tree,
                symbols,
                types,
            )?;
        }

        // evaluate the declared bound when needed
        self.ensure_static_parameter_bound_evaluated(
            module,
            profile,
            static_parameter.declared_type_id,
            tree,
            symbols,
            types,
        )?;

        // re-evaluate unevaluated arguments before building the substitution type
        let resolved_argument =
            if let StaticArgument::Unevaluated { node } = resolved_static_argument {
                self.evaluate_static_argument_as_type(module, profile, *node, tree, symbols, types)?
                    .unwrap_or_else(|| resolved_static_argument.clone())
            } else {
                resolved_static_argument.clone()
            };

        // build the substitution type from the argument
        let substitution_ty_id =
            self.convert_static_argument_type(&resolved_argument, error_node.local_id, types);

        // ensure referenced instance types are available for validation
        self.ensure_reference_instance_types_for_type(
            module,
            profile,
            error_node.local_id,
            static_parameter.declared_type_id,
            types,
        )?;
        self.ensure_reference_instance_types_for_type(
            module,
            profile,
            error_node.local_id,
            substitution_ty_id,
            types,
        )?;

        Ok(substitution_ty_id)
    }

    /// Resolve the type of a static value argument for validation.
    fn static_value_argument_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        value: &StaticExpression,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // prefer static parameter constraints for referenced type expressions
        if let StaticExpression::Type { ty } = value
            && let Type::Reference { symbol, .. } = types.get_type(*ty)
            && let Some(constraint_id) = self.static_parameter_constraint_type(
                module,
                profile,
                *symbol,
                error_node.local_id,
                symbols,
                types,
            )
        {
            self.ensure_static_parameter_bound_evaluated(
                module,
                profile,
                constraint_id,
                tree,
                symbols,
                types,
            )?;
            return Ok(constraint_id);
        }

        // normalize static expression shapes into value types
        self.static_expression_value_type(module, profile, error_node, value, tree, symbols, types)
    }

    /// Check whether a scalar literal matches an enum constraint.
    fn enum_constraint_accepts_literal(
        &self,
        module: &Module,
        profile: ProfileId,
        constraint_ty_id: LocalTypeId,
        literal: &ScalarLiteral,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<bool> {
        // resolve the enum symbol for the constraint when possible
        let enum_symbol = self.enum_symbol_for_type(types.get_type(constraint_ty_id), types);
        let Some(enum_symbol) = enum_symbol else {
            return Ok(false);
        };

        // ensure remote enums are inferred before checking their values
        if enum_symbol.module_id != module.id {
            self.require_analyze_module_infer(enum_symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;
        }

        // select the module context for the enum
        let matches = if enum_symbol.module_id == module.id {
            self.enum_literal_matches_symbol(enum_symbol, literal, tree, symbols, types)
        } else {
            let remote_module = self.program.modules.get(enum_symbol.module_id);
            let remote_module = remote_module.read();
            let remote_dir = remote_module.dir(profile);
            let remote_tree = remote_dir.tree.read();
            let remote_symbols = remote_dir.symbols.read();
            let remote_types = remote_dir.types.read();
            self.enum_literal_matches_symbol(
                enum_symbol,
                literal,
                &remote_tree,
                &remote_symbols,
                &remote_types,
            )
        };

        Ok(matches)
    }

    /// Evaluate a static parameter bound when it is still unevaluated.
    fn ensure_static_parameter_bound_evaluated(
        &self,
        module: &Module,
        profile: ProfileId,
        bound_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        if matches!(types.get_type(bound_id), Type::Unevaluated(_)) {
            self.evaluate_type(module, profile, bound_id, tree, symbols, types)?;
        }
        Ok(())
    }

    /// Check whether a scalar literal matches an enum field value.
    fn enum_literal_matches_symbol(
        &self,
        enum_symbol: GlobalSymbolId,
        literal: &ScalarLiteral,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // ensure the symbol refers to an enum declaration
        let symbol_entry = symbols.get_symbol(enum_symbol.local_id);
        if symbol_entry.ty != SymbolType::Enum {
            return false;
        }

        // collect all enum declarations for the symbol
        let mut declaration_ids = Vec::new();
        if let Some(primary) = symbol_entry.primary_declaration {
            declaration_ids.push(primary);
        }
        if let Some(secondary) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary.iter().copied());
        }

        // scan enum fields for a matching literal value
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let Declaration::Enum { fields, .. } = tree.get(declaration_id) else {
                continue;
            };
            for field_id in fields {
                let field = tree.get(*field_id);
                let field_symbol = field.symbol.into_global(enum_symbol.module_id);
                let Some(value) = types.get_enum_field_value(field_symbol) else {
                    continue;
                };
                if self.enum_field_value_matches_literal(value, literal) {
                    return true;
                }
            }
        }

        false
    }

    /// Check whether an enum field value matches a scalar literal.
    fn enum_field_value_matches_literal(
        &self,
        value: EnumFieldValue,
        literal: &ScalarLiteral,
    ) -> bool {
        match (value, literal) {
            (EnumFieldValue::Int(value), ScalarLiteral::Integer(literal)) => value == *literal,
            (EnumFieldValue::String(value), ScalarLiteral::String(literal)) => value == *literal,
            _ => false,
        }
    }

    /// Convert a static expression into a value type for validation.
    fn static_expression_value_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        value: &StaticExpression,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // map scalar and type literal values directly
        let ty = match value {
            StaticExpression::ScalarLiteral { value } => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value.clone()),
            },
            StaticExpression::TypeLiteral { value } => Type::TypeLiteral {
                value: value.clone(),
            },
            StaticExpression::Type { ty } => {
                return Ok(*ty);
            }
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => self.static_expression_tuple_type(
                module, profile, error_node, elements, tree, symbols, types,
            )?,
            StaticExpression::ObjectExpression { properties } => self
                .static_expression_object_type(
                    module, profile, error_node, properties, tree, symbols, types,
                )?,
            StaticExpression::RangeExpression { .. }
            | StaticExpression::Declaration { .. }
            | StaticExpression::Unevaluated { .. } => Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
        };

        Ok(types.insert_type_from_any(ty, error_node.local_id))
    }

    /// Convert a static tuple/array expression into a tuple type.
    fn static_expression_tuple_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        elements: &[StaticExpression],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // map tuple literal expressions into tuple types
        let mut element_types = Vec::with_capacity(elements.len());
        for element in elements {
            let element_ty_id = self.static_expression_value_type(
                module, profile, error_node, element, tree, symbols, types,
            )?;
            element_types.push(TypeElement::new(element_ty_id));
        }
        Ok(Type::Tuple {
            elements: element_types,
            is_readonly: false,
        })
    }

    /// Convert a static object expression into an object type.
    fn static_expression_object_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        properties: &[StaticProperty],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // map object literal expressions into structural object types
        let mut fields = Vec::new();
        for property in properties {
            let StaticProperty::Field {
                modifiers,
                key,
                value,
                default: _,
                symbol: _,
            } = property
            else {
                continue;
            };
            let key = match *key {
                Some(DynamicKey::Name(name)) => Some(StaticKey::Name(name)),
                Some(DynamicKey::Number(name)) => Some(StaticKey::Number(name)),
                _ => None,
            };
            let Some(key) = key else {
                continue;
            };
            let field_ty_id = self.static_expression_value_type(
                module, profile, error_node, value, tree, symbols, types,
            )?;
            let is_optional = modifiers.is_some_and(|modifiers| {
                modifiers
                    .kind
                    .is_some_and(|kind| kind == BindingKind::Maybe)
            });
            let is_readonly = modifiers.is_some_and(|modifiers| {
                modifiers
                    .mutability
                    .is_some_and(|mutability| mutability == Mutability::Immutable)
            });
            fields.push(TypeField {
                key,
                ty: field_ty_id,
                is_optional,
                is_readonly,
            });
        }

        Ok(Type::Object {
            fields,
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        })
    }

    /// Validate a static argument against its declared type.
    pub(super) fn validate_static_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        static_parameter: &StaticParameter,
        resolved_static_argument: &StaticArgument,
        prepared_substitution: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: Option<&mut InferTable>,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // validate type arguments against the declared bound
        if static_parameter.kind == StaticParameterKind::Type {
            // coerce the argument into a type
            let substitution_ty_id = prepared_substitution.unwrap_or_else(|| {
                self.convert_static_argument_type(
                    resolved_static_argument,
                    error_node.local_id,
                    types,
                )
            });

            // skip bound validation in declaration modules
            if module.language_type.is_declaration() {
                return Ok(Some(substitution_ty_id));
            }

            // skip validation when the declared bound is still unevaluated
            if matches!(
                types.get_type(static_parameter.declared_type_id),
                Type::Unevaluated(_)
            ) {
                return Ok(Some(substitution_ty_id));
            }

            // substitute the argument into self referential bounds
            let expected_ty_id = {
                // bind the static parameter to the argument
                let mut substitutions = HashMap::new();
                substitutions.insert(static_parameter.symbol, substitution_ty_id);

                // apply the substitution to the declared bound
                let mut cache = HashMap::new();
                self.substitute_static_parameters(
                    static_parameter.declared_type_id,
                    &substitutions,
                    types,
                    &mut cache,
                )
            };

            // accept all arguments when the declared bound is unknown or any
            if matches!(
                types.get_type(expected_ty_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown | TypeLiteral::Any
                }
            ) {
                return Ok(Some(substitution_ty_id));
            }

            // register an inference constraint for the declared bound
            if let Some(infer) = infer {
                infer.push_constraint(Constraint::Subtype {
                    sub_type: substitution_ty_id,
                    super_type: expected_ty_id,
                    variance: None,
                });
            }

            // report unassignable type
            if !self.is_infer_var_type(static_parameter.declared_type_id, types)
                && !self.is_infer_var_type(substitution_ty_id, types)
                && self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    expected_ty_id,
                    substitution_ty_id,
                    types,
                    options,
                ) == Assignability::NotAssignable
            {
                // accept static parameter arguments when their constraints satisfy the bound
                if let Type::Reference { symbol, .. } = types.get_type(substitution_ty_id)
                    && let Some(constraint_ty_id) = self.static_parameter_constraint_type(
                        module,
                        profile,
                        *symbol,
                        error_node.local_id,
                        symbols,
                        types,
                    )
                    && self.constraint_satisfies_bound(
                        module,
                        profile,
                        expected_ty_id,
                        constraint_ty_id,
                        symbols,
                        types,
                        options,
                    )
                {
                    return Ok(Some(substitution_ty_id));
                }

                // allow type parameters that satisfy the expected bound via their constraints
                if let Some(constraint_ty_id) = self.materialize_static_argument_constraint_type(
                    module,
                    profile,
                    error_node,
                    substitution_ty_id,
                    symbols,
                    types,
                ) && self.constraint_satisfies_bound(
                    module,
                    profile,
                    expected_ty_id,
                    constraint_ty_id,
                    symbols,
                    types,
                    options,
                ) {
                    return Ok(Some(substitution_ty_id));
                }

                // report failed bound validation
                self.error(AnalyzeError::UnassignableType {
                    node: error_node.into_anchored(Some(profile)),
                    expected_ty: expected_ty_id.into_global(module.id),
                    actual_ty: substitution_ty_id.into_global(module.id),
                });
                return Ok(Some(
                    types.insert_type_from_any(Type::Error, error_node.local_id),
                ));
            }

            // accept validated type arguments
            return Ok(Some(substitution_ty_id));
        }

        // skip value validation in declaration modules
        if module.language_type.is_declaration() {
            return Ok(None);
        }

        // validate value arguments against the declared type
        if static_parameter.kind == StaticParameterKind::Value
            && matches!(resolved_static_argument, StaticArgument::Unevaluated { .. })
        {
            self.error(AnalyzeError::NonStaticArgument {
                node: error_node.into_anchored(Some(profile)),
            });
            return Ok(Some(
                types.insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        let StaticArgument::Evaluated { value, .. } = resolved_static_argument else {
            return Ok(None);
        };

        // reject non static value arguments
        if static_parameter.kind == StaticParameterKind::Value
            && !self.static_value_argument_is_static(value, types)
        {
            self.error(AnalyzeError::NonStaticArgument {
                node: error_node.into_anchored(Some(profile)),
            });
            return Ok(Some(
                types.insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        // reject not assignable value arguments
        let value_ty_id = self
            .static_value_argument_type(module, profile, error_node, value, tree, symbols, types)?;

        // accept enum literal values that match enum constraints
        if let StaticExpression::ScalarLiteral { value: literal } = value
            && self.enum_constraint_accepts_literal(
                module,
                profile,
                static_parameter.declared_type_id,
                literal,
                tree,
                symbols,
                types,
            )?
        {
            return Ok(Some(value_ty_id));
        }

        if !self.is_infer_var_type(static_parameter.declared_type_id, types)
            && self.is_type_assignable(
                module,
                profile,
                symbols,
                static_parameter.declared_type_id,
                value_ty_id,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            // report unassignable value arguments
            self.error(AnalyzeError::UnassignableType {
                node: error_node.into_anchored(Some(profile)),
                expected_ty: static_parameter.declared_type_id.into_global(module.id),
                actual_ty: value_ty_id.into_global(module.id),
            });
            return Ok(Some(
                types.insert_type_from_any(Type::Error, error_node.local_id),
            ));
        }

        Ok(Some(value_ty_id))
    }

    /// Check whether a static value argument is a static expression.
    fn static_value_argument_is_static(&self, value: &StaticExpression, types: &TypeTable) -> bool {
        // classify static expressions by evaluation state
        match value {
            StaticExpression::Unevaluated { .. } => false,
            StaticExpression::ScalarLiteral { .. } => true,
            StaticExpression::TypeLiteral { value } => !matches!(value, TypeLiteral::Unknown),
            StaticExpression::Type { ty } => !matches!(
                types.get_type(*ty),
                Type::Unevaluated(_)
                    | Type::TypeLiteral {
                        value: TypeLiteral::Unknown
                    }
            ),
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments.as_ref().is_none_or(|arguments| {
                arguments.iter().all(|argument| match argument {
                    StaticArgument::Unevaluated { .. } => false,
                    StaticArgument::Evaluated { value, .. } => {
                        self.static_value_argument_is_static(value, types)
                    }
                })
            }),
            StaticExpression::RangeExpression { start, end, .. } => {
                self.static_value_argument_is_static(start, types)
                    && self.static_value_argument_is_static(end, types)
            }
            StaticExpression::ArrayExpression { elements } => elements
                .iter()
                .all(|element| self.static_value_argument_is_static(element, types)),
            StaticExpression::TupleExpression { elements } => elements
                .iter()
                .all(|element| self.static_value_argument_is_static(element, types)),
            StaticExpression::ObjectExpression { properties } => properties
                .iter()
                .all(|property| self.static_property_is_static(property, types)),
        }
    }

    /// Check whether a static property is fully static.
    fn static_property_is_static(&self, property: &StaticProperty, types: &TypeTable) -> bool {
        // accept property values only when they are fully static
        match property {
            StaticProperty::Unevaluated { .. } => false,
            StaticProperty::Field { value, default, .. } => {
                self.static_value_argument_is_static(value, types)
                    && default
                        .as_ref()
                        .is_none_or(|value| self.static_value_argument_is_static(value, types))
            }
            StaticProperty::Method { body, .. } => {
                self.static_value_argument_is_static(body, types)
            }
        }
    }

    /// Resolve the target argument mapping for an extension declaration.
    pub(super) fn extension_target_argument_mapping(
        &self,
        extension_symbol: GlobalSymbolId,
        extension_parameters: &[GlobalSymbolId],
        profile: ProfileId,
    ) -> Option<Vec<usize>> {
        // load the extension declaration
        let module = self.program.modules.get(extension_symbol.module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        let symbol_entry = symbols.get_symbol(extension_symbol.local_id);
        let declaration_id = symbol_entry
            .primary_declaration?
            .try_into_local_typed::<Declaration>()
            .ok()?;
        let declaration = tree.get(declaration_id);
        let Declaration::Extension { target_type, .. } = declaration else {
            return None;
        };

        // read the target type arguments
        let target_expression = tree.get(*target_type);
        let static_arguments = match target_expression {
            Expression::LocalReference {
                static_arguments, ..
            }
            | Expression::ModuleReference {
                static_arguments, ..
            }
            | Expression::GlobalReference {
                static_arguments, ..
            } => static_arguments.as_ref(),
            _ => None,
        }?;

        // map target arguments to extension parameter indices
        let mut mapping = Vec::with_capacity(static_arguments.len());
        for argument_id in static_arguments {
            let argument = tree.get(*argument_id);
            let expression_id = argument.value();
            let expression = tree.get(expression_id);
            let target_symbol = match expression {
                Expression::LocalReference { target_symbol, .. }
                | Expression::ModuleReference { target_symbol, .. }
                | Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
                _ => None,
            }?;

            let parameter_index = extension_parameters
                .iter()
                .position(|parameter_symbol| *parameter_symbol == target_symbol)?;
            mapping.push(parameter_index);
        }

        Some(mapping)
    }

    /// Resolve a static argument constraint for validation.
    fn materialize_static_argument_constraint_type(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        argument_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // collect referenced static parameters
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_type_reference_symbols(
            argument_ty_id,
            types,
            &mut referenced_symbols,
            &mut visited,
        );

        // materialize referenced constraints
        let mut substitutions = HashMap::new();
        let mut visiting_symbols = HashSet::new();
        for symbol in referenced_symbols {
            let constraint_id = self.materialize_static_parameter_constraint(
                module,
                profile,
                error_node,
                symbol,
                symbols,
                types,
                &mut visiting_symbols,
            );
            if let Some(constraint_id) = constraint_id {
                substitutions.insert(symbol, constraint_id);
            }
        }

        // skip when no substitutions are needed
        if substitutions.is_empty() {
            return None;
        }

        // apply nested constraint substitutions
        let mut cache = HashMap::new();
        Some(self.substitute_static_parameters(argument_ty_id, &substitutions, types, &mut cache))
    }

    /// Materialize a static parameter constraint by substituting nested constraints.
    fn materialize_static_parameter_constraint(
        &self,
        module: &Module,
        profile: ProfileId,
        error_node: GlobalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visiting: &mut HashSet<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        // avoid recursive constraint expansion
        if !visiting.insert(symbol) {
            return None;
        }

        // resolve the declared constraint type
        let constraint_id = self.static_parameter_constraint_type(
            module,
            profile,
            symbol,
            error_node.local_id,
            symbols,
            types,
        );
        let Some(constraint_id) = constraint_id else {
            visiting.remove(&symbol);
            return None;
        };

        // collect nested static parameters in the constraint
        let mut referenced_symbols = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_type_reference_symbols(
            constraint_id,
            types,
            &mut referenced_symbols,
            &mut visited,
        );

        // materialize nested constraints
        let mut substitutions = HashMap::new();
        for referenced_symbol in referenced_symbols {
            if referenced_symbol == symbol {
                continue;
            }
            let nested_constraint = self.materialize_static_parameter_constraint(
                module,
                profile,
                error_node,
                referenced_symbol,
                symbols,
                types,
                visiting,
            );
            if let Some(nested_constraint) = nested_constraint {
                substitutions.insert(referenced_symbol, nested_constraint);
            }
        }

        // stop recursive tracking for this symbol
        visiting.remove(&symbol);

        // return the raw constraint when nothing is substituted
        if substitutions.is_empty() {
            return Some(constraint_id);
        }

        // apply nested substitutions
        let mut cache = HashMap::new();
        Some(self.substitute_static_parameters(constraint_id, &substitutions, types, &mut cache))
    }

    /// Resolve static arguments for a type reference.
    pub(crate) fn resolve_type_reference_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // align the symbol id with the stored symbol type
        let symbol = self.typed_symbol_id(module, profile, symbol, symbols);

        // canonicalize import targets while preserving alias identity
        let symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::PreserveAliases,
        );
        let symbol = self.merged_type_symbol_id(module, symbols, profile, symbol);
        let mut symbol = self.typed_symbol_id(module, profile, symbol, symbols);

        // follow alias targets when import bindings are untyped
        if symbol.ty() == SymbolType::Void {
            symbol = self.canonical_symbol_id(
                module,
                symbols,
                profile,
                symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            symbol = self.typed_symbol_id(module, profile, symbol, symbols);
        }

        // ensure remote declarations are available before resolving arguments
        if symbol.module_id != module.id {
            self.require_analyze_module_declare(symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;
        }

        // treat type arguments as types for type references
        let treat_type_arguments_as_types = true;

        // skip non instantiable symbols
        if !self.is_instantiable_symbol(symbol) && symbol.ty() != SymbolType::Extension {
            return Ok(None);
        }

        // reuse resolved arguments when an instance is already registered for this node
        let node_global_id = node_id.into_global(module.id);
        if let Some(instance_id) = types.get_instance_for_node(node_global_id) {
            let instance = types.get_instance(instance_id);
            if instance.symbol_id == symbol {
                if instance.static_arguments.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(instance.static_arguments.clone()));
            }
        }

        // guard against recursive resolution on the same reference
        let argument_slice = static_arguments.unwrap_or(&[]);
        if types.is_static_argument_resolution_in_progress(symbol, argument_slice) {
            if !argument_slice.is_empty() {
                return Ok(Some(argument_slice.to_vec()));
            }
            return Ok(None);
        }

        // mark resolution as in progress for this argument set
        let argument_snapshot = argument_slice.to_vec();
        types.mark_static_argument_resolution_in_progress(symbol, argument_snapshot.clone());

        let result = self.resolve_type_reference_static_arguments_inner(
            module,
            profile,
            node_id,
            symbol,
            static_arguments,
            validate_static_argument_bounds,
            options,
            treat_type_arguments_as_types,
            tree,
            symbols,
            types,
        );

        // clear the in progress marker
        types.clear_static_argument_resolution_in_progress(symbol, &argument_snapshot);
        result
    }

    /// Resolve static arguments for a type reference.
    fn resolve_type_reference_static_arguments_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        validate_static_argument_bounds: bool,
        options: &AnalyzeOptions,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // ensure remote declarations are resolved before reading defaults
        if symbol.module_id != module.id {
            self.require_resolve_module_direct(symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;
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

        // gather static parameter metadata for the declaration
        let static_parameters: Vec<_> = parameter_symbols
            .iter()
            .map(|symbol_id| {
                self.collect_static_parameter(
                    module, *symbol_id, node_id, profile, tree, symbols, types,
                )
            })
            .collect();

        // map arguments to parameter slots
        let argument_values = static_arguments.unwrap_or(&[]);
        let assigned_arguments = self.assign_static_argument_values(
            module.id,
            profile,
            node_id,
            argument_values,
            &static_parameters,
            tree,
        );
        // resolve arguments with defaults and fallbacks
        let mut resolved_arguments = Vec::with_capacity(static_parameters.len());
        let mut resolved_argument_map = HashMap::new();
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
            let mut resolved_argument = self
                .resolve_static_argument(
                    module,
                    profile,
                    static_parameter,
                    assigned_argument,
                    treat_type_arguments_as_types,
                    tree,
                    symbols,
                    types,
                )?
                .unwrap_or_else(|| {
                    // fallback for type references: unknown type
                    let fallback_value = match static_parameter.kind {
                        StaticParameterKind::Type => {
                            let unknown_ty_id = types.insert_type_from_any(
                                Type::TypeLiteral {
                                    value: TypeLiteral::Unknown,
                                },
                                error_node.local_id,
                            );
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

            // substitute earlier value parameters in defaults
            if static_parameter.kind == StaticParameterKind::Value {
                resolved_argument = self.substitute_value_parameter_reference(
                    resolved_argument,
                    &resolved_argument_map,
                    types,
                    error_node.into_anchored(Some(profile)),
                )?;
            }

            // inherit value constraints when passing a static parameter through
            if static_parameter.kind == StaticParameterKind::Value {
                let referenced_symbol = match &resolved_argument {
                    StaticArgument::Evaluated {
                        value: StaticExpression::Type { ty },
                        ..
                    } => match types.get_type(*ty) {
                        Type::Reference { symbol, .. } => Some(*symbol),
                        _ => None,
                    },
                    _ => None,
                };

                if let Some(referenced_symbol) = referenced_symbol {
                    let constraint_id = self.static_parameter_constraint_type(
                        module,
                        profile,
                        referenced_symbol,
                        error_node.local_id,
                        symbols,
                        types,
                    );

                    if let Some(constraint_id) = constraint_id
                        && matches!(
                            types.get_type(constraint_id),
                            Type::TypeLiteral {
                                value: TypeLiteral::Unknown
                            }
                        )
                    {
                        if matches!(
                            types.get_type(static_parameter.declared_type_id),
                            Type::Unevaluated(_)
                        ) {
                            self.evaluate_type(
                                module,
                                profile,
                                static_parameter.declared_type_id,
                                tree,
                                symbols,
                                types,
                            )?;
                        }

                        if !matches!(
                            types.get_type(static_parameter.declared_type_id),
                            Type::TypeLiteral {
                                value: TypeLiteral::Unknown
                            }
                        ) {
                            types.set_static_parameter_constraint_type(
                                referenced_symbol,
                                static_parameter.declared_type_id,
                            );
                        }
                    }
                }
            }

            // materialized type argument validation when needed
            let materialized_substitution = if validate_static_argument_bounds
                && static_parameter.kind == StaticParameterKind::Type
            {
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

            // validate type and value arguments against declared bounds
            let validated_type = if validate_static_argument_bounds {
                self.validate_static_argument(
                    module,
                    profile,
                    error_node,
                    static_parameter,
                    &resolved_argument,
                    materialized_substitution,
                    tree,
                    symbols,
                    types,
                    None,
                    options,
                )?
            } else {
                None
            };

            // update resolved arguments with validated substitutions
            if let Some(substitution_ty_id) = validated_type {
                let argument_name = match &resolved_argument {
                    StaticArgument::Evaluated { name, .. } => *name,
                    StaticArgument::Unevaluated { .. } => None,
                };
                let preserves_reference = match &resolved_argument {
                    StaticArgument::Evaluated {
                        value: StaticExpression::Type { ty },
                        ..
                    } => match types.get_type(*ty) {
                        Type::Reference { symbol, .. } => self
                            .symbol_is_static_parameter(module, profile, *symbol, symbols, types),
                        _ => false,
                    },
                    _ => false,
                };
                if matches!(types.get_type(substitution_ty_id), Type::Error)
                    || static_parameter.kind == StaticParameterKind::Type
                {
                    resolved_argument = StaticArgument::Evaluated {
                        name: argument_name,
                        value: StaticExpression::Type {
                            ty: substitution_ty_id,
                        },
                    };
                } else if static_parameter.kind == StaticParameterKind::Value
                    && !preserves_reference
                {
                    let replacement = StaticArgument::Evaluated {
                        name: argument_name,
                        value: StaticExpression::Type {
                            ty: substitution_ty_id,
                        },
                    };
                    resolved_argument = self.normalize_value_static_argument(replacement, types);
                }
            }

            resolved_argument_map.insert(static_parameter.symbol, resolved_argument.clone());
            resolved_arguments.push(resolved_argument);
        }

        // record resolved arguments for this reference instance
        if !resolved_arguments.is_empty() && self.is_instantiable_symbol(symbol) {
            let node_global_id = node_id.into_global(module.id);
            self.register_instance_for_node(
                node_global_id,
                symbol,
                resolved_arguments.clone(),
                types,
            );
        }

        Ok(Some(resolved_arguments))
    }

    /// Build type parameter substitutions for a type symbol.
    pub(crate) fn build_type_parameter_substitutions_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
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

        // collect static parameters for the declaration
        let static_parameters: Vec<_> = parameter_symbols
            .iter()
            .map(|symbol_id| {
                self.collect_static_parameter(
                    module, *symbol_id, source_id, profile, tree, symbols, types,
                )
            })
            .collect();

        // build substitutions for type and value parameters
        let mut substitutions = HashMap::new();
        for (static_parameter, argument) in static_parameters.iter().zip(resolved_arguments.iter())
        {
            let ty_id = self.convert_static_argument_type(
                argument,
                types.get_type_source(static_parameter.declared_type_id),
                types,
            );
            substitutions.insert(static_parameter.symbol, ty_id);
        }

        let (symbol_key, symbol_space) = if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            (symbol_entry.key, symbol_entry.space)
        } else {
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();
            let symbol_entry = remote_symbols.get_symbol(symbol.local_id);
            (symbol_entry.key, symbol_entry.space)
        };
        if let Some(symbol_key) = symbol_key
            && let Some(ambient_symbols) =
                self.get_ambient_lib_symbol_sources_for_merge(profile, symbol_key, symbol_space)
        {
            let resolved_parameter_types = parameter_symbols
                .iter()
                .map(|symbol| substitutions.get(symbol).copied())
                .collect::<Vec<_>>();

            if resolved_parameter_types.iter().any(|ty| ty.is_some()) {
                for ambient_symbol in ambient_symbols {
                    if ambient_symbol == symbol {
                        continue;
                    }

                    let Some(other_parameters) = self.collect_static_parameter_symbols(
                        module,
                        ambient_symbol,
                        profile,
                        tree,
                        symbols,
                    ) else {
                        continue;
                    };

                    for (index, parameter_symbol) in other_parameters.iter().enumerate() {
                        let Some(Some(mapped)) = resolved_parameter_types.get(index) else {
                            continue;
                        };
                        substitutions.entry(*parameter_symbol).or_insert(*mapped);
                    }
                }
            }
        }

        substitutions
    }

    /// Evaluate a static argument as a type.
    pub(super) fn evaluate_static_argument_as_type(
        &self,
        module: &Module,
        profile: ProfileId,
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

        // try evaluate expression as a type
        let expression_id = argument.value();
        let ty_id = self.try_evaluate_expression_to_type(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            true,
            true,
        )?;
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
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        self.evaluate_static_argument_as_value_inner(
            module,
            profile,
            argument_id,
            tree,
            symbols,
            types,
        )
    }

    /// Evaluate a static argument as a value.
    fn evaluate_static_argument_as_value_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // capture argument name for reuse in evaluated form
        let argument = tree.get(argument_id);
        let argument_name = match argument {
            Argument::Named { name, .. } => Some(*name),
            _ => None,
        };

        // evaluate the expression into a static value when possible
        let expression_id = argument.value();
        let value = self.evaluate_static_expression_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            None,
        )?;
        let value = if let Some(value) = value {
            value
        } else if let Some(enum_symbol) =
            self.enum_symbol_for_member_expression(module, profile, expression_id, tree, symbols)?
        {
            let ty = types.insert_type_from_any(
                Type::Reference {
                    symbol: enum_symbol,
                    static_arguments: None,
                },
                expression_id.into_any(),
            );
            StaticExpression::Type { ty }
        } else {
            return Ok(None);
        };

        Ok(Some(StaticArgument::Evaluated {
            name: argument_name,
            value,
        }))
    }

    /// Resolve the enum symbol for an enum member expression.
    fn enum_symbol_for_member_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let Expression::Member {
            name,
            static_arguments,
            left,
        } = tree.get(expression_id)
        else {
            return Ok(None);
        };
        if static_arguments.is_some() {
            return Ok(None);
        }

        let Some(enum_symbol) = self
            .reference_symbol_for_expression(module, *left, profile, tree, symbols)
            .filter(|symbol| symbol.ty() == SymbolType::Enum)
        else {
            return Ok(None);
        };

        if self
            .enum_field_symbol_for_name(module, profile, enum_symbol, *name, tree, symbols)
            .is_some()
        {
            Ok(Some(enum_symbol))
        } else {
            Ok(None)
        }
    }

    /// Convert a static argument into a type id for substitution.
    pub(crate) fn convert_static_argument_type(
        &self,
        argument: &StaticArgument,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // convert evaluated static expressions into type ids
        let convert_value = |value: &StaticExpression,
                             source_id: LocalNodeIdAny,
                             types: &mut TypeTable,
                             compiler: &Compiler| {
            match value {
                StaticExpression::Type { ty } => *ty,
                StaticExpression::TypeLiteral { value } => types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: value.clone(),
                    },
                    source_id,
                ),
                StaticExpression::ScalarLiteral { value } => types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value.clone()),
                    },
                    source_id,
                ),
                StaticExpression::ArrayExpression { elements }
                | StaticExpression::TupleExpression { elements } => {
                    let mut element_types = Vec::with_capacity(elements.len());
                    for element in elements {
                        let element_ty = compiler.convert_static_argument_type(
                            &StaticArgument::Evaluated {
                                name: None,
                                value: element.clone(),
                            },
                            source_id,
                            types,
                        );
                        element_types.push(TypeElement::new(element_ty));
                    }
                    types.insert_type_from_any(
                        Type::Tuple {
                            elements: element_types,
                            is_readonly: false,
                        },
                        source_id,
                    )
                }
                _ => types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    source_id,
                ),
            }
        };

        match argument {
            StaticArgument::Evaluated { value, .. } => convert_value(value, source_id, types, self),
            StaticArgument::Unevaluated { .. } => types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                source_id,
            ),
        }
    }

    /// Create a fallback static argument for function instantiation.
    pub(super) fn missing_static_argument_for_function(
        &self,
        module: &Module,
        profile: ProfileId,
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
                    let infer_ty_id =
                        types.insert_type_from_any(Type::InferVar { id: infer_var_id }, node_id);
                    infer.bind_type(infer_var_id, infer_ty_id);

                    infer_ty_id
                } else {
                    self.error(AnalyzeError::MissingType {
                        node: node_id.into_global(module.id).into_anchored(Some(profile)),
                    });
                    types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        },
                        node_id,
                    )
                };

                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: inferred_ty_id },
                })
            }
            StaticParameterKind::Value => {
                self.error(AnalyzeError::MissingStaticArgument {
                    node: node_id.into_global(module.id).into_anchored(Some(profile)),
                });
                let inferred_ty_id = types.insert_type_from_any(Type::Error, node_id);
                Ok(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: StaticExpression::Type { ty: inferred_ty_id },
                })
            }
        }
    }

    /// Infer a value static argument from matching dynamic arguments.
    pub(super) fn infer_static_argument_from_dynamic_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        static_parameter: &StaticParameter,
        dynamic_parameters: &[LocalTypeId],
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticArgument>> {
        // require a value parameter and matching argument list
        if static_parameter.kind != StaticParameterKind::Value {
            return Ok(None);
        }
        if dynamic_parameters.len() != dynamic_arguments.len() {
            return Ok(None);
        }

        // scan positional arguments for direct static parameter references
        for (param_ty_id, argument_id) in dynamic_parameters.iter().zip(dynamic_arguments.iter()) {
            let param_ty_id = self.unwrap_type_value(*param_ty_id, types);
            let param_ty = types.get_type(param_ty_id).clone();

            let expression_id = tree.get(*argument_id).value();
            let value = self.evaluate_static_expression_value(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                None,
            )?;
            let value = if let Some(value) = value {
                value
            } else if let Some(enum_symbol) = self.enum_symbol_for_member_expression(
                module,
                profile,
                expression_id,
                tree,
                symbols,
            )? {
                let ty = types.insert_type_from_any(
                    Type::Reference {
                        symbol: enum_symbol,
                        static_arguments: None,
                    },
                    expression_id.into_any(),
                );
                StaticExpression::Type { ty }
            } else {
                continue;
            };

            if !self.static_value_argument_is_static(&value, types) {
                continue;
            }

            // handle direct static parameter references
            if let Type::Reference { symbol, .. } = &param_ty
                && *symbol == static_parameter.symbol
            {
                return Ok(Some(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value,
                }));
            }

            // infer array sizes from literal arguments
            if let Type::ArraySized { count, .. } = &param_ty
                && let Some(target_symbol) =
                    self.reference_symbol_for_expression(module, *count, profile, tree, symbols)
                && target_symbol == static_parameter.symbol
            {
                let elements = match &value {
                    StaticExpression::ArrayExpression { elements }
                    | StaticExpression::TupleExpression { elements } => elements,
                    _ => continue,
                };
                let count_value = StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(elements.len() as i64),
                };
                return Ok(Some(StaticArgument::Evaluated {
                    name: static_parameter.name,
                    value: count_value,
                }));
            }
        }

        Ok(None)
    }

    /// Check whether a constraint satisfies a declared bound.
    fn constraint_satisfies_bound(
        &self,
        module: &Module,
        profile: ProfileId,
        expected_ty_id: LocalTypeId,
        constraint_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        // skip validation when the declared bound still depends on static parameters
        let mut visited = HashSet::new();
        if self.type_contains_static_parameters(
            module,
            profile,
            expected_ty_id,
            symbols,
            types,
            &mut visited,
        ) {
            return true;
        }

        // skip validation when the constraint still depends on static parameters
        visited.clear();
        if self.type_contains_static_parameters(
            module,
            profile,
            constraint_ty_id,
            symbols,
            types,
            &mut visited,
        ) {
            return true;
        }

        // accept arguments whose constraints satisfy the expected bound
        self.is_type_assignable(
            module,
            profile,
            symbols,
            expected_ty_id,
            constraint_ty_id,
            types,
            options,
        )
        .is_assignable()
    }

    /// Evaluate a static default expression for a parameter.
    pub(super) fn evaluate_static_default_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        parameter_kind: StaticParameterKind,
        name: Option<StringId>,
        default_expression: LocalNodeId<Expression>,
        treat_type_arguments_as_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<StaticArgument> {
        // prefer value defaults when type arguments stay unconverted
        if !treat_type_arguments_as_types
            && let Some(value) = self.evaluate_static_expression_value(
                module,
                profile,
                default_expression,
                tree,
                symbols,
                types,
                None,
            )?
        {
            return Ok(StaticArgument::Evaluated { name, value });
        }

        // evaluate default value based on the parameter kind
        let value = match parameter_kind {
            StaticParameterKind::Type => {
                let resolved = self.try_evaluate_expression_to_type_value(
                    module,
                    profile,
                    default_expression,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                    true,
                    true,
                )?;

                let resolved = match resolved {
                    Type::Unevaluated(_) => {
                        return Ok(StaticArgument::Evaluated {
                            name,
                            value: StaticExpression::Unevaluated {
                                node: default_expression,
                            },
                        });
                    }
                    resolved => resolved,
                };

                let ty_id = types.insert_type_from(resolved, default_expression);
                StaticExpression::Type { ty: ty_id }
            }
            StaticParameterKind::Value => {
                if let Some(value) = self.evaluate_static_expression_value(
                    module,
                    profile,
                    default_expression,
                    tree,
                    symbols,
                    types,
                    None,
                )? {
                    value
                } else if let Some(enum_symbol) = self.enum_symbol_for_member_expression(
                    module,
                    profile,
                    default_expression,
                    tree,
                    symbols,
                )? {
                    let ty = types.insert_type_from_any(
                        Type::Reference {
                            symbol: enum_symbol,
                            static_arguments: None,
                        },
                        default_expression.into_any(),
                    );
                    StaticExpression::Type { ty }
                } else {
                    StaticExpression::Unevaluated {
                        node: default_expression,
                    }
                }
            }
        };

        Ok(StaticArgument::Evaluated { name, value })
    }

    /// Substitute static parameter references in a type.
    pub(crate) fn substitute_static_parameters(
        &self,
        ty_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        if let Some(mapped) = cache.get(&ty_id).copied() {
            return mapped;
        }
        cache.insert(ty_id, ty_id);

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

                        types.insert_type_from_type(
                            Type::Reference {
                                symbol,
                                static_arguments: Some(mapped_arguments),
                            },
                            ty_id,
                        )
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
                    types.insert_type_from_type(
                        Type::Value {
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Unary { operator, right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Unary {
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::Binary {
                            left: mapped_left,
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let distributive_union = match distributive_symbol {
                    Some(symbol) => match types.get_type(left).clone() {
                        Type::Reference {
                            symbol: reference_symbol,
                            static_arguments: None,
                        } if reference_symbol == symbol => {
                            if let Some(substitution) = substitutions.get(&symbol) {
                                let mut union_source = *substitution;
                                if let Type::Reference {
                                    symbol: union_symbol,
                                    static_arguments: None,
                                } = types.get_type(union_source)
                                    && union_symbol.ty() == SymbolType::TypeAlias
                                    && let Some(instance_id) =
                                        types.get_instance_type_id(*union_symbol)
                                {
                                    union_source = instance_id;
                                }

                                match types.get_type(union_source).clone() {
                                    Type::Union { elements } => Some((symbol, elements)),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    },
                    None => None,
                };

                if let Some((symbol, elements)) = distributive_union {
                    let mut branches = Vec::with_capacity(elements.len());
                    for element in elements {
                        let mut branch_substitutions = substitutions.clone();
                        branch_substitutions.insert(symbol, element);
                        let mut branch_cache = HashMap::new();
                        let mapped_left = self.substitute_static_parameters(
                            left,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_right = self.substitute_static_parameters(
                            right,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_then = self.substitute_static_parameters(
                            then_type,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let mapped_else = self.substitute_static_parameters(
                            else_type,
                            &branch_substitutions,
                            types,
                            &mut branch_cache,
                        );
                        let branch_id = types.insert_type_from_type(
                            Type::Conditional {
                                distributive_symbol: None,
                                left: mapped_left,
                                right: mapped_right,
                                then_type: mapped_then,
                                else_type: mapped_else,
                            },
                            ty_id,
                        );
                        branches.push(branch_id);
                    }

                    types.insert_type_from_type(Type::Union { elements: branches }, ty_id)
                } else {
                    let mapped_left =
                        self.substitute_static_parameters(left, substitutions, types, cache);
                    let mut branch_substitutions = substitutions.clone();
                    if let Some(symbol) = distributive_symbol {
                        branch_substitutions.remove(&symbol);
                    }
                    let mapped_right = self.substitute_static_parameters(
                        right,
                        &branch_substitutions,
                        types,
                        cache,
                    );
                    let mapped_then = self.substitute_static_parameters(
                        then_type,
                        &branch_substitutions,
                        types,
                        cache,
                    );
                    let mapped_else = self.substitute_static_parameters(
                        else_type,
                        &branch_substitutions,
                        types,
                        cache,
                    );
                    if mapped_left == left
                        && mapped_right == right
                        && mapped_then == then_type
                        && mapped_else == else_type
                    {
                        ty_id
                    } else {
                        types.insert_type_from_type(
                            Type::Conditional {
                                distributive_symbol,
                                left: mapped_left,
                                right: mapped_right,
                                then_type: mapped_then,
                                else_type: mapped_else,
                            },
                            ty_id,
                        )
                    }
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
                        symbol: parameter.symbol,
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type_from_type(
                        Type::Mapped {
                            parameter,
                            modifiers,
                            value: mapped_value,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::Index {
                            left: mapped_left,
                            index: mapped_index,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::TemplateLiteral {
                            strings,
                            spans: mapped_spans,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                let Some(static_arguments) = static_arguments else {
                    return ty_id;
                };

                let mut changed = false;
                let mapped_arguments = static_arguments
                    .iter()
                    .map(|argument| {
                        let mapped =
                            self.substitute_static_argument(argument, substitutions, types, cache);
                        if mapped != *argument {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();

                if changed {
                    types.insert_type_from_type(
                        Type::Import {
                            target,
                            qualifier,
                            static_arguments: Some(mapped_arguments),
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Infer { name, constraint } => {
                let mapped_constraint = constraint.map(|constraint| {
                    self.substitute_static_parameters(constraint, substitutions, types, cache)
                });
                if mapped_constraint == constraint {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Infer {
                            name,
                            constraint: mapped_constraint,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::Predicate {
                            asserts,
                            subject,
                            target: mapped_target,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::ValueOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::ReferenceOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::PointerOf { mutability, right } => {
                let mapped_right =
                    self.substitute_static_parameters(right, substitutions, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::PointerOf {
                            mutability,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                // substitute the array element type
                let mapped_element =
                    self.substitute_static_parameters(element, substitutions, types, cache);

                // update array size inferred types when the count is a substituted parameter
                let count_global = count.into_global_any(types.module_id);
                if let Some(count_type_id) = types.get_inferred_type_id(count_global)
                    && let Type::Reference { symbol, .. } = types.get_type(count_type_id)
                    && let Some(substitution) = substitutions.get(symbol)
                    && *substitution != count_type_id
                {
                    types.set_inferred_type(count_global, *substitution);
                }

                // reuse the existing type if substitutions were no-ops
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ArraySized {
                            element: mapped_element,
                            count,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                let mapped_element = element.map(|element| {
                    self.substitute_static_parameters(element, substitutions, types, cache)
                });
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Array {
                            element: mapped_element,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let mut did_change = false;
                let mapped_elements = elements
                    .into_iter()
                    .map(|element| {
                        let TypeElement {
                            label,
                            ty,
                            is_optional,
                            is_readonly,
                            is_rest,
                        } = element;
                        let mapped_ty =
                            self.substitute_static_parameters(ty, substitutions, types, cache);
                        if mapped_ty != ty {
                            did_change = true;
                        }
                        TypeElement {
                            label,
                            ty: mapped_ty,
                            is_optional,
                            is_readonly,
                            is_rest,
                        }
                    })
                    .collect::<Vec<_>>();
                if did_change {
                    types.insert_type_from_type(
                        Type::Tuple {
                            elements: mapped_elements,
                            is_readonly,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::Object {
                            fields: mapped_fields,
                            call_signatures: mapped_call_signatures,
                            construct_signatures: mapped_construct_signatures,
                            index_signatures: mapped_index_signatures,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::Function {
                            asynchrony,
                            cardinality,
                            static_parameters,
                            this_parameter: mapped_this,
                            dynamic_parameters: mapped_parameters,
                            return_type: mapped_return,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::Union {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
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
                    types.insert_type_from_type(
                        Type::Intersection {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
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

    /// Materialize static arguments inside type references for substitution.
    pub(crate) fn materialize_static_arguments_in_type(
        &self,
        argument_module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        argument_tree: &NodeTree,
        argument_symbols: &SymbolTable,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        // evaluate unevaluated types before materialization
        if matches!(types.get_type(ty_id), Type::Unevaluated(_)) {
            let _ = self.evaluate_type(
                argument_module,
                profile,
                ty_id,
                argument_tree,
                argument_symbols,
                types,
            );
        }

        // reuse cached materializations to avoid recursion loops
        if let Some(mapped) = cache.get(&ty_id).copied() {
            return mapped;
        }
        cache.insert(ty_id, ty_id);

        // snapshot the current type before mapping
        let ty = types.get_type(ty_id).clone();
        if matches!(ty, Type::Unevaluated(_)) {
            return ty_id;
        }

        let mapped = match ty {
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                let Some(static_arguments) = static_arguments else {
                    return ty_id;
                };

                // materialize unevaluated arguments using parameter kinds
                let mut resolved_arguments = self.materialize_static_arguments_for_reference(
                    argument_module,
                    profile,
                    symbol,
                    &static_arguments,
                    argument_tree,
                    argument_symbols,
                    types,
                );

                // materialize nested argument types for substitution
                let mut changed = resolved_arguments != static_arguments;
                for argument in resolved_arguments.iter_mut() {
                    let StaticArgument::Evaluated { name, value } = argument else {
                        continue;
                    };
                    let StaticExpression::Type { ty } = value else {
                        continue;
                    };
                    let mapped_ty = self.materialize_static_arguments_in_type(
                        argument_module,
                        profile,
                        *ty,
                        argument_tree,
                        argument_symbols,
                        types,
                        cache,
                    );
                    if mapped_ty != *ty {
                        *argument = StaticArgument::Evaluated {
                            name: *name,
                            value: StaticExpression::Type { ty: mapped_ty },
                        };
                        changed = true;
                    }
                }

                // reuse the existing type when no arguments changed
                if !changed {
                    ty_id
                } else {
                    if !resolved_arguments.is_empty() {
                        self.register_instance_for_symbol(
                            symbol,
                            resolved_arguments.clone(),
                            types,
                        );
                    }

                    types.insert_type_from_type(
                        Type::Reference {
                            symbol,
                            static_arguments: Some(resolved_arguments),
                        },
                        ty_id,
                    )
                }
            }
            Type::This | Type::TypeLiteral { .. } | Type::InferVar { .. } | Type::Error => ty_id,
            Type::Value { value } => {
                // propagate materialization through type-as-value wrappers
                let mapped_value = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    value,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_value == value {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Value {
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Unary { operator, right } => {
                let mapped_right = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    right,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Unary {
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let mapped_left = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    left,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                let mapped_right = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    right,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Binary {
                            left: mapped_left,
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let mapped_left = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    left,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                let mapped_right = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    right,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                let mapped_then = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    then_type,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                let mapped_else = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    else_type,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_left == left
                    && mapped_right == right
                    && mapped_then == then_type
                    && mapped_else == else_type
                {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Conditional {
                            distributive_symbol,
                            left: mapped_left,
                            right: mapped_right,
                            then_type: mapped_then,
                            else_type: mapped_else,
                        },
                        ty_id,
                    )
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    parameter.constraint,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                let mapped_key_remap = parameter.key_remap.map(|key_remap| {
                    self.materialize_static_arguments_in_type(
                        argument_module,
                        profile,
                        key_remap,
                        argument_tree,
                        argument_symbols,
                        types,
                        cache,
                    )
                });
                let mapped_value = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    value,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    ty_id
                } else {
                    let parameter = TypeMappedParameter {
                        name: parameter.name,
                        symbol: parameter.symbol,
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type_from_type(
                        Type::Mapped {
                            parameter,
                            modifiers,
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Index { left, index } => {
                let mapped_left = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    left,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                let mapped_index = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    index,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_left == left && mapped_index == index {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Index {
                            left: mapped_left,
                            index: mapped_index,
                        },
                        ty_id,
                    )
                }
            }
            Type::TemplateLiteral { strings, spans } => {
                let mut changed = false;
                let mapped_spans = spans
                    .iter()
                    .map(|span| {
                        let mapped = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            *span,
                            argument_tree,
                            argument_symbols,
                            types,
                            cache,
                        );
                        if mapped != *span {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::TemplateLiteral {
                            strings,
                            spans: mapped_spans,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                let Some(static_arguments) = static_arguments else {
                    return ty_id;
                };

                let mut changed = false;
                let mapped_arguments = static_arguments
                    .iter()
                    .map(|argument| {
                        let StaticArgument::Evaluated { name, value } = argument else {
                            return argument.clone();
                        };

                        let mapped_value = match value {
                            StaticExpression::Type { ty } => {
                                let mapped_ty = self.materialize_static_arguments_in_type(
                                    argument_module,
                                    profile,
                                    *ty,
                                    argument_tree,
                                    argument_symbols,
                                    types,
                                    cache,
                                );
                                if mapped_ty != *ty {
                                    changed = true;
                                }
                                StaticExpression::Type { ty: mapped_ty }
                            }
                            _ => value.clone(),
                        };

                        let mapped_argument = StaticArgument::Evaluated {
                            name: *name,
                            value: mapped_value,
                        };
                        if &mapped_argument != argument {
                            changed = true;
                        }
                        mapped_argument
                    })
                    .collect::<Vec<_>>();

                if changed {
                    types.insert_type_from_type(
                        Type::Import {
                            target,
                            qualifier,
                            static_arguments: Some(mapped_arguments),
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Infer { .. } => ty_id,
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let mapped_target = target.map(|target| {
                    self.materialize_static_arguments_in_type(
                        argument_module,
                        profile,
                        target,
                        argument_tree,
                        argument_symbols,
                        types,
                        cache,
                    )
                });
                if mapped_target == target {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Predicate {
                            asserts,
                            subject,
                            target: mapped_target,
                        },
                        ty_id,
                    )
                }
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    right,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ValueOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    right,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ReferenceOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::PointerOf { mutability, right } => {
                let mapped_right = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    right,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::PointerOf {
                            mutability,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                let mapped_element = self.materialize_static_arguments_in_type(
                    argument_module,
                    profile,
                    element,
                    argument_tree,
                    argument_symbols,
                    types,
                    cache,
                );
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ArraySized {
                            element: mapped_element,
                            count,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                let mapped_element = element.map(|element| {
                    self.materialize_static_arguments_in_type(
                        argument_module,
                        profile,
                        element,
                        argument_tree,
                        argument_symbols,
                        types,
                        cache,
                    )
                });
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Array {
                            element: mapped_element,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            element.ty,
                            argument_tree,
                            argument_symbols,
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
                    types.insert_type_from_type(
                        Type::Tuple {
                            elements: mapped_elements,
                            is_readonly,
                        },
                        ty_id,
                    )
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
                        let mapped = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            field.ty,
                            argument_tree,
                            argument_symbols,
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
                        let mapped = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            *signature,
                            argument_tree,
                            argument_symbols,
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
                        let mapped = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            *signature,
                            argument_tree,
                            argument_symbols,
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
                        let mapped_key = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            signature.key_type,
                            argument_tree,
                            argument_symbols,
                            types,
                            cache,
                        );
                        let mapped_value = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            signature.value_type,
                            argument_tree,
                            argument_symbols,
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
                    types.insert_type_from_type(
                        Type::Object {
                            fields: mapped_fields,
                            call_signatures: mapped_call_signatures,
                            construct_signatures: mapped_construct_signatures,
                            index_signatures: mapped_index_signatures,
                        },
                        ty_id,
                    )
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
                    let mapped = self.materialize_static_arguments_in_type(
                        argument_module,
                        profile,
                        this_parameter,
                        argument_tree,
                        argument_symbols,
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
                        let mapped = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            *parameter,
                            argument_tree,
                            argument_symbols,
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
                    let mapped = self.materialize_static_arguments_in_type(
                        argument_module,
                        profile,
                        return_type,
                        argument_tree,
                        argument_symbols,
                        types,
                        cache,
                    );
                    if mapped != return_type {
                        changed = true;
                    }
                    mapped
                });
                if changed {
                    types.insert_type_from_type(
                        Type::Function {
                            asynchrony,
                            cardinality,
                            static_parameters,
                            this_parameter: mapped_this,
                            dynamic_parameters: mapped_parameters,
                            return_type: mapped_return,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Union { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            *element,
                            argument_tree,
                            argument_symbols,
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
                    types.insert_type_from_type(
                        Type::Union {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Intersection { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.materialize_static_arguments_in_type(
                            argument_module,
                            profile,
                            *element,
                            argument_tree,
                            argument_symbols,
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
                    types.insert_type_from_type(
                        Type::Intersection {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Unevaluated(_) => ty_id,
        };

        cache.insert(ty_id, mapped);
        mapped
    }

    /// Materialize static arguments for a type reference based on parameter kinds.
    pub(crate) fn materialize_static_arguments_for_reference(
        &self,
        argument_module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        static_arguments: &[StaticArgument],
        argument_tree: &NodeTree,
        argument_symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Vec<StaticArgument> {
        // skip when no arguments need evaluation
        if !static_arguments
            .iter()
            .any(|argument| matches!(argument, StaticArgument::Unevaluated { .. }))
        {
            return static_arguments.to_vec();
        }

        // collect parameter symbols for the reference
        let Some(parameter_symbols) = self.collect_static_parameter_symbols(
            argument_module,
            symbol,
            profile,
            argument_tree,
            argument_symbols,
        ) else {
            return static_arguments.to_vec();
        };
        if parameter_symbols.is_empty() {
            return static_arguments.to_vec();
        }

        // select a source node for parameter inference
        let Some(source_id) = static_arguments.iter().find_map(|argument| match argument {
            StaticArgument::Unevaluated { node } => Some(node.into_any()),
            _ => None,
        }) else {
            return static_arguments.to_vec();
        };

        // map parameter names to their resolved kinds
        let mut parameter_kinds = Vec::with_capacity(parameter_symbols.len());
        let mut parameter_name_kinds = HashMap::new();
        for parameter_symbol in parameter_symbols {
            let parameter = self.collect_static_parameter(
                argument_module,
                parameter_symbol,
                source_id,
                profile,
                argument_tree,
                argument_symbols,
                types,
            );
            let kind = parameter.kind;
            if let Some(name) = parameter.name {
                parameter_name_kinds.insert(name, kind);
            }
            parameter_kinds.push(kind);
        }

        // evaluate arguments based on the referenced parameter kinds
        let mut resolved_arguments = Vec::with_capacity(static_arguments.len());
        for (index, argument) in static_arguments.iter().enumerate() {
            let (argument_name, argument_node) = match argument {
                StaticArgument::Evaluated { name, .. } => (*name, None),
                StaticArgument::Unevaluated { node } => {
                    let argument_node = argument_tree.get(*node);
                    let name = match argument_node {
                        Argument::Named { name, .. } => Some(*name),
                        _ => None,
                    };
                    (name, Some(*node))
                }
            };

            let parameter_kind = argument_name
                .and_then(|name| parameter_name_kinds.get(&name).copied())
                .or_else(|| parameter_kinds.get(index).copied());

            let Some(parameter_kind) = parameter_kind else {
                resolved_arguments.push(argument.clone());
                continue;
            };

            let Some(argument_node) = argument_node else {
                resolved_arguments.push(argument.clone());
                continue;
            };

            let argument = argument_tree.get(argument_node);
            let expression_id = argument.value();
            let evaluated = match parameter_kind {
                StaticParameterKind::Type => {
                    if let Ok(ty_id) = self.try_evaluate_expression_to_type(
                        argument_module,
                        profile,
                        expression_id,
                        argument_tree,
                        argument_symbols,
                        types,
                        false,
                        true,
                    ) && !matches!(types.get_type(ty_id), Type::Unevaluated(_))
                    {
                        Some(StaticExpression::Type { ty: ty_id })
                    } else {
                        None
                    }
                }
                StaticParameterKind::Value => self
                    .evaluate_static_expression_value(
                        argument_module,
                        profile,
                        expression_id,
                        argument_tree,
                        argument_symbols,
                        types,
                        None,
                    )
                    .ok()
                    .flatten(),
            };

            if let Some(value) = evaluated {
                resolved_arguments.push(StaticArgument::Evaluated {
                    name: argument_name,
                    value,
                });
            } else {
                resolved_arguments.push(StaticArgument::Unevaluated {
                    node: argument_node,
                });
            }
        }

        resolved_arguments
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
