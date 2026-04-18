use destack_codegen_js as js;
use destack_workspace::EsTarget;

use super::linker::Rewriter;

impl Rewriter<'_, '_> {
    /// Fold one comparison expression.
    pub(super) fn fold_comparison_expression(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<(js::Expression, Option<js::ScriptSymbolId>)> {
        if let Some(expression) = self.fold_literal_comparison(left, operator, right) {
            return Some((expression, None));
        }

        if let Some(expression) = self.fold_typeof_undefined_comparison(left, operator, right) {
            return Some((expression, None));
        }

        self.fold_loose_nullish_comparison(left, operator, right)
            .map(|expression| (expression, None))
    }

    /// Fold one binary expression.
    pub(super) fn fold_binary_expression(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<(js::Expression, Option<js::ScriptSymbolId>)> {
        match operator {
            js::BinaryOperator::EqualStrict | js::BinaryOperator::NotEqualStrict => {
                if let Some(expression) = self.fold_literal_comparison(left, operator, right) {
                    return Some((expression, None));
                }

                self.fold_strict_comparison(left, operator, right)
                    .map(|expression| (expression, None))
            }

            js::BinaryOperator::LessThan
            | js::BinaryOperator::LessThanOrEqual
            | js::BinaryOperator::GreaterThan
            | js::BinaryOperator::GreaterThanOrEqual => self
                .fold_literal_comparison(left, operator, right)
                .map(|expression| (expression, None)),

            js::BinaryOperator::Or => {
                if let Some(expression) = self.fold_associative_binary_chain(left, operator, right)
                {
                    return Some((expression, None));
                }

                if let Some(expression) = self.fold_nullish_disjunction(left, operator, right) {
                    return Some((expression, None));
                }

                self.fold_literal_logical_with_symbol(self.module, left, operator, right)
            }

            js::BinaryOperator::And => {
                if let Some(expression) = self.fold_associative_binary_chain(left, operator, right)
                {
                    return Some((expression, None));
                }

                if let Some(expression) = self.fold_nullish_conjunction(left, operator, right) {
                    return Some((expression, None));
                }

                self.fold_literal_logical_with_symbol(self.module, left, operator, right)
            }

            js::BinaryOperator::Coalesce => {
                if let Some(expression) = self.fold_associative_binary_chain(left, operator, right)
                {
                    return Some((expression, None));
                }

                self.fold_nullish_coalescing_with_symbol(left, right)
            }

            js::BinaryOperator::Add
            | js::BinaryOperator::Subtract
            | js::BinaryOperator::Multiply
            | js::BinaryOperator::Divide
            | js::BinaryOperator::Remainder
            | js::BinaryOperator::Exponent
            | js::BinaryOperator::ShiftLeft
            | js::BinaryOperator::ShiftRight
            | js::BinaryOperator::UnsignedShiftRight
            | js::BinaryOperator::ElementwiseAnd
            | js::BinaryOperator::ElementwiseXor
            | js::BinaryOperator::ElementwiseOr => self
                .fold_numeric_binary_expression(self.module, left, operator, right)
                .map(|expression| (expression, None)),

            _ => None,
        }
    }

    /// Fold one ternary expression.
    pub(super) fn fold_ternary_expression(
        &mut self,
        expression_id: js::LocalNodeId<js::Expression>,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<(js::Expression, Option<js::ScriptSymbolId>)> {
        if self.output_uses_es2020_syntax() {
            if let Some(expression) =
                self.fold_es2020_ternary(condition, then_expression, else_expression)
            {
                return Some((expression, None));
            }
        }

        if let Some(rewritten) = self.fold_literal_ternary_with_symbol(
            self.module,
            condition,
            then_expression,
            else_expression,
        ) {
            return Some(rewritten);
        }

        if let Some(expression) =
            self.fold_boolean_ternary(condition, then_expression, else_expression)
        {
            return Some((
                expression,
                self.module
                    .tree
                    .symbol(condition)
                    .or(self.module.tree.symbol(expression_id)),
            ));
        }

        None
    }

    /// Return whether output syntax minification may introduce ES2020 syntax.
    fn output_uses_es2020_syntax(&self) -> bool {
        matches!(
            self.target.es_target,
            EsTarget::Es2020
                | EsTarget::Es2021
                | EsTarget::Es2022
                | EsTarget::Es2023
                | EsTarget::Es2024
                | EsTarget::EsNext
        )
    }

    /// Fold one strict or loose comparison when a shorter nullish form exists.
    pub(super) fn fold_loose_nullish_comparison(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let (left, right) = self.canonicalize_comparison_operands(left, right);

        if Self::is_undefined_expression(self.module, right) {
            let right = Self::insert_null_literal(self.module, right);

            return Some(js::Expression::Binary {
                left,
                operator,
                right,
            });
        }

        None
    }

    /// Fold one `typeof x == "undefined"` style comparison to a shorter string test.
    pub(super) fn fold_typeof_undefined_comparison(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let (left, right) = self.typeof_undefined_comparison_pair(left, right)?;
        let operator = match operator {
            js::BinaryOperator::Equal | js::BinaryOperator::EqualStrict => {
                js::BinaryOperator::GreaterThan
            }
            js::BinaryOperator::NotEqual | js::BinaryOperator::NotEqualStrict => {
                js::BinaryOperator::LessThan
            }
            _ => return None,
        };
        let right = self.module.tree.insert_from(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::String(self.module.strings.intern("u")),
            },
            right,
        );

        Some(js::Expression::Binary {
            left,
            operator,
            right,
        })
    }

    /// Fold one nullish coalescing expression when the left side is already known.
    pub(super) fn fold_nullish_coalescing(
        &self,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        // always right
        if Self::is_always_nullish_without_side_effects(self.module, left) {
            return Some(self.module.tree.get(right).clone());
        }

        // always left
        if Self::is_never_nullish(self.module, left) {
            return Some(self.module.tree.get(left).clone());
        }

        None
    }
}

impl Rewriter<'_, '_> {
    /// Fold one literal short circuit when the left operand already decides the result.
    fn fold_literal_logical(
        &self,
        module: &js::ScriptModule,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let Some(left_truthiness) = self.literal_truthiness(module, left) else {
            return None;
        };

        match operator {
            js::BinaryOperator::And => {
                if left_truthiness {
                    Some(module.tree.get(right).clone())
                } else {
                    Some(module.tree.get(left).clone())
                }
            }
            js::BinaryOperator::Or => {
                if left_truthiness {
                    Some(module.tree.get(left).clone())
                } else {
                    Some(module.tree.get(right).clone())
                }
            }
            _ => None,
        }
    }

    /// Fold one lowered ternary back into one ES2020 syntax form when possible.
    pub(super) fn fold_es2020_ternary(
        &mut self,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        // nullish coalescing
        if let Some(rewritten) =
            self.fold_es2020_nullish_coalescing_ternary(condition, then_expression, else_expression)
        {
            return Some(rewritten);
        }

        // optional chaining
        self.fold_es2020_optional_chain_ternary(condition, then_expression, else_expression)
    }

    /// Fold one lowered ternary back into `??` when the value flow matches.
    fn fold_es2020_nullish_coalescing_ternary(
        &mut self,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let (value, is_non_nullish) = self.loose_nullish_test(condition)?;

        // `value != null ? value : fallback`
        if is_non_nullish && Self::same_expression_value(self.module, value, then_expression) {
            return Some(js::Expression::Binary {
                left: value,
                operator: js::BinaryOperator::Coalesce,
                right: else_expression,
            });
        }

        // `value == null ? fallback : value`
        if !is_non_nullish && Self::same_expression_value(self.module, value, else_expression) {
            return Some(js::Expression::Binary {
                left: value,
                operator: js::BinaryOperator::Coalesce,
                right: then_expression,
            });
        }

        None
    }

    /// Fold one lowered ternary back into optional chaining when the guarded value matches.
    fn fold_es2020_optional_chain_ternary(
        &mut self,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let (value, is_non_nullish) = self.loose_nullish_test(condition)?;

        // `value != null ? value.member : undefined`
        if is_non_nullish && Self::is_undefined_expression(self.module, else_expression) {
            return Self::rewrite_optional_chain_expression(self.module, value, then_expression);
        }

        // `value == null ? undefined : value.member`
        if !is_non_nullish && Self::is_undefined_expression(self.module, then_expression) {
            return Self::rewrite_optional_chain_expression(self.module, value, else_expression);
        }

        None
    }

    /// Return one loose nullish test value and whether it tests for non-nullish values.
    fn loose_nullish_test(
        &self,
        condition: js::LocalNodeId<js::Expression>,
    ) -> Option<(js::LocalNodeId<js::Expression>, bool)> {
        let js::Expression::Binary {
            left,
            operator,
            right,
        } = self.module.tree.get(condition).clone()
        else {
            return None;
        };

        let (left, right) = self.canonicalize_comparison_operands(left, right);

        if !Self::is_null_expression(self.module, right) {
            return None;
        }

        match operator {
            js::BinaryOperator::Equal => Some((left, false)),
            js::BinaryOperator::NotEqual => Some((left, true)),
            _ => None,
        }
    }
}

impl Rewriter<'_, '_> {
    /// Fold one literal ternary when the condition already chooses the branch.
    fn fold_literal_ternary(
        &self,
        module: &js::ScriptModule,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let Some(condition_truthiness) = self.literal_truthiness(module, condition) else {
            return None;
        };

        if condition_truthiness {
            Some(module.tree.get(then_expression).clone())
        } else {
            Some(module.tree.get(else_expression).clone())
        }
    }

    /// Fold one nullish coalescing expression and preserve the surviving branch symbol.
    fn fold_nullish_coalescing_with_symbol(
        &self,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<(js::Expression, Option<js::ScriptSymbolId>)> {
        let expression = self.fold_nullish_coalescing(left, right)?;

        let symbol_id = if Rewriter::is_always_nullish_without_side_effects(self.module, left) {
            self.module.tree.symbol(right)
        } else if Rewriter::is_never_nullish(self.module, left) {
            self.module.tree.symbol(left)
        } else {
            None
        };

        Some((expression, symbol_id))
    }
}

impl Rewriter<'_, '_> {
    /// Fold one literal logical expression and preserve the surviving branch symbol.
    fn fold_literal_logical_with_symbol(
        &self,
        module: &js::ScriptModule,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<(js::Expression, Option<js::ScriptSymbolId>)> {
        let expression = self.fold_literal_logical(module, left, operator, right)?;

        let left_truthiness = self.literal_truthiness(module, left)?;
        let symbol_id = match operator {
            js::BinaryOperator::And => {
                if left_truthiness {
                    module.tree.symbol(right)
                } else {
                    module.tree.symbol(left)
                }
            }
            js::BinaryOperator::Or => {
                if left_truthiness {
                    module.tree.symbol(left)
                } else {
                    module.tree.symbol(right)
                }
            }
            _ => None,
        };

        Some((expression, symbol_id))
    }

    /// Fold one literal ternary and preserve the chosen branch symbol.
    fn fold_literal_ternary_with_symbol(
        &self,
        module: &js::ScriptModule,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<(js::Expression, Option<js::ScriptSymbolId>)> {
        let expression =
            self.fold_literal_ternary(module, condition, then_expression, else_expression)?;

        let condition_truthiness = self.literal_truthiness(module, condition)?;
        let symbol_id = if condition_truthiness {
            module.tree.symbol(then_expression)
        } else {
            module.tree.symbol(else_expression)
        };

        Some((expression, symbol_id))
    }

    /// Fold one right nested logical chain to one flatter left associative chain.
    pub(super) fn fold_associative_binary_chain(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        if !matches!(
            operator,
            js::BinaryOperator::Or | js::BinaryOperator::And | js::BinaryOperator::Coalesce
        ) {
            return None;
        }

        let js::Expression::Binary {
            left: right_left,
            operator: right_operator,
            right: right_right,
        } = self.module.tree.get(right).clone()
        else {
            return None;
        };

        if right_operator != operator {
            return None;
        }

        let left = self.module.tree.insert_from(
            js::Expression::Binary {
                left,
                operator,
                right: right_left,
            },
            right,
        );

        Some(js::Expression::Binary {
            left,
            operator,
            right: right_right,
        })
    }

    /// Fold one strict comparison when a shorter loose form exists.
    pub(super) fn fold_strict_comparison(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        if let Some(rewritten) = self.fold_typeof_undefined_comparison(left, operator, right) {
            return Some(rewritten);
        }

        if let Some((left, right)) = self.typeof_string_comparison_pair(left, right) {
            let operator = match operator {
                js::BinaryOperator::EqualStrict => js::BinaryOperator::Equal,
                js::BinaryOperator::NotEqualStrict => js::BinaryOperator::NotEqual,
                _ => unreachable!("strict comparison rewrite only accepts strict operators"),
            };

            return Some(js::Expression::Binary {
                left,
                operator,
                right,
            });
        }

        None
    }

    /// Fold one primitive literal comparison to its boolean result.
    pub(super) fn fold_literal_comparison(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let left = Self::scalar_comparison_literal_expression(self.module, left)?;
        let right = Self::scalar_comparison_literal_expression(self.module, right)?;
        let value = Self::evaluate_literal_comparison(self.module, &left, operator, &right)?;

        Some(js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::Boolean(value),
        })
    }

    /// Fold one sequence expression by dropping removable prefix expressions.
    pub(super) fn fold_sequence_expression(
        &self,
        expressions: &[js::LocalNodeId<js::Expression>],
    ) -> Option<(js::Expression, Option<js::ScriptSymbolId>)> {
        let first_live_index = expressions.iter().position(|expression_id| {
            !Self::is_removable_sequence_prefix(self.module, *expression_id)
        })?;

        if first_live_index == 0 {
            return None;
        }

        let remaining = &expressions[first_live_index..];

        if remaining.len() == 1 {
            return Some((
                self.module.tree.get(remaining[0]).clone(),
                self.module.tree.symbol(remaining[0]),
            ));
        }

        Some((
            js::Expression::SequenceExpression {
                expressions: remaining.to_vec(),
            },
            None,
        ))
    }

    /// Fold one boolean ternary to one shorter boolean form.
    pub(super) fn fold_boolean_ternary(
        &mut self,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let then_value = Self::boolean_literal_value(self.module, then_expression)?;
        let else_value = Self::boolean_literal_value(self.module, else_expression)?;

        if then_value == else_value {
            return Some(js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::Boolean(then_value),
            });
        }

        if then_value && !else_value {
            let condition = self.module.tree.insert_from(
                js::Expression::Unary {
                    operator: js::UnaryOperator::Not,
                    right: condition,
                },
                condition,
            );

            return Some(js::Expression::Unary {
                operator: js::UnaryOperator::Not,
                right: condition,
            });
        }

        if !then_value && else_value {
            return Some(js::Expression::Unary {
                operator: js::UnaryOperator::Not,
                right: condition,
            });
        }

        None
    }
}

impl Rewriter<'_, '_> {
    /// Rewrite one unary expression when the result is already known.
    pub(super) fn fold_unary_expression(
        &mut self,
        operator: js::UnaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        match operator {
            js::UnaryOperator::Void => self.fold_static_unary_expression(operator, right),
            js::UnaryOperator::Plus => {
                let scalar = self.output_scalar_literal_expression(self.module, right)?;

                match scalar {
                    js::ScalarLiteral::Number(value) => Some(js::Expression::ScalarLiteral {
                        value: js::ScalarLiteral::Number(value),
                    }),
                    _ => None,
                }
            }
            js::UnaryOperator::Negate => {
                let scalar = self.output_scalar_literal_expression(self.module, right)?;

                match scalar {
                    js::ScalarLiteral::Number(value) => Some(js::Expression::ScalarLiteral {
                        value: js::ScalarLiteral::Number(-value),
                    }),
                    _ => None,
                }
            }
            js::UnaryOperator::ElementwiseNot => {
                let scalar = self.output_scalar_literal_expression(self.module, right)?;

                match scalar {
                    js::ScalarLiteral::Number(value) => Some(js::Expression::ScalarLiteral {
                        value: js::ScalarLiteral::Number((!Self::js_to_int32(value)) as f64),
                    }),
                    _ => None,
                }
            }
            _ => self.fold_static_unary_expression(operator, right),
        }
    }

    /// Fold one unary expression when the result is already known.
    pub(super) fn fold_static_unary_expression(
        &self,
        operator: js::UnaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        match operator {
            js::UnaryOperator::Void => {
                if !Self::is_removable_sequence_prefix(self.module, right) {
                    return None;
                }

                Some(js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Undefined,
                })
            }
            js::UnaryOperator::Plus => {
                let scalar = Self::scalar_literal_expression(self.module, right)?;

                match scalar {
                    js::ScalarLiteral::Number(value) => Some(js::Expression::ScalarLiteral {
                        value: js::ScalarLiteral::Number(value),
                    }),
                    _ => None,
                }
            }
            js::UnaryOperator::Negate => {
                let scalar = Self::scalar_literal_expression(self.module, right)?;

                match scalar {
                    js::ScalarLiteral::Number(value) => Some(js::Expression::ScalarLiteral {
                        value: js::ScalarLiteral::Number(-value),
                    }),
                    _ => None,
                }
            }
            js::UnaryOperator::Not => {
                let truthiness = Self::literal_truthiness_static(self.module, right)?;

                Some(js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Boolean(!truthiness),
                })
            }
            js::UnaryOperator::PostIncrement
            | js::UnaryOperator::PostDecrement
            | js::UnaryOperator::PreIncrement
            | js::UnaryOperator::PreDecrement
            | js::UnaryOperator::ElementwiseNot
            | js::UnaryOperator::Typeof => None,
        }
    }

    /// Rewrite one binary expression when both sides are scalar numbers.
    fn fold_numeric_binary_expression(
        &self,
        module: &js::ScriptModule,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let js::ScalarLiteral::Number(left) =
            self.output_scalar_literal_expression(module, left)?
        else {
            return None;
        };
        let js::ScalarLiteral::Number(right) =
            self.output_scalar_literal_expression(module, right)?
        else {
            return None;
        };

        let value = match operator {
            js::BinaryOperator::Add => left + right,
            js::BinaryOperator::Subtract => left - right,
            js::BinaryOperator::Multiply => left * right,
            js::BinaryOperator::Divide => left / right,
            js::BinaryOperator::Remainder => left % right,
            js::BinaryOperator::Exponent => left.powf(right),
            js::BinaryOperator::ShiftLeft => {
                (Self::js_to_int32(left) << (Self::js_to_uint32(right) & 31)) as f64
            }
            js::BinaryOperator::ShiftRight => {
                (Self::js_to_int32(left) >> (Self::js_to_uint32(right) & 31)) as f64
            }
            js::BinaryOperator::UnsignedShiftRight => {
                (Self::js_to_uint32(left) >> (Self::js_to_uint32(right) & 31)) as f64
            }
            js::BinaryOperator::ElementwiseAnd => {
                (Self::js_to_int32(left) & Self::js_to_int32(right)) as f64
            }
            js::BinaryOperator::ElementwiseXor => {
                (Self::js_to_int32(left) ^ Self::js_to_int32(right)) as f64
            }
            js::BinaryOperator::ElementwiseOr => {
                (Self::js_to_int32(left) | Self::js_to_int32(right)) as f64
            }
            _ => return None,
        };

        Some(js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::Number(value),
        })
    }

