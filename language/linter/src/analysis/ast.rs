use std::collections::HashMap;

use destack_ast as ast;

use crate::{ConstValue, LintRegexParse};

/// Cache shared analysis results for AST lint rules.
#[derive(Debug, Default)]
pub struct LintAstAnalysisCache {
    /// Cached constant values for AST expressions.
    const_values: HashMap<u32, Option<ConstValue>>,
    /// Cached regex parse results by pattern and optional flags string ids.
    regex_parse: HashMap<(ast::StringId, Option<ast::StringId>), LintRegexParse>,
}

impl LintAstAnalysisCache {
    /// Return a cached constant value for an expression.
    pub fn const_value(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Expression>,
    ) -> Option<ConstValue> {
        if let Some(value) = self.const_values.get(&id.id) {
            return *value;
        }

        let value = evaluate_const_value(tree, id);
        self.const_values.insert(id.id, value);
        value
    }

    /// Return cached regex parse info for a pattern string.
    pub fn regex_parse(&mut self, strings: &ast::StringPool, id: ast::StringId) -> LintRegexParse {
        self.regex_parse_with_flags(strings, id, None)
    }

    /// Return cached regex parse info for a pattern string and optional flags.
    pub fn regex_parse_with_flags(
        &mut self,
        strings: &ast::StringPool,
        pattern_id: ast::StringId,
        flags_id: Option<ast::StringId>,
    ) -> LintRegexParse {
        let cache_key = (pattern_id, flags_id);
        if let Some(parse) = self.regex_parse.get(&cache_key) {
            return parse.clone();
        }

        let pattern = strings.get(pattern_id);
        let flags = flags_id.map(|id| strings.get(id));
        let flags = flags.as_deref();
        let parse = LintRegexParse::parse_with_flags(pattern.as_ref(), flags);
        self.regex_parse.insert(cache_key, parse.clone());
        parse
    }
}

/// Evaluate an AST expression to a constant value when possible.
fn evaluate_const_value(
    tree: &ast::NodeTree,
    id: ast::LocalNodeId<ast::Expression>,
) -> Option<ConstValue> {
    let expression = tree.get(id);
    match expression {
        ast::Expression::ScalarLiteral(literal) => match literal {
            ast::ScalarLiteral::Null => Some(ConstValue::Null),
            ast::ScalarLiteral::Boolean(value) => Some(ConstValue::Boolean(*value)),
            ast::ScalarLiteral::Integer(value) => Some(ConstValue::Integer(*value)),
            ast::ScalarLiteral::Bigint(value) => Some(ConstValue::Bigint(*value)),
            ast::ScalarLiteral::Float(value) => Some(ConstValue::Float(*value)),
            _ => None,
        },
        ast::Expression::Parenthesized { expression } => evaluate_const_value(tree, *expression),
        ast::Expression::Unary { operator, right } => {
            let value = evaluate_const_value(tree, *right)?;
            match operator {
                ast::UnaryOperator::Not => Some(ConstValue::Boolean(!value.to_bool())),
                ast::UnaryOperator::Plus => Some(value),
                ast::UnaryOperator::Negate | ast::UnaryOperator::WrappingNegate => {
                    negate_const_value(value)
                }
                ast::UnaryOperator::ElementwiseNot => bit_not_const_value(value),
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
