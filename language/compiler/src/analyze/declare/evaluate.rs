use std::collections::HashSet;

use crate::analyze::common::CanonicalSymbolMode;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Argument, BinaryOperator, BindingKind, Declaration, DynamicKey, EnumFieldValue, Expression,
    FunctionMode, FunctionSignature, GlobalSymbolId, IntrinsicType, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, Mutability, NodeTree, PrimitiveType, Property, Resolution, ScalarLiteral,
    StaticArgument, StaticExpression, SymbolTable, Type, TypeElement, TypeField,
    TypeIndexSignature, TypeLiteral, TypeMappedParameter, TypeTable, TypeUnaryOperator,
    UnaryOperator,
};
use destack_workspace::{Module, ModuleSource, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Evaluate a Type (in-place).
    /// Converts Type::Unevaluated to the actual Type value.
    pub(crate) fn evaluate_type(
        &self,
        module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // pull the unevaluated expression id when needed
        let expression_id = {
            let ty = types.get_type(ty_id);
            let Type::Unevaluated(expression_id) = *ty else {
                return Ok(());
            };
            expression_id
        };

        // evaluate and update in place
        let evaluated_ty = self.try_evaluate_expression_to_type_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            true,
        )?;
        let ty = types.get_type_mut(ty_id);
        *ty = evaluated_ty;

        // invalidate normalization cache after in-place updates
        types.invalidate_normalization_cache();

        Ok(())
    }

    /// Try to evaluate an Expression as a Type.
    /// Returns the evaluated Type value, or a Type::Unevaluated if it fails.
    /// Set validate_static_argument_bounds to false to defer bound checks.
    pub(crate) fn try_evaluate_expression_to_type_value(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
    ) -> AnalyzeResult<Type> {
        // default to enforcing implicit managed checks
        self.try_evaluate_expression_to_type_value_with_controls(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            true,
        )
    }

    /// Try to evaluate an Expression as a Type with ownership enforcement controls.
    fn try_evaluate_expression_to_type_value_with_controls(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Type> {
        // evaluate to a concrete type when possible
        let ty = self
            .evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )?
            .unwrap_or(Type::Unevaluated(expression_id));
        Ok(ty)
    }

    /// Try to evaluate an Expression as a Type id.
    /// Set validate_static_argument_bounds to false to defer bound checks.
    /// Set enforce_implicit_managed to false to skip noImplicitManaged enforcement.
    pub(crate) fn try_evaluate_expression_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        // evaluate to a concrete type when possible
        let ty = self.try_evaluate_expression_to_type_value_with_controls(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Evaluate a function signature into a Type.
    pub(super) fn evaluate_function_signature_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        signature: &FunctionSignature,
        source_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // collect static parameter placeholders
        let static_parameters =
            self.static_parameter_placeholders_for_signature(module, signature, tree, types);

        // evaluate parameter types
        let mut dynamic_parameters = Vec::with_capacity(signature.dynamic_parameters.len());
        for parameter_id in signature.dynamic_parameters.iter() {
            let declared_type_id = types
                .get_declared_type_id(parameter_id.into_global(module.id).into())
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, *parameter_id)
                });

            self.evaluate_type(module, profile, declared_type_id, tree, symbols, types)?;

            dynamic_parameters.push(declared_type_id);
        }

        // evaluate this parameter when present
        let this_parameter = if let Some(this_parameter_id) = signature.this_parameter {
            let declared_type_id = types
                .get_declared_type_id(this_parameter_id.into_global(module.id).into())
                .unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, this_parameter_id)
                });

            self.evaluate_type(module, profile, declared_type_id, tree, symbols, types)?;

            Some(declared_type_id)
        } else {
            None
        };

        // evaluate return type
        let return_type = if let Some(return_type_id) = signature.return_type {
            Some(self.try_evaluate_expression_to_type(
                module,
                profile,
                return_type_id,
                tree,
                symbols,
                types,
                true,
                true,
            )?)
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            Some(types.insert_type_from_any(ty, source_id))
        };

        // build function type
        Ok(Type::Function {
            asynchrony: signature.asynchrony,
            cardinality: signature.cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        })
    }

    /// Evaluate static arguments for a type reference.
    fn evaluate_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // skip when there are no static arguments
        let Some(static_arguments) = static_arguments else {
            return Ok(None);
        };

        let mut evaluated_arguments = Vec::with_capacity(static_arguments.len());

        // evaluate each static argument into a literal or type
        for argument_id in static_arguments {
            let argument = tree.get(*argument_id);
            let name = match argument {
                Argument::Named { name, .. } => Some(*name),
                _ => None,
            };
            let expression_id = argument.value();

            // keep comptime expressions unevaluated for static argument resolution
            if matches!(tree.get(expression_id), Expression::Comptime { .. }) {
                evaluated_arguments.push(StaticArgument::Unevaluated { node: *argument_id });
                continue;
            }

            // evaluate static values directly when possible
            if let Some(value) = self.evaluate_static_expression_value(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                None,
            )? {
                evaluated_arguments.push(StaticArgument::Evaluated { name, value });
                continue;
            }

            // fall back to resolving the argument type
            let resolved = self.evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                true,
                true,
            )?;

            // keep unevaluated arguments so later phases can resolve them
            let Some(resolved) = resolved else {
                evaluated_arguments.push(StaticArgument::Unevaluated { node: *argument_id });
                continue;
            };

            // treat resolved types as static arguments
            let ty_id = types.insert_type_from(resolved, expression_id);
            evaluated_arguments.push(StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty: ty_id },
            });
        }

        Ok(Some(evaluated_arguments))
    }

    /// Collect element types for a binary union or intersection expression.
    /// (This is a faster and deterministic alternative for the elementwise combinators.)
    fn collect_binary_type_elements(
        &self,
        module: &Module,
        profile: ProfileId,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
        operator: BinaryOperator,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Vec<LocalTypeId>> {
        // seed the work list right-to-left so left is processed first
        let mut pending_expressions = Vec::new();
        pending_expressions.push(right);
        pending_expressions.push(left);

        // walk the binary tree
        let mut elements = Vec::new();
        while let Some(expression_id) = pending_expressions.pop() {
            let expression = tree.get(expression_id);

            // unwrap parenthesized expressions
            if let Expression::Parenthesized { expression } = expression {
                pending_expressions.push(*expression);
                continue;
            }

            // flatten nested union or intersection expressions
            if let Expression::Binary {
                left,
                operator: nested_operator,
                right,
                ..
            } = expression
                && *nested_operator == operator
            {
                // push right first to preserve left-to-right order
                pending_expressions.push(*right);
                pending_expressions.push(*left);
                continue;
            }

            // evaluate the leaf expression to a type id
            let element_id = self.try_evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )?;

            // flatten nested union or intersection types
            match (operator, types.get_type(element_id)) {
                (
                    BinaryOperator::ElementwiseOr,
                    Type::Union {
                        elements: union_elements,
                    },
                ) => {
                    elements.extend_from_slice(union_elements);
                }
                (
                    BinaryOperator::ElementwiseAnd,
                    Type::Intersection {
                        elements: intersection_elements,
                    },
                ) => {
                    elements.extend_from_slice(intersection_elements);
                }
                _ => {
                    elements.push(element_id);
                }
            }
        }

        Ok(elements)
    }

    /// Evaluate an expression into a static value expression.
    pub(crate) fn evaluate_static_expression_value(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        enum_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        self.evaluate_static_expression_value_inner(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            enum_symbol,
        )
    }

    /// Evaluate an expression into a static value expression.
    #[allow(clippy::only_used_in_recursion)]
    fn evaluate_static_expression_value_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        enum_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        let expression = tree.get(expression_id);

        let value = match expression {
            Expression::ScalarLiteral { value } => StaticExpression::ScalarLiteral {
                value: value.clone(),
            },
            Expression::TypeLiteral { value } => StaticExpression::TypeLiteral {
                value: value.clone(),
            },
            Expression::Type { value } => StaticExpression::Type { ty: *value },
            Expression::Parenthesized { expression } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *expression,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                );
            }
            Expression::Cast { value, .. } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *value,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                );
            }
            Expression::OwnershipCast { value, .. } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *value,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                );
            }
            Expression::Unary { operator, right } => {
                let right_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *right,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                )?;
                let Some(StaticExpression::ScalarLiteral { value }) = right_value else {
                    return Ok(None);
                };
                let ScalarLiteral::Integer(value) = value else {
                    return Ok(None);
                };
                let value = match operator {
                    UnaryOperator::Plus => value,
                    UnaryOperator::Negate => -value,
                    UnaryOperator::ElementwiseNot => !value,
                    _ => return Ok(None),
                };
                StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(value),
                }
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *left,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                )?;
                let right_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *right,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                )?;
                let (left_value, right_value) = match (left_value, right_value) {
                    (
                        Some(StaticExpression::ScalarLiteral {
                            value: ScalarLiteral::Integer(left_value),
                        }),
                        Some(StaticExpression::ScalarLiteral {
                            value: ScalarLiteral::Integer(right_value),
                        }),
                    ) => (left_value, right_value),
                    _ => return Ok(None),
                };

                let value = match operator {
                    BinaryOperator::Add => left_value.checked_add(right_value),
                    BinaryOperator::Subtract => left_value.checked_sub(right_value),
                    BinaryOperator::Multiply => left_value.checked_mul(right_value),
                    BinaryOperator::Divide => {
                        if right_value == 0 {
                            None
                        } else {
                            let remainder = left_value % right_value;
                            if remainder != 0 {
                                None
                            } else {
                                left_value.checked_div(right_value)
                            }
                        }
                    }
                    BinaryOperator::Remainder => left_value.checked_rem(right_value),
                    BinaryOperator::ShiftLeft => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| left_value.checked_shl(shift)),
                    BinaryOperator::ShiftRight => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| left_value.checked_shr(shift)),
                    BinaryOperator::UnsignedShiftRight => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| (left_value as u64).checked_shr(shift))
                        .and_then(|shifted| i64::try_from(shifted).ok()),
                    BinaryOperator::ElementwiseAnd => Some(left_value & right_value),
                    BinaryOperator::ElementwiseOr => Some(left_value | right_value),
                    BinaryOperator::ElementwiseXor => Some(left_value ^ right_value),
                    _ => None,
                };

                let Some(value) = value else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(value),
                }
            }
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                let Some(enum_symbol) = enum_symbol else {
                    return Ok(None);
                };
                let value = self.enum_field_value_for_symbol_reference(
                    module,
                    enum_symbol,
                    *target_symbol,
                    symbols,
                    types,
                );
                let Some(value) = value else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: match value {
                        EnumFieldValue::Int(value) => ScalarLiteral::Integer(value),
                        EnumFieldValue::String(value) => ScalarLiteral::String(value),
                    },
                }
            }
            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Ok(None);
                }
                let Some(enum_symbol) = enum_symbol else {
                    return Ok(None);
                };

                let node_id = expression_id.into_global_any(module.id);
                let resolution_id = types.get_resolution_for_node(node_id);
                let target_symbol = if let Some(resolution_id) = resolution_id {
                    let resolution = types.get_resolution(resolution_id);
                    let Resolution::Static { candidate, .. } = resolution else {
                        return Ok(None);
                    };
                    candidate.target_symbol
                } else {
                    let left_symbol = match tree.get(*left) {
                        Expression::LocalReference { target_symbol, .. }
                        | Expression::ModuleReference { target_symbol, .. }
                        | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
                        _ => return Ok(None),
                    };

                    if left_symbol != enum_symbol {
                        return Ok(None);
                    }

                    let Some(field_symbol) =
                        self.enum_field_symbol_for_name(module, enum_symbol, *name, tree, symbols)
                    else {
                        return Ok(None);
                    };

                    field_symbol
                };

                let value = self.enum_field_value_for_symbol_reference(
                    module,
                    enum_symbol,
                    target_symbol,
                    symbols,
                    types,
                );
                let Some(value) = value else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: match value {
                        EnumFieldValue::Int(value) => ScalarLiteral::Integer(value),
                        EnumFieldValue::String(value) => ScalarLiteral::String(value),
                    },
                }
            }
            Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let start_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *start,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                )?;
                let end_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *end,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                )?;

                let (Some(start_value), Some(end_value)) = (start_value, end_value) else {
                    return Ok(None);
                };

                StaticExpression::RangeExpression {
                    start: Box::new(start_value),
                    end: Box::new(end_value),
                    is_inclusive: *is_inclusive,
                }
            }
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value_inner(
                        module,
                        profile,
                        element.value(),
                        tree,
                        symbols,
                        types,
                        enum_symbol,
                    )?;
                    let Some(value) = value else {
                        return Ok(None);
                    };
                    values.push(value);
                }

                StaticExpression::ArrayExpression { elements: values }
            }
            Expression::TupleExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value_inner(
                        module,
                        profile,
                        element.value(),
                        tree,
                        symbols,
                        types,
                        enum_symbol,
                    )?;
                    let Some(value) = value else {
                        return Ok(None);
                    };
                    values.push(value);
                }

                StaticExpression::TupleExpression { elements: values }
            }
            _ => return Ok(None),
        };

        Ok(Some(value))
    }

    /// Evaluate an Expression into a Type with validation controls.
    fn evaluate_expression_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<Type>> {
        let options = self.analyze_context_options_for_module(module.id);
        let is_user_module = matches!(module.source, ModuleSource::User);

        // clone to avoid holding a tree borrow across recursive evaluation
        let expression = tree.get(expression_id).clone();

        let ty = match expression {
            Expression::ScalarLiteral { value } => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value.clone()),
            },
            Expression::TypeLiteral { value } => {
                // reject forbidden type literals in user code
                if is_user_module {
                    // disallow explicit any
                    if options.no_any && matches!(value, TypeLiteral::Any) {
                        return Err(AnalyzeError::AnyTypeDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }

                    // disallow explicit unknown
                    if options.no_unknown && matches!(value, TypeLiteral::Unknown) {
                        return Err(AnalyzeError::UnknownTypeDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }

                    // disallow imprecise primitives
                    if options.no_imprecise_primitives
                        && matches!(value, TypeLiteral::Primitive(PrimitiveType::Number))
                    {
                        return Err(AnalyzeError::ImprecisePrimitiveDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
                }

                // map builtin iterator return to configured strictness
                if let TypeLiteral::Intrinsic(IntrinsicType::BuiltinIteratorReturn) = value {
                    let profile = self.program.profile(profile);
                    let mapped = if profile.key.flags.strict_builtin_iterator_return {
                        TypeLiteral::Undefined
                    } else {
                        TypeLiteral::Any
                    };
                    Type::TypeLiteral { value: mapped }
                } else {
                    Type::TypeLiteral {
                        value: value.clone(),
                    }
                }
            }
            Expression::This => Type::This,
            Expression::Parenthesized { expression } => {
                return self.evaluate_expression_to_type(
                    module,
                    profile,
                    expression,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                );
            }

            Expression::Declaration {
                declaration: declaration_id,
            } => {
                let declaration = tree.get(declaration_id).clone();
                if let Declaration::Function { signature, .. } = declaration {
                    self.evaluate_function_signature_to_type(
                        module,
                        profile,
                        &signature,
                        declaration_id.into_any(),
                        tree,
                        symbols,
                        types,
                    )?
                } else {
                    // #Incomplete: only function declarations are evaluable as types (?)
                    return Ok(None);
                }
            }

            // not
            Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Unary {
                    operator: TypeUnaryOperator::Not,
                    right: type_id,
                }
            }
            // maybe
            Expression::Maybe { .. } => {
                return Ok(None); // cannot be evaluated to a type here (not supported in type contexts)
            }
            // must
            Expression::Must { left } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Unary {
                    operator: TypeUnaryOperator::Must,
                    right: type_id,
                }
            }
            // value
            Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::ValueOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // reference
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::ReferenceOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // pointer
            Expression::PointerOf { mutability, right } => {
                let type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    false,
                )?;
                Type::PointerOf {
                    mutability,
                    right: type_id,
                }
            }
            // unary
            Expression::TypeUnary { operator, right } => {
                if operator == TypeUnaryOperator::Typeof {
                    return Ok(Some(self.evaluate_typeof_expression(
                        module,
                        profile,
                        expression_id,
                        right,
                        tree,
                        symbols,
                        types,
                    )?));
                }

                let right_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Unary {
                    operator,
                    right: right_id,
                }
            }
            // binary
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let right_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Binary {
                    left: left_id,
                    operator,
                    right: right_id,
                }
            }
            Expression::TypeConditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                let left_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let right_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    right,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let should_validate_branches = !self.type_contains_static_parameters(
                    module,
                    profile,
                    left_id,
                    symbols,
                    types,
                    &mut HashSet::new(),
                ) && validate_static_argument_bounds;
                let then_type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    then_type,
                    tree,
                    symbols,
                    types,
                    should_validate_branches,
                    enforce_implicit_managed,
                )?;
                let else_type_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    else_type,
                    tree,
                    symbols,
                    types,
                    should_validate_branches,
                    enforce_implicit_managed,
                )?;
                Type::Conditional {
                    left: left_id,
                    right: right_id,
                    then_type: then_type_id,
                    else_type: else_type_id,
                }
            }
            Expression::TypeMapped {
                parameter,
                modifiers,
                value,
            } => {
                let constraint = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    parameter.constraint,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                // cache the mapped parameter constraint for later validation
                let parameter_symbol = parameter.symbol.into_global(module.id);
                types.set_static_parameter_constraint_type(parameter_symbol, constraint);
                let key_remap = parameter.key_remap.map(|key_remap| {
                    self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        key_remap,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let key_remap = match key_remap {
                    Some(Ok(key_remap)) => Some(key_remap),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                let value_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    value,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let parameter = TypeMappedParameter {
                    name: parameter.name,
                    constraint,
                    key_remap,
                };
                Type::Mapped {
                    parameter,
                    modifiers,
                    value: value_id,
                }
            }
            Expression::TypeIndex { left, index } => {
                let left_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    left,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                let index_id = self.try_evaluate_expression_to_type(
                    module,
                    profile,
                    index,
                    tree,
                    symbols,
                    types,
                    validate_static_argument_bounds,
                    enforce_implicit_managed,
                )?;
                Type::Index {
                    left: left_id,
                    index: index_id,
                }
            }
            Expression::TypeTemplateLiteral { strings, spans } => {
                let spans = spans
                    .iter()
                    .map(|span| {
                        self.try_evaluate_expression_to_type(
                            module,
                            profile,
                            *span,
                            tree,
                            symbols,
                            types,
                            validate_static_argument_bounds,
                            enforce_implicit_managed,
                        )
                    })
                    .collect::<AnalyzeResult<Vec<_>>>()?;
                Type::TemplateLiteral {
                    strings: strings.clone(),
                    spans,
                }
            }
            Expression::TypeImport {
                target,
                qualifier,
                static_arguments,
            } => {
                let static_arguments = self.evaluate_static_arguments(
                    module,
                    profile,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                )?;
                Type::Import {
                    target,
                    qualifier: qualifier.clone(),
                    static_arguments,
                }
            }
            Expression::TypeInfer { name, constraint } => {
                let constraint = constraint.map(|constraint| {
                    self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        constraint,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let constraint = match constraint {
                    Some(Ok(constraint)) => Some(constraint),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                Type::Infer { name, constraint }
            }
            Expression::TypePredicate {
                asserts,
                subject,
                target,
            } => {
                let target = target.map(|target| {
                    self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        target,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                });
                let target = match target {
                    Some(Ok(target)) => Some(target),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                Type::Predicate {
                    asserts,
                    subject,
                    target,
                }
            }

            // union and intersection types
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => match operator {
                BinaryOperator::ElementwiseOr => {
                    let elements = self.collect_binary_type_elements(
                        module,
                        profile,
                        left,
                        right,
                        operator,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    Type::Union { elements }
                }
                BinaryOperator::ElementwiseAnd => {
                    let elements = self.collect_binary_type_elements(
                        module,
                        profile,
                        left,
                        right,
                        operator,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    Type::Intersection { elements }
                }
                _ => return Ok(None),
            },

            // references
            Expression::LocalReference {
                target_symbol,
                static_arguments,
                ..
            }
            | Expression::ModuleReference {
                target_symbol,
                static_arguments,
                ..
            }
            | Expression::GlobalReference {
                target_symbol,
                static_arguments,
                ..
            } => {
                // resolve import targets without collapsing type aliases
                let target_symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    profile,
                    target_symbol,
                    CanonicalSymbolMode::PreserveAliases,
                );

                // prefer merged type symbols for namespaces
                let target_symbol =
                    self.merged_type_symbol_id(module, symbols, profile, target_symbol);
                let static_arguments = self.evaluate_static_arguments(
                    module,
                    profile,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                )?;
                let options = self.analyze_context_options_for_module(module.id);
                let resolved_arguments = self.resolve_type_reference_static_arguments(
                    module,
                    profile,
                    expression_id.into_any(),
                    target_symbol,
                    static_arguments.as_deref(),
                    validate_static_argument_bounds,
                    &options,
                    tree,
                    symbols,
                    types,
                )?;
                let static_arguments = resolved_arguments.or(static_arguments);
                if let Some(arguments) = static_arguments.as_deref()
                    && arguments.iter().any(|argument| match argument {
                        StaticArgument::Evaluated {
                            value: StaticExpression::Type { ty },
                            ..
                        } => matches!(types.get_type(*ty), Type::Error),
                        _ => false,
                    })
                {
                    return Ok(Some(Type::Error));
                }

                // normalize well known references into canonical structural types
                if let Some(normalized) = self.normalize_well_known_type_reference(
                    module,
                    symbols,
                    profile,
                    expression_id.into_any(),
                    target_symbol,
                    static_arguments.as_deref(),
                    types,
                ) {
                    return Ok(Some(normalized));
                }

                Type::Reference {
                    symbol: target_symbol,
                    static_arguments,
                }
            }

            // tuple (anonymous)
            Expression::ArrayExpression { elements } => {
                // evaluate element types
                let mut element_types = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let argument = tree.get(element_id);
                    let value_id = argument.value();
                    let value_ty_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        value_id,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    let mut element = TypeElement::new(value_ty_id);
                    match argument {
                        Argument::Labeled { label, .. } => {
                            element.label = Some(*label);
                        }
                        Argument::Spread { .. } => {
                            element.is_rest = true;
                        }
                        _ => {}
                    }
                    element_types.push(element);
                }

                Type::Tuple {
                    elements: element_types,
                }
            }

            // tuple (anonymous)
            Expression::TupleExpression { elements } => {
                // evaluate element types
                let mut element_types = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let value_id = {
                        let argument = tree.get(element_id);
                        argument.value()
                    };
                    let value_ty_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        value_id,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    element_types.push(TypeElement::new(value_ty_id));
                }

                Type::Tuple {
                    elements: element_types,
                }
            }
            // sequence expression (comma operator)
            Expression::SequenceExpression { .. } => {
                return Err(AnalyzeError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
            }
            // object (anonymous)
            Expression::ObjectExpression { properties } => {
                // evaluate object fields
                // #Cleanup: extract property -> type field evaluation?
                let mut fields = Vec::with_capacity(properties.len());
                let mut call_signatures = Vec::new();
                let mut construct_signatures = Vec::new();
                let mut index_signatures = Vec::new();
                for property_id in properties {
                    let property = tree.get(property_id).clone();
                    let field = match property {
                        Property::Field {
                            modifiers,
                            key,
                            value,
                            ..
                        } => {
                            // index signature
                            if let Some(DynamicKey::NamedExpression { name, key }) = key {
                                let key_type = self.try_evaluate_expression_to_type(
                                    module,
                                    profile,
                                    key,
                                    tree,
                                    symbols,
                                    types,
                                    validate_static_argument_bounds,
                                    enforce_implicit_managed,
                                )?;
                                let value_type = if let Some(value_id) = value {
                                    self.try_evaluate_expression_to_type(
                                        module,
                                        profile,
                                        value_id,
                                        tree,
                                        symbols,
                                        types,
                                        validate_static_argument_bounds,
                                        enforce_implicit_managed,
                                    )?
                                } else {
                                    let ty = Type::TypeLiteral {
                                        value: TypeLiteral::Unknown,
                                    };
                                    types.insert_type_from(ty, property_id)
                                };
                                let is_readonly = modifiers.is_some_and(|modifiers| {
                                    modifiers.mutability == Some(Mutability::Immutable)
                                });
                                index_signatures.push(TypeIndexSignature {
                                    name,
                                    key_type,
                                    value_type,
                                    is_readonly,
                                });
                                continue;
                            }

                            let Some(key) = key.and_then(|key| {
                                self.static_key_from_dynamic_key(profile, key, tree, symbols, types)
                            }) else {
                                if module.language_type.is_declaration() {
                                    continue;
                                }
                                return Err(AnalyzeError::UnsupportedConstruct {
                                    node: property_id
                                        .into_global_any(module.id)
                                        .into_anchored(Some(profile)),
                                });
                            };

                            let ty = if let Some(value_id) = value {
                                self.try_evaluate_expression_to_type(
                                    module,
                                    profile,
                                    value_id,
                                    tree,
                                    symbols,
                                    types,
                                    validate_static_argument_bounds,
                                    enforce_implicit_managed,
                                )?
                            } else {
                                let ty = Type::TypeLiteral {
                                    value: TypeLiteral::Unknown,
                                };
                                types.insert_type_from(ty, property_id)
                            };
                            let is_optional = modifiers.is_some_and(|modifiers| {
                                modifiers.kind == Some(BindingKind::Maybe)
                            });
                            let is_readonly = modifiers.is_some_and(|modifiers| {
                                modifiers.mutability == Some(Mutability::Immutable)
                            });

                            TypeField {
                                key,
                                ty,
                                is_optional,
                                is_readonly,
                            }
                        }
                        Property::Method {
                            modifiers,
                            key,
                            signature,
                            ..
                        } => {
                            // call or construct signature
                            if key.is_none()
                                && matches!(
                                    signature.mode,
                                    Some(FunctionMode::Call)
                                        | Some(FunctionMode::New)
                                        | Some(FunctionMode::Constructor)
                                )
                            {
                                let ty = self.evaluate_function_signature_to_type(
                                    module,
                                    profile,
                                    &signature,
                                    property_id.into_any(),
                                    tree,
                                    symbols,
                                    types,
                                )?;
                                let ty_id = types.insert_type_from(ty, property_id);
                                match signature.mode {
                                    Some(FunctionMode::New) | Some(FunctionMode::Constructor) => {
                                        construct_signatures.push(ty_id);
                                    }
                                    _ => {
                                        call_signatures.push(ty_id);
                                    }
                                }
                                continue;
                            }

                            let Some(key) = key.and_then(|key| {
                                self.static_key_from_dynamic_key(profile, key, tree, symbols, types)
                            }) else {
                                if module.language_type.is_declaration() {
                                    continue;
                                }
                                return Err(AnalyzeError::UnsupportedConstruct {
                                    node: property_id
                                        .into_global_any(module.id)
                                        .into_anchored(Some(profile)),
                                });
                            };

                            let ty = self.evaluate_function_signature_to_type(
                                module,
                                profile,
                                &signature,
                                property_id.into_any(),
                                tree,
                                symbols,
                                types,
                            )?;
                            let ty_id = types.insert_type_from(ty, property_id);

                            let is_optional = modifiers.is_some_and(|modifiers| {
                                modifiers.kind == Some(BindingKind::Maybe)
                            });
                            let is_readonly = modifiers.is_some_and(|modifiers| {
                                modifiers.mutability == Some(Mutability::Immutable)
                            });

                            TypeField {
                                key,
                                ty: ty_id,
                                is_optional,
                                is_readonly,
                            }
                        }
                        Property::Spread { .. } => {
                            // #Incomplete: spread properties into types
                            return Err(AnalyzeError::UnsupportedConstruct {
                                node: property_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            });
                        }
                    };

                    fields.push(field);
                }

                Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                }
            }

            // array or slice
            Expression::Index { left, right } => {
                // array with static length
                if let Some(right) = right {
                    let left_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        left,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    Type::ArraySized {
                        element: left_id,
                        count: right,
                    }
                }
                // slice
                else {
                    let left_id = self.try_evaluate_expression_to_type(
                        module,
                        profile,
                        left,
                        tree,
                        symbols,
                        types,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )?;
                    Type::Array {
                        element: Some(left_id),
                    }
                }
            }

            _ => return Ok(None),
        };

        // enforce implicit managed restrictions for type expressions
        if is_user_module
            && enforce_implicit_managed
            && options.no_implicit_managed
            && !self.expression_has_explicit_ownership(tree, expression_id)
            && self.type_is_implicit_managed(module, profile, &ty, types)
        {
            return Err(AnalyzeError::ImplicitManagedTypeDisabled {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
        }

        Ok(Some(ty))
    }

    /// Evaluate a typeof type expression into a Type.
    fn evaluate_typeof_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        _symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        // unwrap parenthesized targets
        let mut target_id = right_id;
        loop {
            let Expression::Parenthesized { expression } = tree.get(target_id) else {
                break;
            };
            target_id = *expression;
        }

        // resolve the target symbol for a typeof reference
        let Some(target_symbol) = tree.get(target_id).target_symbol() else {
            return Ok(Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            });
        };

        // resolve the value type for the target symbol
        let value_ty_id = if let Some(value_ty_id) = types.get_value_type_id(target_symbol) {
            Some(value_ty_id)
        } else if target_symbol.module_id != module.id {
            Some(self.resolve_remote_symbol_value_type(
                module,
                profile,
                expression_id.into_any(),
                target_symbol,
                types,
            )?)
        } else {
            None
        };

        // fall back to unknown when the value type is missing
        let Some(value_ty_id) = value_ty_id else {
            return Ok(Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            });
        };

        Ok(types.get_type(value_ty_id).clone())
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{Expression, PrimitiveType, Type, TypeLiteral};

    use crate::TestProgram;

    #[test]
    fn test_analyze_evaluate_type_on_let_expression() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "declare let x: number");
        test.analyze_module(module_id);
        test.compile_check_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let types = dir.types.read();

        let let_expr_id = dir.roots[0];
        let expression = tree.get(let_expr_id);
        let &Expression::Statement {
            statement: let_expr_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let let_expression = tree.get(let_expr_id);
        let Expression::Let { declarators, .. } = let_expression else {
            panic!("expected let expression");
        };
        let declarator_id = declarators.first().unwrap();

        let let_ty = types
            .get_declared_type(declarator_id.into_global(module.id).into())
            .unwrap();

        assert_eq!(
            *let_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        );
    }

    #[test]
    fn test_analyze_evaluate_type_on_let_expression_int() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "declare let x: int");
        test.analyze_module(module_id);
        test.compile_check_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let types = dir.types.read();

        let let_expr_id = dir.roots[0];
        let expression = tree.get(let_expr_id);
        let &Expression::Statement {
            statement: let_expr_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let let_expression = tree.get(let_expr_id);
        let Expression::Let { declarators, .. } = let_expression else {
            panic!("expected let expression");
        };
        let declarator_id = declarators.first().unwrap();

        let let_ty = types
            .get_declared_type(declarator_id.into_global(module.id).into())
            .unwrap();

        // int resolves to Arbitrary { width: 32, is_signed: true } which is semantically Int32
        assert!(matches!(
            let_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Int(int_type))
            } if int_type.width() == Some(32) && int_type.is_signed()
        ));
    }
}