    /// Convert one number with JS `ToInt32` semantics.
    fn js_to_int32(value: f64) -> i32 {
        if !value.is_finite() || value == 0.0 {
            return 0;
        }

        let truncated = value.trunc();
        let modulo = truncated.rem_euclid(4294967296.0);

        if modulo >= 2147483648.0 {
            (modulo - 4294967296.0) as i32
        } else {
            modulo as i32
        }
    }

    /// Convert one number with JS `ToUint32` semantics.
    fn js_to_uint32(value: f64) -> u32 {
        if !value.is_finite() || value == 0.0 {
            return 0;
        }

        value.trunc().rem_euclid(4294967296.0) as u32
    }

    /// Rewrite one `a === null || a === undefined` style disjunction.
    pub(super) fn fold_nullish_disjunction(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let js::BinaryOperator::Or = operator else {
            return None;
        };

        let (value, nullish) =
            self.nullish_comparison_pair(left, right, js::BinaryOperator::EqualStrict)?;

        Some(js::Expression::Binary {
            left: value,
            operator: js::BinaryOperator::Equal,
            right: nullish,
        })
    }

    /// Rewrite one `a !== null && a !== undefined` style conjunction.
    pub(super) fn fold_nullish_conjunction(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let js::BinaryOperator::And = operator else {
            return None;
        };

        let (value, nullish) =
            self.nullish_comparison_pair(left, right, js::BinaryOperator::NotEqualStrict)?;

        Some(js::Expression::Binary {
            left: value,
            operator: js::BinaryOperator::NotEqual,
            right: nullish,
        })
    }

