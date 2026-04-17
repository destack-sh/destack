use destack_codegen_js as js;

use super::linker::Rewriter;

impl Rewriter<'_, '_> {
    /// Return whether two expression values are the same reference shape for minify purposes.
    pub(super) fn same_expression_value(
        module: &js::ScriptModule,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> bool {
        if left == right {
            return true;
        }

        if let (Some(left_symbol), Some(right_symbol)) =
            (module.tree.symbol(left), module.tree.symbol(right))
        {
            if left_symbol == right_symbol {
                return true;
            }
        }

        match (module.tree.get(left), module.tree.get(right)) {
            (
                js::Expression::Path {
                    path: left_path,
                    generic_arguments: left_arguments,
                },
                js::Expression::Path {
                    path: right_path,
                    generic_arguments: right_arguments,
                },
            ) => left_arguments.is_empty() && right_arguments.is_empty() && left_path == right_path,
            (
                js::Expression::Parenthesized {
                    expression: left_expression,
                },
                _,
            ) => Self::same_expression_value(module, *left_expression, right),
            (
                _,
                js::Expression::Parenthesized {
                    expression: right_expression,
                },
            ) => Self::same_expression_value(module, left, *right_expression),
            _ => false,
        }
    }

    /// Insert one null literal near one source expression.
    pub(super) fn insert_null_literal(
        module: &mut js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> js::LocalNodeId<js::Expression> {
        module.tree.insert_from(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::Null,
            },
            expression_id,
        )
    }

    /// Return whether one expression is a bare null value.
    pub(super) fn is_null_expression(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        matches!(
            module.tree.get(expression_id),
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::Null
            }
        )
    }

    /// Return whether one expression is a bare undefined value.
    pub(super) fn is_undefined_expression(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        matches!(
            Self::scalar_literal_expression(module, expression_id),
            Some(js::ScalarLiteral::Undefined)
        )
    }

    /// Return whether one expression is one bare `typeof` operation.
    pub(super) fn is_typeof_expression(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        matches!(
            module.tree.get(expression_id),
            js::Expression::Unary {
                operator: js::UnaryOperator::Typeof,
                ..
            }
        )
    }

