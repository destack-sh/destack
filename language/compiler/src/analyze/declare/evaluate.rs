use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Argument, BinaryOperator, BindingKind, Declaration, DynamicKey, Expression, FunctionMode,
    FunctionSignature, LocalNodeId, LocalTypeId, Mutability, NodeTree, Property, StaticArgument,
    StaticExpression, SymbolTable, Type, TypeElement, TypeField, TypeIndexSignature, TypeLiteral,
    TypeMappedParameter, TypeTable, TypeUnaryOperator, UnaryOperator,
};
use destack_workspace::{Module, ProfileId};

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
        )?;
        let ty = types.get_type_mut(ty_id);
        *ty = evaluated_ty;
        types.clear_normalization_cache();

        Ok(())
    }

    /// Try to evaluate an Expression as a Type.
    /// Returns the evaluated Type value, or a Type::Unevaluated if it fails.
    pub(crate) fn try_evaluate_expression_to_type_value(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        let ty = self
            .evaluate_expression_to_type(module, profile, expression_id, tree, symbols, types)?
            .unwrap_or(Type::Unevaluated(expression_id));
        Ok(ty)
    }

    /// Try to evaluate an Expression as a Type id.
    pub(crate) fn try_evaluate_expression_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let ty = self.try_evaluate_expression_to_type_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
        )?;
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Evaluate a function signature into a Type.
    fn evaluate_function_signature_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        signature: &FunctionSignature,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
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

        // evaluate return type
        let return_type = if let Some(return_type_id) = signature.return_type {
            Some(self.try_evaluate_expression_to_type(
                module,
                profile,
                return_type_id,
                tree,
                symbols,
                types,
            )?)
        } else {
            None
        };

        // build function type
        Ok(Type::Function {
            asynchrony: signature.asynchrony,
            cardinality: signature.cardinality,
            static_parameters: Vec::new(),
            this_parameter: None,
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

            // capture the name for named arguments
            let name = match argument {
                Argument::Named { name, .. } => Some(*name),
                _ => None,
            };

            let expression_id = argument.value();

            // evaluate static values directly when possible
            if let Some(value) = self.evaluate_static_expression_value_for_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
            )? {
                evaluated_arguments.push(StaticArgument::Evaluated { name, value });
                continue;
            }

            // fall back to resolving the argument type
            let ty_id = self.try_evaluate_expression_to_type(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
            )?;

            // keep unevaluated types so later phases can resolve them
            if matches!(types.get_type(ty_id), Type::Unevaluated { .. }) {
                evaluated_arguments.push(StaticArgument::Unevaluated { node: *argument_id });
                continue;
            }

            // treat resolved types as static arguments
            evaluated_arguments.push(StaticArgument::Evaluated {
                name,
                value: StaticExpression::Type { ty: ty_id },
            });
        }

        Ok(Some(evaluated_arguments))
    }

    /// Evaluate an expression into a static value expression.
    #[allow(clippy::only_used_in_recursion)]
    fn evaluate_static_expression_value_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
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
            Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let start_value = self.evaluate_static_expression_value_for_type(
                    module, profile, *start, tree, symbols, types,
                )?;
                let end_value = self.evaluate_static_expression_value_for_type(
                    module, profile, *end, tree, symbols, types,
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
                    let value = self.evaluate_static_expression_value_for_type(
                        module,
                        profile,
                        element.value(),
                        tree,
                        symbols,
                        types,
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
                    let value = self.evaluate_static_expression_value_for_type(
                        module,
                        profile,
                        element.value(),
                        tree,
                        symbols,
                        types,
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

    /// Evaluate an Expression into a Type.
    fn evaluate_expression_to_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Type>> {
        // clone to avoid holding a tree borrow across recursive evaluation
        let expression = tree.get(expression_id).clone();

        let ty = match expression {
            Expression::ScalarLiteral { value } => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value.clone()),
            },
            Expression::TypeLiteral { value } => Type::TypeLiteral {
                value: value.clone(),
            },
            Expression::This => Type::This,
            Expression::Parenthesized { expression } => {
                return self.evaluate_expression_to_type(
                    module, profile, expression, tree, symbols, types,
                );
            }

            Expression::Declaration { declaration } => {
                let declaration = tree.get(declaration).clone();
                if let Declaration::Function { signature, .. } = declaration {
                    self.evaluate_function_signature_to_type(
                        module, profile, &signature, tree, symbols, types,
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
                    module, profile, right, tree, symbols, types,
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
                let type_id = self
                    .try_evaluate_expression_to_type(module, profile, left, tree, symbols, types)?;
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
                    module, profile, right, tree, symbols, types,
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
                    module, profile, right, tree, symbols, types,
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
                    module, profile, right, tree, symbols, types,
                )?;
                Type::PointerOf {
                    mutability,
                    right: type_id,
                }
            }
            // unary
            Expression::TypeUnary { operator, right } => {
                let right_id = self.try_evaluate_expression_to_type(
                    module, profile, right, tree, symbols, types,
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
                let left_id = self
                    .try_evaluate_expression_to_type(module, profile, left, tree, symbols, types)?;
                let right_id = self.try_evaluate_expression_to_type(
                    module, profile, right, tree, symbols, types,
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
                let left_id = self
                    .try_evaluate_expression_to_type(module, profile, left, tree, symbols, types)?;
                let right_id = self.try_evaluate_expression_to_type(
                    module, profile, right, tree, symbols, types,
                )?;
                let then_type_id = self.try_evaluate_expression_to_type(
                    module, profile, then_type, tree, symbols, types,
                )?;
                let else_type_id = self.try_evaluate_expression_to_type(
                    module, profile, else_type, tree, symbols, types,
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
                )?;
                let key_remap = parameter.key_remap.map(|key_remap| {
                    self.try_evaluate_expression_to_type(
                        module, profile, key_remap, tree, symbols, types,
                    )
                });
                let key_remap = match key_remap {
                    Some(Ok(key_remap)) => Some(key_remap),
                    Some(Err(error)) => return Err(error),
                    None => None,
                };
                let value_id = self.try_evaluate_expression_to_type(
                    module, profile, value, tree, symbols, types,
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
                let left_id = self
                    .try_evaluate_expression_to_type(module, profile, left, tree, symbols, types)?;
                let index_id = self.try_evaluate_expression_to_type(
                    module, profile, index, tree, symbols, types,
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
                            module, profile, *span, tree, symbols, types,
                        )
                    })
                    .collect::<AnalyzeResult<Vec<_>>>()?;
                Type::TemplateLiteral {
                    strings: strings.clone(),
                    spans,
                }
            }
            Expression::TypeImport { target, qualifier } => Type::Import {
                target,
                qualifier: qualifier.clone(),
            },
            Expression::TypeInfer { name, constraint } => {
                let constraint = constraint.map(|constraint| {
                    self.try_evaluate_expression_to_type(
                        module, profile, constraint, tree, symbols, types,
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
                        module, profile, target, tree, symbols, types,
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
            } => {
                let left_id = self
                    .try_evaluate_expression_to_type(module, profile, left, tree, symbols, types)?;
                let right_id = self.try_evaluate_expression_to_type(
                    module, profile, right, tree, symbols, types,
                )?;

                match operator {
                    BinaryOperator::ElementwiseOr => {
                        let mut elements = Vec::new();

                        if let Type::Union {
                            elements: union_elements,
                        } = types.get_type(left_id)
                        {
                            elements.extend(union_elements.iter().copied());
                        } else {
                            elements.push(left_id);
                        }

                        if let Type::Union {
                            elements: union_elements,
                        } = types.get_type(right_id)
                        {
                            elements.extend(union_elements.iter().copied());
                        } else {
                            elements.push(right_id);
                        }

                        Type::Union { elements }
                    }
                    BinaryOperator::ElementwiseAnd => {
                        let mut elements = Vec::new();

                        if let Type::Intersection {
                            elements: intersection_elements,
                        } = types.get_type(left_id)
                        {
                            elements.extend(intersection_elements.iter().copied());
                        } else {
                            elements.push(left_id);
                        }

                        if let Type::Intersection {
                            elements: intersection_elements,
                        } = types.get_type(right_id)
                        {
                            elements.extend(intersection_elements.iter().copied());
                        } else {
                            elements.push(right_id);
                        }

                        Type::Intersection { elements }
                    }
                    _ => return Ok(None),
                }
            }

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
                let static_arguments = self.evaluate_static_arguments(
                    module,
                    profile,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                )?;

                // normalize well known references into canonical structural types
                if let Some(normalized) = self.normalize_well_known_type_reference(
                    module,
                    symbols,
                    profile,
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
            Expression::TupleExpression { elements } => {
                // evaluate element types
                let mut element_types = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let value_id = {
                        let argument = tree.get(element_id);
                        argument.value()
                    };
                    let value_ty_id = self.try_evaluate_expression_to_type(
                        module, profile, value_id, tree, symbols, types,
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
                    node: expression_id.into_global_any(module.id),
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
                                    module, profile, key, tree, symbols, types,
                                )?;
                                let value_type = if let Some(value_id) = value {
                                    self.try_evaluate_expression_to_type(
                                        module, profile, value_id, tree, symbols, types,
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
                                self.static_key_from_dynamic_key(profile, key, tree)
                            }) else {
                                return Err(AnalyzeError::UnsupportedConstruct {
                                    node: property_id.into_global_any(module.id),
                                });
                            };

                            let ty = if let Some(value_id) = value {
                                self.try_evaluate_expression_to_type(
                                    module, profile, value_id, tree, symbols, types,
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
                                    module, profile, &signature, tree, symbols, types,
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
                                self.static_key_from_dynamic_key(profile, key, tree)
                            }) else {
                                return Err(AnalyzeError::UnsupportedConstruct {
                                    node: property_id.into_global_any(module.id),
                                });
                            };

                            let ty = self.evaluate_function_signature_to_type(
                                module, profile, &signature, tree, symbols, types,
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
                                node: property_id.into_global_any(module.id),
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
                        module, profile, left, tree, symbols, types,
                    )?;
                    Type::ArraySized {
                        element: left_id,
                        count: right,
                    }
                }
                // slice
                else {
                    let left_id = self.try_evaluate_expression_to_type(
                        module, profile, left, tree, symbols, types,
                    )?;
                    Type::Array {
                        element: Some(left_id),
                    }
                }
            }

            _ => return Ok(None),
        };

        Ok(Some(ty))
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
        test.compile_dump_clean();

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
        test.compile_dump_clean();

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