    /// Return one shared comparison value and one null literal for one nullish comparison pair.
    fn nullish_comparison_pair(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
    ) -> Option<(
        js::LocalNodeId<js::Expression>,
        js::LocalNodeId<js::Expression>,
    )> {
        let js::Expression::Binary {
            left: left_value,
            operator: left_operator,
            right: left_primitive,
        } = self.module.tree.get(left).clone()
        else {
            return None;
        };
        let js::Expression::Binary {
            left: right_value,
            operator: right_operator,
            right: right_primitive,
        } = self.module.tree.get(right).clone()
        else {
            return None;
        };

        if left_operator != operator || right_operator != operator {
            return None;
        }

        if !Self::same_expression_value(self.module, left_value, right_value) {
            return None;
        }

        let left_is_null = Self::is_null_expression(self.module, left_primitive);
        let left_is_undefined = Self::is_undefined_expression(self.module, left_primitive);
        let right_is_null = Self::is_null_expression(self.module, right_primitive);
        let right_is_undefined = Self::is_undefined_expression(self.module, right_primitive);

        if !(left_is_null && right_is_undefined || left_is_undefined && right_is_null) {
            return None;
        }

        let nullish = Self::insert_null_literal(self.module, left_primitive);

        Some((left_value, nullish))
    }