    /// Return whether one expression is one string literal.
    pub(super) fn is_string_literal_expression(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        matches!(
            module.tree.get(expression_id),
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::String(_),
            }
        )
    }

    /// Return whether one expression is the string literal `"undefined"`.
    pub(super) fn is_undefined_string_expression(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        let js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(value),
        } = module.tree.get(expression_id)
        else {
            return false;
        };

        module.strings.get(*value) == "undefined"
    }

    /// Return whether one expression is a primitive literal for comparison ordering.
    pub(super) fn is_primitive_literal_expression(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        Self::scalar_literal_expression(module, expression_id).is_some()
    }

    /// Return the truthiness of one side effect free literal expression.
    pub(super) fn literal_truthiness(
        &self,
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<bool> {
        Self::literal_truthiness_static(module, expression_id)
    }

    /// Return the truthiness of one side effect free literal expression.
    pub(super) fn literal_truthiness_static(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<bool> {
        match Self::scalar_literal_expression(module, expression_id)? {
            js::ScalarLiteral::Null | js::ScalarLiteral::Undefined => Some(false),
            js::ScalarLiteral::Boolean(value) => Some(value),
            js::ScalarLiteral::Number(value) => Some(!value.is_nan() && value != 0.0),
            js::ScalarLiteral::Bigint(value) => Some(value != 0),
            js::ScalarLiteral::String(value) => Some(!module.strings.get(value).is_empty()),
            js::ScalarLiteral::RegexString { .. } => Some(true),
        }
    }

    /// Return one scalar literal expression recursively through parentheses.
    pub(super) fn scalar_literal_expression(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<js::ScalarLiteral> {
        match module.tree.get(expression_id) {
            js::Expression::Parenthesized { expression } => {
                Self::scalar_literal_expression(module, *expression)
            }
            js::Expression::Unary { operator, right } => {
                let scalar = Self::scalar_literal_expression(module, *right)?;

                match (operator, scalar) {
                    (js::UnaryOperator::Plus, js::ScalarLiteral::Number(value)) => {
                        Some(js::ScalarLiteral::Number(value))
                    }
                    (js::UnaryOperator::Negate, js::ScalarLiteral::Number(value)) => {
                        Some(js::ScalarLiteral::Number(-value))
                    }
                    (js::UnaryOperator::Void, _) => {
                        if Self::is_removable_sequence_prefix(module, *right) {
                            Some(js::ScalarLiteral::Undefined)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            js::Expression::Path {
                path,
                generic_arguments,
            } => {
                if !generic_arguments.is_empty() {
                    return None;
                }

                Self::global_scalar_literal_expression(module, expression_id, path)
            }
            js::Expression::ScalarLiteral { value } => Some(value.clone()),
            _ => None,
        }
    }

    /// Return one output-side scalar literal expression, including builtin scalar globals.
    pub(super) fn output_scalar_literal_expression(
        &self,
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<js::ScalarLiteral> {
        match module.tree.get(expression_id) {
            js::Expression::Parenthesized { expression } => {
                self.output_scalar_literal_expression(module, *expression)
            }
            js::Expression::Unary { operator, right } => {
                let scalar = self.output_scalar_literal_expression(module, *right)?;

                match (operator, scalar) {
                    (js::UnaryOperator::Plus, js::ScalarLiteral::Number(value)) => {
                        Some(js::ScalarLiteral::Number(value))
                    }
                    (js::UnaryOperator::Negate, js::ScalarLiteral::Number(value)) => {
                        Some(js::ScalarLiteral::Number(-value))
                    }
                    (js::UnaryOperator::Void, _) => {
                        if Self::is_removable_sequence_prefix(module, *right) {
                            Some(js::ScalarLiteral::Undefined)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            js::Expression::Path {
                path,
                generic_arguments,
            } => {
                if !generic_arguments.is_empty() {
                    return None;
                }

                self.output_global_scalar_literal_expression(module, expression_id, path)
            }
            js::Expression::ScalarLiteral { value } => Some(value.clone()),
            _ => None,
        }
    }

    /// Return one literal comparison value, including folded `typeof` strings.
    pub(super) fn scalar_comparison_literal_expression(
        module: &mut js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<js::ScalarLiteral> {
        if let Some(value) = Self::scalar_literal_expression(module, expression_id) {
            return Some(value);
        }

        let js::Expression::Unary {
            operator: js::UnaryOperator::Typeof,
            right,
        } = module.tree.get(expression_id).clone()
        else {
            return None;
        };

        let kind = Self::typeof_literal_kind(module, right)?;

        Some(js::ScalarLiteral::String(module.strings.intern(kind)))
    }

    /// Return one bare global primitive literal when one is known.
    pub(super) fn global_scalar_literal_expression(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
        path: &js::Path,
    ) -> Option<js::ScalarLiteral> {
        if module.tree.symbol(expression_id).is_some() || path.segments.len() != 1 {
            return None;
        }

        match module.strings.get(path.segments[0]).as_ref() {
            "undefined" => Some(js::ScalarLiteral::Undefined),
            "NaN" => Some(js::ScalarLiteral::Number(f64::NAN)),
            "Infinity" => Some(js::ScalarLiteral::Number(f64::INFINITY)),
            _ => None,
        }
    }

    /// Return one builtin scalar global when output may treat it as a literal.
    pub(super) fn output_global_scalar_literal_expression(
        &self,
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
        path: &js::Path,
    ) -> Option<js::ScalarLiteral> {
        if let Some(value) = Self::global_scalar_literal_expression(module, expression_id, path) {
            return Some(value);
        }

        if path.segments.len() != 1 {
            return None;
        }

        let js::ScriptSymbolId::Source(symbol_id) = module.tree.symbol(expression_id)? else {
            return None;
        };

        let context = self.context?;

        if !context.module(symbol_id.module_id).is_builtin() {
            return None;
        }

        match module.strings.get(path.segments[0]).as_ref() {
            "undefined" => Some(js::ScalarLiteral::Undefined),
            "NaN" => Some(js::ScalarLiteral::Number(f64::NAN)),
            "Infinity" => Some(js::ScalarLiteral::Number(f64::INFINITY)),
            _ => None,
        }
    }

    /// Return one boolean literal value recursively through parentheses.
    pub(super) fn boolean_literal_value(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<bool> {
        let js::ScalarLiteral::Boolean(value) =
            Self::scalar_literal_expression(module, expression_id)?
        else {
            return None;
        };

        Some(value)
    }

    /// Return whether one expression may be removed from a comma prefix.
    pub(super) fn is_removable_sequence_prefix(
        module: &js::ScriptModule,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        match module.tree.get(expression_id) {
            js::Expression::Parenthesized { expression } => {
                Self::is_removable_sequence_prefix(module, *expression)
            }
            js::Expression::ScalarLiteral { .. } => true,
            _ => false,
        }
    }

    /// Evaluate one primitive literal comparison.
    pub(super) fn evaluate_literal_comparison(
        module: &js::ScriptModule,
        left: &js::ScalarLiteral,
        operator: js::BinaryOperator,
        right: &js::ScalarLiteral,
    ) -> Option<bool> {
        match operator {
            js::BinaryOperator::EqualStrict => {
                Some(Self::strict_literal_equality(module, left, right))
            }
            js::BinaryOperator::NotEqualStrict => {
                Some(!Self::strict_literal_equality(module, left, right))
            }
            js::BinaryOperator::Equal => Self::loose_literal_equality(module, left, right),
            js::BinaryOperator::NotEqual => {
                Self::loose_literal_equality(module, left, right).map(|value| !value)
            }
            js::BinaryOperator::LessThan => {
                Self::literal_relational_comparison(module, left, right, 0)
            }
            js::BinaryOperator::LessThanOrEqual => {
                Self::literal_relational_comparison(module, left, right, 1)
            }
            js::BinaryOperator::GreaterThan => {
                Self::literal_relational_comparison(module, left, right, 2)
            }
            js::BinaryOperator::GreaterThanOrEqual => {
                Self::literal_relational_comparison(module, left, right, 3)
            }
            _ => None,
        }
    }

    /// Return whether two primitive literals are strictly equal.
    pub(super) fn strict_literal_equality(
        module: &js::ScriptModule,
        left: &js::ScalarLiteral,
        right: &js::ScalarLiteral,
    ) -> bool {
        match (left, right) {
            (js::ScalarLiteral::Null, js::ScalarLiteral::Null) => true,
            (js::ScalarLiteral::Undefined, js::ScalarLiteral::Undefined) => true,
            (js::ScalarLiteral::Boolean(left), js::ScalarLiteral::Boolean(right)) => left == right,
            (js::ScalarLiteral::Number(left), js::ScalarLiteral::Number(right)) => left == right,
            (js::ScalarLiteral::Bigint(left), js::ScalarLiteral::Bigint(right)) => left == right,
            (js::ScalarLiteral::String(left), js::ScalarLiteral::String(right)) => {
                module.strings.get(*left).as_ref() == module.strings.get(*right).as_ref()
            }
            _ => false,
        }
    }

    /// Return whether two primitive literals are loosely equal when that is simple to prove.
    pub(super) fn loose_literal_equality(
        module: &js::ScriptModule,
        left: &js::ScalarLiteral,
        right: &js::ScalarLiteral,
    ) -> Option<bool> {
        match (left, right) {
            (js::ScalarLiteral::Null, js::ScalarLiteral::Undefined)
            | (js::ScalarLiteral::Undefined, js::ScalarLiteral::Null) => Some(true),
            _ if std::mem::discriminant(left) == std::mem::discriminant(right) => {
                Some(Self::strict_literal_equality(module, left, right))
            }
            _ => None,
        }
    }

    /// Return one relational comparison result for primitive literals.
    ///
    /// relation:
    /// 0: <
    /// 1: <=
    /// 2: >
    /// 3: >=
    pub(super) fn literal_relational_comparison(
        module: &js::ScriptModule,
        left: &js::ScalarLiteral,
        right: &js::ScalarLiteral,
        relation: u8,
    ) -> Option<bool> {
        let value = match (left, right) {
            (js::ScalarLiteral::Number(left), js::ScalarLiteral::Number(right)) => match relation {
                0 => left < right,
                1 => left <= right,
                2 => left > right,
                3 => left >= right,
                _ => unreachable!("unexpected relation"),
            },
            (js::ScalarLiteral::Bigint(left), js::ScalarLiteral::Bigint(right)) => match relation {
                0 => left < right,
                1 => left <= right,
                2 => left > right,
                3 => left >= right,
                _ => unreachable!("unexpected relation"),
            },
            (js::ScalarLiteral::String(left), js::ScalarLiteral::String(right)) => {
                let left = module.strings.get(*left);
                let right = module.strings.get(*right);
                let left = left.as_ref();
                let right = right.as_ref();

                match relation {
                    0 => left < right,
                    1 => left <= right,
                    2 => left > right,
                    3 => left >= right,
                    _ => unreachable!("unexpected relation"),
                }
            }
            (js::ScalarLiteral::Boolean(left), js::ScalarLiteral::Boolean(right)) => match relation
            {
                0 => (*left as u8) < (*right as u8),
                1 => (*left as u8) <= (*right as u8),
                2 => (*left as u8) > (*right as u8),
                3 => (*left as u8) >= (*right as u8),
                _ => unreachable!("unexpected relation"),
            },
            _ => return None,
        };

        Some(value)
    }

    /// Return one static string kind for a folded `typeof` operand.
    pub(super) fn typeof_literal_kind(
        module: &js::ScriptModule,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<&'static str> {
        let kind = match Self::scalar_literal_expression(module, right)? {
            js::ScalarLiteral::Null => "object",
            js::ScalarLiteral::Undefined => "undefined",
            js::ScalarLiteral::Boolean(_) => "boolean",
            js::ScalarLiteral::Number(_) => "number",
            js::ScalarLiteral::Bigint(_) => "bigint",
            js::ScalarLiteral::String(_) | js::ScalarLiteral::RegexString { .. } => "string",
        };

        Some(kind)
    }
}
