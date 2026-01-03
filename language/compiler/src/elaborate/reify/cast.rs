use destack_dir::{
    Argument, Block, CastKind, CastSource, Declarator, Expression, IfKind, LocalNodeId,
    LocalTypeId, MatchCase, NodeTree, NodeType, SymbolTable, Type, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::r#type::{
    is_integer_type, is_nullable_union, is_pointer_type, is_scalar_literal_type, is_union_type,
    numeric_cast_kind,
};
use crate::{Compiler, ElaborateError, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify an explicit cast expression into a cast node.
    pub(super) fn reify_explicit_cast_expression(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        target_type: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // read the source and target type ids
        let value_type_id = types
            .get_declared_or_inferred_type_id(value.into_global_any(module_id))
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: value.into_global_any(module_id),
            })?;
        let target_type_id = types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: expression_id.into_global_any(module_id),
            })?;

        // classify the cast
        let kind = self.cast_kind_for_types(
            module_id,
            profile,
            symbols,
            types,
            value_type_id,
            target_type_id,
            module,
        );

        // replace the expression with a cast node
        tree.replace(
            expression_id,
            Expression::Cast {
                kind,
                source: CastSource::Explicit,
                value,
                target_type,
            },
        );

        Ok(())
    }

    /// Reify implicit casts in let and using bindings.
    pub(super) fn reify_implicit_casts_in_binding(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        declarators: &[LocalNodeId<Declarator>],
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // visit each declarator
        for declarator_id in declarators {
            // skip declarators without a value
            let declarator = tree.get(*declarator_id).clone();
            let Some(value_id) = declarator.value else {
                continue;
            };
            let Some(target_type_id) =
                types.get_declared_type_id(declarator_id.into_global_any(module_id))
            else {
                continue;
            };

            // wrap the value with a cast when needed
            let cast_value_id = self.wrap_value_with_cast(
                module_id,
                profile,
                value_id,
                value_id,
                target_type_id,
                tree,
                symbols,
                types,
                module,
            )?;

            // update the declarator when the value changes
            if cast_value_id != value_id {
                let updated = Declarator {
                    value: Some(cast_value_id),
                    ..declarator
                };
                tree.replace(*declarator_id, updated);
            }
        }

        Ok(())
    }

    /// Reify implicit casts in an assignment expression.
    pub(super) fn reify_implicit_casts_in_assignment(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // read the target type from the left hand side
        let target_type_id = types
            .get_declared_or_inferred_type_id(left.into_global_any(module_id))
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: left.into_global_any(module_id),
            })?;

        // wrap the right hand side when needed
        let cast_right_id = self.wrap_value_with_cast(
            module_id,
            profile,
            expression_id,
            right,
            target_type_id,
            tree,
            symbols,
            types,
            module,
        )?;

        // replace the assignment when the value changes
        if cast_right_id != right {
            tree.replace(
                expression_id,
                Expression::Assign {
                    left,
                    right: cast_right_id,
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in a return expression.
    pub(super) fn reify_implicit_casts_in_return(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        value: Option<LocalNodeId<Expression>>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // skip returns without values
        let Some(value_id) = value else {
            return Ok(());
        };

        // read the declared return type
        let Some(target_type_id) =
            self.enclosing_return_type(module_id, expression_id, tree, types)
        else {
            return Ok(());
        };

        // wrap the return value when needed
        let cast_value_id = self.wrap_value_with_cast(
            module_id,
            profile,
            expression_id,
            value_id,
            target_type_id,
            tree,
            symbols,
            types,
            module,
        )?;

        // replace the return when the value changes
        if cast_value_id != value_id {
            tree.replace(
                expression_id,
                Expression::Return {
                    value: Some(cast_value_id),
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in call arguments.
    pub(super) fn reify_implicit_casts_in_call(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // resolve expected types for arguments
        let Some(expected_argument_types) = self.expected_argument_types_for_call(
            module_id,
            expression_id,
            dynamic_arguments,
            tree,
            types,
        )?
        else {
            return Ok(());
        };

        // visit each argument and insert casts as needed
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            let Some(expected_type_id) = expected_argument_types.get(index).copied().flatten()
            else {
                continue;
            };

            let argument = tree.get(*argument_id).clone();
            let value_id = argument.value();

            let cast_value_id = self.wrap_value_with_cast(
                module_id,
                profile,
                expression_id,
                value_id,
                expected_type_id,
                tree,
                symbols,
                types,
                module,
            )?;

            if cast_value_id == value_id {
                continue;
            }

            let updated = match argument {
                Argument::Named { name, .. } => Argument::Named {
                    name,
                    value: cast_value_id,
                },
                Argument::Labeled { label, .. } => Argument::Labeled {
                    label,
                    value: cast_value_id,
                },
                Argument::Positional { .. } => Argument::Positional {
                    value: cast_value_id,
                },
                Argument::Spread { label, .. } => Argument::Spread {
                    label,
                    value: cast_value_id,
                },
            };
            tree.replace(*argument_id, updated);
        }

        Ok(())
    }

    /// Reify implicit casts in ternary expressions.
    pub(super) fn reify_implicit_casts_in_ternary(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // use the expression type as the target for both branches
        let Some(target_type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };

        // cast the then branch when needed
        let cast_then_id = self.wrap_value_with_cast(
            module_id,
            profile,
            expression_id,
            then_expression,
            target_type_id,
            tree,
            symbols,
            types,
            module,
        )?;

        // cast the else branch when present
        let cast_else_id = if let Some(else_expression) = else_expression {
            let cast_else_id = self.wrap_value_with_cast(
                module_id,
                profile,
                expression_id,
                else_expression,
                target_type_id,
                tree,
                symbols,
                types,
                module,
            )?;
            Some(cast_else_id)
        } else {
            None
        };

        // update the ternary expression when any branch changes
        if cast_then_id != then_expression || cast_else_id != else_expression {
            tree.replace(
                expression_id,
                Expression::If {
                    kind: IfKind::Ternary,
                    condition,
                    then_expression: cast_then_id,
                    else_expression: cast_else_id,
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in match case bodies.
    pub(super) fn reify_implicit_casts_in_match(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // use the match expression type as the target
        let Some(target_type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };

        // visit each case and cast the produced value
        for case_id in cases {
            let case = tree.get(*case_id).clone();
            match case {
                MatchCase::Expression {
                    selector,
                    body,
                    scope,
                } => {
                    let cast_body_id = self.wrap_value_with_cast(
                        module_id,
                        profile,
                        expression_id,
                        body,
                        target_type_id,
                        tree,
                        symbols,
                        types,
                        module,
                    )?;

                    if cast_body_id != body {
                        tree.replace(
                            *case_id,
                            MatchCase::Expression {
                                selector,
                                body: cast_body_id,
                                scope,
                            },
                        );
                    }
                }
                MatchCase::Block {
                    selector: _,
                    body,
                    scope: _,
                } => {
                    let block = tree.get(body).clone();
                    let Some(last_expression_id) = block.expressions.last().copied() else {
                        continue;
                    };

                    let cast_last_id = self.wrap_value_with_cast(
                        module_id,
                        profile,
                        expression_id,
                        last_expression_id,
                        target_type_id,
                        tree,
                        symbols,
                        types,
                        module,
                    )?;

                    if cast_last_id != last_expression_id {
                        let mut expressions = block.expressions;
                        let Some(last_expression) = expressions.last_mut() else {
                            continue;
                        };
                        *last_expression = cast_last_id;
                        tree.replace(
                            body,
                            Block {
                                scope: block.scope,
                                expressions,
                            },
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Wrap a value in a cast when the target type differs.
    fn wrap_value_with_cast(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // read the source type id
        let value_type_id = types
            .get_declared_or_inferred_type_id(value_id.into_global_any(module_id))
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: value_id.into_global_any(module_id),
            })?;

        // skip when the types already match
        if value_type_id == target_type_id {
            return Ok(value_id);
        }

        // classify the cast
        let kind = self.cast_kind_for_types(
            module_id,
            profile,
            symbols,
            types,
            value_type_id,
            target_type_id,
            module,
        );

        // skip numeric casts for scalar literals
        let value_type = types.get_type(value_type_id);
        if is_scalar_literal_type(value_type) && is_numeric_cast_kind(kind) {
            return Ok(value_id);
        }
        if kind == CastKind::Identity {
            return Ok(value_id);
        }

        // build the target type expression
        let scope = tree.get_scope(origin_id);
        let target_expression_id =
            tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let target_expression = match types.get_type(target_type_id) {
            Type::TypeLiteral { value } => Expression::TypeLiteral {
                value: value.clone(),
            },
            _ => Expression::Type {
                value: target_type_id,
            },
        };
        let target_expression_id = tree.insert(target_expression_id, target_expression);

        // set the inferred type for the target type expression
        let target_type_value = Type::Value {
            value: target_type_id,
        };
        let target_type_value_id = types.insert_type_from(target_type_value, target_expression_id);
        types.set_inferred_type(
            target_expression_id.into_global_any(module_id),
            target_type_value_id,
        );

        // insert the cast expression
        let cast_expression_id =
            tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let cast_expression_id = tree.insert(
            cast_expression_id,
            Expression::Cast {
                kind,
                source: CastSource::Implicit,
                value: value_id,
                target_type: target_expression_id,
            },
        );
        types.set_inferred_type(
            cast_expression_id.into_global_any(module_id),
            target_type_id,
        );

        Ok(cast_expression_id)
    }

    /// Classify the cast kind for two types.
    fn cast_kind_for_types(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        source_id: LocalTypeId,
        target_id: LocalTypeId,
        module: &Module,
    ) -> CastKind {
        // fast path for identical types
        if source_id == target_id {
            return CastKind::Identity;
        }

        // read the source and target types
        let source = types.get_type(source_id).clone();
        let target = types.get_type(target_id).clone();

        // handle numeric casts first
        if let Some(kind) = numeric_cast_kind(&source, &target) {
            return kind;
        }

        // handle pointer casts
        if is_pointer_type(&source) && is_integer_type(&target) {
            return CastKind::PointerToInt;
        }
        if is_integer_type(&source) && is_pointer_type(&target) {
            return CastKind::IntToPointer;
        }
        if is_pointer_type(&source) && is_pointer_type(&target) {
            return CastKind::PointerCast;
        }

        // handle nullable casts
        if is_nullable_union(&target, types) {
            return CastKind::NullableUpcast;
        }
        if is_nullable_union(&source, types) {
            return CastKind::NullableDowncast;
        }

        // handle union casts
        if is_union_type(&target) {
            return CastKind::UnionUpcast;
        }
        if is_union_type(&source) {
            return CastKind::UnionDowncast;
        }

        // fall back to assignability based instance casts
        let options = self.analyze_context_options_for_module(module_id);
        let assignable = self.is_type_assignable(
            module, profile, symbols, target_id, source_id, types, &options,
        );
        if assignable.is_assignable() {
            CastKind::InstanceUpcast
        } else {
            CastKind::InstanceDowncast
        }
    }
}

/// Check whether a cast kind is numeric.
fn is_numeric_cast_kind(kind: CastKind) -> bool {
    matches!(
        kind,
        CastKind::IntWiden
            | CastKind::IntNarrow
            | CastKind::IntSignChange
            | CastKind::FloatWiden
            | CastKind::FloatNarrow
            | CastKind::IntToFloat
            | CastKind::FloatToInt
    )
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_reify_implicit_cast_in_binding() {
        // binding casts are inserted for mismatched types
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    let value: float = intValue();
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    let value = intValue() as float;
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_assignment() {
        // assignment casts are inserted for mismatched types
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    let value: float = intValue();
    value = intValue();
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            // FUGU: make casts (and/or unbind?) more specific than "float"?
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    let value = intValue() as float;
    value = intValue() as float;
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_return() {
        // return casts are inserted for declared return types
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    return intValue();
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    return intValue() as float;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_call_argument() {
        // call arguments are cast to parameter types
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function takeFloat(value: float): float {
    return value;
}

function test(): float {
    return takeFloat(intValue());
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function takeFloat(value): float {
    return value;
}

function test(): float {
    return takeFloat(intValue() as float);
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_ternary() {
        // ternary branches cast to the expression type
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function floatValue(): float {
    return 1;
}

function intValue(): int32 {
    return 1;
}

function test(condition: boolean): float {
    return condition ? floatValue() : intValue();
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function floatValue(): float {
    return 1;
}

function intValue(): int32 {
    return 1;
}

function test(condition): float {
    return condition ? floatValue() : intValue() as float;
}
"#,
        );
    }

    #[test]
    fn test_reify_explicit_cast_expression() {
        // explicit casts stay explicit
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    return intValue() as float;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    return intValue() as float64;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_using_binding() {
        // using bindings cast initializers when needed
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): void {
    using value: float = intValue();
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): void {
    using value = intValue() as float;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_skip_same_type() {
        // matching types do not insert casts
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 {
    let value: int32 = intValue();
    return value;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 {
    let value = intValue();
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_skip_numeric_literal() {
        // scalar literal numeric casts are omitted
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): float {
    let value: float = 1;
    return value;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated
        test.assert_elaborated(
            module_id,
            r#"
function test(): float {
    let value = 1;
    return value;
}
"#,
        );
    }
}