    /// Return one normalized `typeof x` and `"undefined"` comparison pair.
    fn typeof_undefined_comparison_pair(
        &self,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<(
        js::LocalNodeId<js::Expression>,
        js::LocalNodeId<js::Expression>,
    )> {
        let (left, right) = self.canonicalize_comparison_operands(left, right);

        if Self::is_typeof_expression(self.module, left)
            && Self::is_undefined_string_expression(self.module, right)
        {
            return Some((left, right));
        }

        None
    }

    /// Return one normalized `typeof x` and string literal comparison pair.
    fn typeof_string_comparison_pair(
        &self,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<(
        js::LocalNodeId<js::Expression>,
        js::LocalNodeId<js::Expression>,
    )> {
        let (left, right) = self.canonicalize_comparison_operands(left, right);

        if Self::is_typeof_expression(self.module, left)
            && Self::is_string_literal_expression(self.module, right)
        {
            return Some((left, right));
        }

        None
    }

    /// Return one canonical comparison operand order for primitive literal comparisons.
    fn canonicalize_comparison_operands(
        &self,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> (
        js::LocalNodeId<js::Expression>,
        js::LocalNodeId<js::Expression>,
    ) {
        if Self::is_primitive_literal_expression(self.module, left)
            && !Self::is_primitive_literal_expression(self.module, right)
        {
            return (right, left);
        }

        (left, right)
    }

    /// Rewrite one global `Infinity` reference to `1 / 0`.
    pub(super) fn fold_global_infinity_reference(
        &mut self,
        expression_id: js::LocalNodeId<js::Expression>,
        path: js::Path,
    ) -> Option<js::Expression> {
        if path.segments.len() != 1 || self.module.strings.get(path.segments[0]) != "Infinity" {
            return None;
        }

        let Some(js::ScalarLiteral::Number(value)) =
            self.output_global_scalar_literal_expression(self.module, expression_id, &path)
        else {
            return None;
        };

        if value != f64::INFINITY {
            return None;
        }

        let one = self.module.tree.insert_from(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::Number(1.0),
            },
            expression_id,
        );
        let zero = self.module.tree.insert_from(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::Number(0.0),
            },
            expression_id,
        );

        Some(js::Expression::Binary {
            left: one,
            operator: js::BinaryOperator::Divide,
            right: zero,
        })
    }

    /// Rewrite one literal `typeof` to its string result.
    pub(super) fn fold_typeof_literal(
        &mut self,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let kind = Self::typeof_literal_kind(self.module, right)?;

        Some(js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(self.module.strings.intern(kind)),
        })
    }

