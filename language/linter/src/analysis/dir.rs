use std::collections::HashMap;

use destack_dir as dir;

use crate::{ConstValue, LintRegexParse};

/// Cache shared analysis results for DIR lint rules.
#[derive(Debug, Default)]
pub struct LintDirAnalysisCache {
    /// Cached constant values for DIR expressions.
    const_values: HashMap<u32, Option<ConstValue>>,
    /// Cached regex parse results by pattern and optional flags string ids.
    regex_parse: HashMap<(dir::StringId, Option<dir::StringId>), LintRegexParse>,
}

impl LintDirAnalysisCache {
    /// Return a cached constant value for an expression.
    pub fn const_value(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<ConstValue> {
        if let Some(value) = self.const_values.get(&id.id) {
            return *value;
        }

        let value = evaluate_const_value(tree, id);
        self.const_values.insert(id.id, value);
        value
    }

    /// Return cached regex parse info for a pattern string.
    pub fn regex_parse(&mut self, strings: &dir::StringPool, id: dir::StringId) -> LintRegexParse {
        self.regex_parse_with_flags(strings, id, None)
    }

    /// Return cached regex parse info for a pattern string and optional flags.
    pub fn regex_parse_with_flags(
        &mut self,
        strings: &dir::StringPool,
        pattern_id: dir::StringId,
        flags_id: Option<dir::StringId>,
    ) -> LintRegexParse {
        let cache_key = (pattern_id, flags_id);
        if let Some(parse) = self.regex_parse.get(&cache_key) {
            return parse.clone();
        }

        let pattern = strings.get(pattern_id);
        let flags = flags_id.map(|id| strings.get(id));
        let parse = LintRegexParse::parse_with_flags(pattern, flags);
        self.regex_parse.insert(cache_key, parse.clone());
        parse
    }
}

/// Evaluate a DIR expression to a constant value when possible.
fn evaluate_const_value(
    tree: &dir::Tree,
    id: dir::LocalNodeId<dir::Expression>,
) -> Option<ConstValue> {
    let expression = tree.get(id);
    match expression {
        dir::Expression::ScalarLiteral(value) => match value {
            dir::ScalarLiteral::Null => Some(ConstValue::Null),
            dir::ScalarLiteral::Boolean(value) => Some(ConstValue::Boolean(*value)),
            dir::ScalarLiteral::Integer(value) => Some(ConstValue::Integer(*value)),
            dir::ScalarLiteral::Bigint(value) => Some(ConstValue::Bigint(*value)),
            dir::ScalarLiteral::Float(value) => Some(ConstValue::Float(*value)),
            _ => None,
        },
        dir::Expression::Parenthesized { expression } => evaluate_const_value(tree, *expression),
        dir::Expression::Unary { operator, right } => {
            let value = evaluate_const_value(tree, *right)?;
            match operator {
                dir::UnaryOperator::Not => Some(ConstValue::Boolean(!value.to_bool())),
                dir::UnaryOperator::Plus => Some(value),
                dir::UnaryOperator::Negate => negate_const_value(value),
                dir::UnaryOperator::ElementwiseNot => bit_not_const_value(value),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Apply unary negation to a constant value.
fn negate_const_value(value: ConstValue) -> Option<ConstValue> {
    match value {
        ConstValue::Integer(value) => Some(ConstValue::Integer(-value)),
        ConstValue::Bigint(value) => Some(ConstValue::Bigint(-value)),
        ConstValue::Float(value) => Some(ConstValue::Float(-value)),
        ConstValue::Boolean(value) => Some(ConstValue::Integer(-(value as i64))),
        ConstValue::Null => Some(ConstValue::Integer(0)),
        ConstValue::Undefined => None,
    }
}

/// Apply bitwise not to a constant value.
fn bit_not_const_value(value: ConstValue) -> Option<ConstValue> {
    match value {
        ConstValue::Integer(value) => Some(ConstValue::Integer(!value)),
        ConstValue::Bigint(value) => Some(ConstValue::Bigint(!value)),
        ConstValue::Boolean(value) => Some(ConstValue::Integer(!(value as i64))),
        ConstValue::Null => Some(ConstValue::Integer(!0)),
        _ => None,
    }
}
