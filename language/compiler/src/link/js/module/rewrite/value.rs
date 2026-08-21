use crate::emit::js;

use super::linker::Rewriter;

impl Rewriter<'_, '_> {
    /// Return whether two expression values are the same reference shape for minify purposes.
    pub(super) fn same_expression_value(
        module: &js::Module,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> bool {
        if left == right {
            return true;
        }

        if let (Some(left_symbol), Some(right_symbol)) =
            (module.tree.symbol(left), module.tree.symbol(right))
            && left_symbol == right_symbol
        {
            return true;
        }

        match (module.tree.get(left), module.tree.get(right)) {
            (
                js::Expression::Path { path: left_path },
                js::Expression::Path { path: right_path },
            ) => left_path == right_path,
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
        module: &mut js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> js::LocalNodeId<js::Expression> {
        module.tree.insert_from(
            js::Expression::Literal {
                value: js::Literal::Null,
            },
            expression_id,
        )
    }

    /// Return whether one expression is a bare null value.
    pub(super) fn is_null_expression(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        matches!(
            module.tree.get(expression_id),
            js::Expression::Literal {
                value: js::Literal::Null
            }
        )
    }

    /// Return whether one expression is a bare undefined value.
    pub(super) fn is_undefined_expression(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        matches!(
            Self::scalar_literal_expression(module, expression_id),
            Some(js::Literal::Undefined)
        )
    }

    /// Return whether one expression is one bare `typeof` operation.
    pub(super) fn is_typeof_expression(
        module: &js::Module,
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
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        matches!(
            module.tree.get(expression_id),
            js::Expression::Literal {
                value: js::Literal::String(_),
            }
        )
    }

    /// Return whether one expression is the string literal `"undefined"`.
    pub(super) fn is_undefined_string_expression(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        let js::Expression::Literal {
            value: js::Literal::String(value),
        } = module.tree.get(expression_id)
        else {
            return false;
        };

        module.strings.get(*value) == "undefined"
    }

    /// Return whether one expression is a primitive literal for comparison ordering.
    pub(super) fn is_primitive_literal_expression(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        Self::scalar_literal_expression(module, expression_id).is_some()
    }

    /// Return the truthiness of one side effect free literal expression.
    pub(super) fn literal_truthiness(
        &self,
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<bool> {
        Self::literal_truthiness_static(module, expression_id)
    }

    /// Return the truthiness of one side effect free literal expression.
    pub(super) fn literal_truthiness_static(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<bool> {
        match Self::scalar_literal_expression(module, expression_id)? {
            js::Literal::Null | js::Literal::Undefined => Some(false),
            js::Literal::Boolean(value) => Some(value),
            js::Literal::Number(value) => Some(!value.is_nan() && value != 0.0),
            js::Literal::Bigint(value) => Some(value != 0),
            js::Literal::String(value) => Some(!module.strings.get(value).is_empty()),
            js::Literal::RegexString { .. } => Some(true),
        }
    }

    /// Return one scalar literal expression recursively through parentheses.
    pub(super) fn scalar_literal_expression(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Literal> {
        match module.tree.get(expression_id) {
            js::Expression::Parenthesized { expression } => {
                Self::scalar_literal_expression(module, *expression)
            }
            js::Expression::Unary { operator, right } => {
                let scalar = Self::scalar_literal_expression(module, *right)?;

                match (operator, scalar) {
                    (js::UnaryOperator::Plus, js::Literal::Number(value)) => {
                        Some(js::Literal::Number(value))
                    }
                    (js::UnaryOperator::Negate, js::Literal::Number(value)) => {
                        Some(js::Literal::Number(-value))
                    }
                    (js::UnaryOperator::Void, _) => {
                        if Self::is_removable_sequence_prefix(module, *right) {
                            Some(js::Literal::Undefined)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            js::Expression::Path { path } => {
                Self::global_scalar_literal_expression(module, expression_id, path)
            }
            js::Expression::Literal { value } => Some(value.clone()),
            _ => None,
        }
    }

    /// Return one output-side scalar literal expression, including builtin scalar globals.
    pub(super) fn output_scalar_literal_expression(
        &self,
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Literal> {
        match module.tree.get(expression_id) {
            js::Expression::Parenthesized { expression } => {
                self.output_scalar_literal_expression(module, *expression)
            }
            js::Expression::Unary { operator, right } => {
                let scalar = self.output_scalar_literal_expression(module, *right)?;

                match (operator, scalar) {
                    (js::UnaryOperator::Plus, js::Literal::Number(value)) => {
                        Some(js::Literal::Number(value))
                    }
                    (js::UnaryOperator::Negate, js::Literal::Number(value)) => {
                        Some(js::Literal::Number(-value))
                    }
                    (js::UnaryOperator::Void, _) => {
                        if Self::is_removable_sequence_prefix(module, *right) {
                            Some(js::Literal::Undefined)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            js::Expression::Path { path } => {
                self.output_global_scalar_literal_expression(module, expression_id, path)
            }
            js::Expression::Literal { value } => Some(value.clone()),
            _ => None,
        }
    }

    /// Return one literal comparison value, including folded `typeof` strings.
    pub(super) fn scalar_comparison_literal_expression(
        module: &mut js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Literal> {
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

        Some(js::Literal::String(module.strings.intern(kind)))
    }

    /// Return one bare global primitive literal when one is known.
    pub(super) fn global_scalar_literal_expression(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
        path: &js::Path,
    ) -> Option<js::Literal> {
        if module.tree.symbol(expression_id).is_some() || path.segments.len() != 1 {
            return None;
        }

        match module.strings.get(path.segments[0]) {
            "undefined" => Some(js::Literal::Undefined),
            "NaN" => Some(js::Literal::Number(f64::NAN)),
            "Infinity" => Some(js::Literal::Number(f64::INFINITY)),
            _ => None,
        }
    }

    /// Return one builtin scalar global when output may treat it as a literal.
    pub(super) fn output_global_scalar_literal_expression(
        &self,
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
        path: &js::Path,
    ) -> Option<js::Literal> {
        if let Some(value) = Self::global_scalar_literal_expression(module, expression_id, path) {
            return Some(value);
        }

        if path.segments.len() != 1 {
            return None;
        }

        let js::ScriptSymbolId::Source(_) = module.tree.symbol(expression_id)? else {
            return None;
        };

        match module.strings.get(path.segments[0]) {
            "undefined" => Some(js::Literal::Undefined),
            "NaN" => Some(js::Literal::Number(f64::NAN)),
            "Infinity" => Some(js::Literal::Number(f64::INFINITY)),
            _ => None,
        }
    }

    /// Return one boolean literal value recursively through parentheses.
    pub(super) fn boolean_literal_value(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> Option<bool> {
        let js::Literal::Boolean(value) =
            Self::scalar_literal_expression(module, expression_id)?
        else {
            return None;
        };

        Some(value)
    }

    /// Return whether one expression may be removed from a comma prefix.
    pub(super) fn is_removable_sequence_prefix(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
    ) -> bool {
        match module.tree.get(expression_id) {
            js::Expression::Parenthesized { expression } => {
                Self::is_removable_sequence_prefix(module, *expression)
            }
            js::Expression::Literal { .. } => true,
            _ => false,
        }
    }

    /// Evaluate one primitive literal comparison.
    pub(super) fn evaluate_literal_comparison(
        module: &js::Module,
        left: &js::Literal,
        operator: js::BinaryOperator,
        right: &js::Literal,
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
        module: &js::Module,
        left: &js::Literal,
        right: &js::Literal,
    ) -> bool {
        match (left, right) {
            (js::Literal::Null, js::Literal::Null) => true,
            (js::Literal::Undefined, js::Literal::Undefined) => true,
            (js::Literal::Boolean(left), js::Literal::Boolean(right)) => left == right,
            (js::Literal::Number(left), js::Literal::Number(right)) => left == right,
            (js::Literal::Bigint(left), js::Literal::Bigint(right)) => left == right,
            (js::Literal::String(left), js::Literal::String(right)) => {
                module.strings.get(*left) == module.strings.get(*right)
            }
            _ => false,
        }
    }

    /// Return whether two primitive literals are loosely equal when that is simple to prove.
    pub(super) fn loose_literal_equality(
        module: &js::Module,
        left: &js::Literal,
        right: &js::Literal,
    ) -> Option<bool> {
        match (left, right) {
            (js::Literal::Null, js::Literal::Undefined)
            | (js::Literal::Undefined, js::Literal::Null) => Some(true),
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
        module: &js::Module,
        left: &js::Literal,
        right: &js::Literal,
        relation: u8,
    ) -> Option<bool> {
        let value = match (left, right) {
            (js::Literal::Number(left), js::Literal::Number(right)) => match relation {
                0 => left < right,
                1 => left <= right,
                2 => left > right,
                3 => left >= right,
                _ => unreachable!("unexpected relation"),
            },
            (js::Literal::Bigint(left), js::Literal::Bigint(right)) => match relation {
                0 => left < right,
                1 => left <= right,
                2 => left > right,
                3 => left >= right,
                _ => unreachable!("unexpected relation"),
            },
            (js::Literal::String(left), js::Literal::String(right)) => {
                let left = module.strings.get(*left);
                let right = module.strings.get(*right);

                match relation {
                    0 => left < right,
                    1 => left <= right,
                    2 => left > right,
                    3 => left >= right,
                    _ => unreachable!("unexpected relation"),
                }
            }
            (js::Literal::Boolean(left), js::Literal::Boolean(right)) => match relation
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
        module: &js::Module,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<&'static str> {
        let kind = match Self::scalar_literal_expression(module, right)? {
            js::Literal::Null => "object",
            js::Literal::Undefined => "undefined",
            js::Literal::Boolean(_) => "boolean",
            js::Literal::Number(_) => "number",
            js::Literal::Bigint(_) => "bigint",
            js::Literal::String(_) | js::Literal::RegexString { .. } => "string",
        };

        Some(kind)
    }
}