    /// Rewrite one guarded receiver chain into optional chaining.
    fn rewrite_optional_chain_expression(
        module: &mut js::ScriptModule,
        receiver: js::LocalNodeId<js::Expression>,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        let rewritten = Self::rewrite_optional_chain_receiver(module, receiver, expression_id)?;

        Some(module.tree.get(rewritten).clone())
    }

    /// Rewrite one receiver occurrence inside one expression tree into an optional receiver.
    fn rewrite_optional_chain_receiver(
        module: &mut js::ScriptModule,
        receiver: js::LocalNodeId<js::Expression>,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<js::LocalNodeId<js::Expression>> {
        // direct receiver
        if Self::same_expression_value(module, receiver, expression_id) {
            let rewritten = js::Expression::Maybe {
                position: Self::optional_receiver_position(module, expression_id),
                left: expression_id,
            };

            return Some(module.tree.insert_from(rewritten, expression_id));
        }

        let expression = module.tree.get(expression_id).clone();

        // recurse down the receiver side
        let rewritten = match expression {
            js::Expression::Parenthesized { expression } => {
                let expression =
                    Self::rewrite_optional_chain_receiver(module, receiver, expression)?;

                js::Expression::Parenthesized { expression }
            }
            js::Expression::Member { left, name } => {
                let left = Self::rewrite_optional_chain_receiver(module, receiver, left)?;

                js::Expression::Member { left, name }
            }
            js::Expression::Index {
                position,
                left,
                right,
            } => {
                let left = Self::rewrite_optional_chain_receiver(module, receiver, left)?;
                let position = if matches!(module.tree.get(left), js::Expression::Maybe { .. }) {
                    js::PostfixPosition::Indirect
                } else {
                    position
                };

                js::Expression::Index {
                    position,
                    left,
                    right,
                }
            }
            js::Expression::Call {
                position,
                left,
                generic_arguments,
                arguments,
            } => {
                let left = Self::rewrite_optional_chain_receiver(module, receiver, left)?;
                let position = if matches!(module.tree.get(left), js::Expression::Maybe { .. }) {
                    js::PostfixPosition::Indirect
                } else {
                    position
                };

                js::Expression::Call {
                    position,
                    left,
                    generic_arguments,
                    arguments,
                }
            }
            _ => return None,
        };

        Some(module.tree.insert_from(rewritten, expression_id))
    }

    /// Return the postfix position for one wrapped optional receiver.
    fn optional_receiver_position(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> js::PostfixPosition {
        if matches!(
            module.tree.get(expression_id),
            js::Expression::Maybe { .. } | js::Expression::Must { .. }
        ) {
            js::PostfixPosition::Indirect
        } else {
            js::PostfixPosition::Direct
        }
    }

    /// Return whether one expression is always nullish and has no observable side effects.
    fn is_always_nullish_without_side_effects(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        matches!(
            Self::scalar_literal_expression(module, expression_id),
            Some(js::ScalarLiteral::Null | js::ScalarLiteral::Undefined)
        )
    }

    /// Return whether one expression can never evaluate to null or undefined.
    fn is_never_nullish(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        if matches!(
            Self::scalar_literal_expression(module, expression_id),
            Some(
                js::ScalarLiteral::Boolean(_)
                    | js::ScalarLiteral::Number(_)
                    | js::ScalarLiteral::Bigint(_)
                    | js::ScalarLiteral::String(_)
                    | js::ScalarLiteral::RegexString { .. }
            )
        ) {
            return true;
        }

        match module.tree.get(expression_id) {
            js::Expression::ArrayLiteral { .. }
            | js::Expression::ObjectLiteral { .. }
            | js::Expression::Declaration { .. }
            | js::Expression::ArrowFunction { .. }
            | js::Expression::TemplateLiteral { .. } => true,
            js::Expression::Unary {
                operator: js::UnaryOperator::Void,
                ..
            } => false,
            js::Expression::Unary { .. } => true,
            js::Expression::Parenthesized { expression } => {
                Self::is_never_nullish(module, *expression)
            }
            _ => false,
        }
    }
}
