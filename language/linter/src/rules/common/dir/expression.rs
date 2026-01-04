use destack_base::{StringId, StringPool};
use destack_dir as dir;

use crate::{ConstValue, LintModuleDirContext};

/// The base of a reference path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceBase {
    /// A symbol-backed reference.
    Symbol(dir::GlobalSymbolId),
    /// A `this` reference.
    This,
}

/// A reference path from a base symbol to member names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePath {
    /// The base of the path.
    pub base: ReferenceBase,
    /// The member names from the base expression.
    pub members: Vec<StringId>,
}

/// Resolve the target symbol for a reference expression.
pub fn expression_target_symbol(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    // unwrap parenthesized expressions first
    let expression_id = unwrap_parenthesized_expression(tree, expression_id);

    // return the reference target symbol when present
    let expression = tree.get(expression_id);
    expression.target_symbol()
}

/// Resolve a reference path for member expressions.
pub fn expression_reference_path(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ReferencePath> {
    // collect member names walking left
    let mut members = Vec::new();
    let base = expression_reference_path_base(tree, expression_id, &mut members)?;

    // normalize member order
    members.reverse();

    Some(ReferencePath { base, members })
}

/// Return true when the expression is a global qualified member access.
pub fn expression_is_global_qualified_member(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    qualifiers: &[dir::GlobalSymbolId],
    member_name: StringId,
) -> bool {
    // resolve the member path
    let Some(path) = expression_reference_path(tree, expression_id) else {
        return false;
    };

    // ensure the requested member is present
    if path.members.as_slice() != [member_name] {
        return false;
    }

    // ensure the base is a known global qualifier
    match path.base {
        ReferenceBase::Symbol(symbol) => qualifiers.contains(&symbol),
        ReferenceBase::This => false,
    }
}

/// Collect the global qualifier symbols for the active profile.
pub fn global_qualifier_symbols(ctx: &LintModuleDirContext<'_>) -> Vec<dir::GlobalSymbolId> {
    // collect canonical global qualifiers
    let mut qualifiers = Vec::new();

    // resolve configured globals
    for name in ["globalThis", "window", "self", "global"] {
        let name_id = ctx.program.strings.intern(name);
        if let Some(symbol) = ctx.get_lib_item(name_id) {
            qualifiers.push(symbol);
        }
    }

    qualifiers
}

/// Return the expression id with parenthesized nodes unwrapped.
pub fn unwrap_parenthesized_expression(
    tree: &dir::NodeTree,
    mut expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    // walk through parenthesized expressions
    loop {
        let dir::Expression::Parenthesized { expression } = tree.get(expression_id) else {
            return expression_id;
        };
        expression_id = *expression;
    }
}

/// Convert a constant value into an i64 when possible.
pub fn const_i64(value: &ConstValue) -> Option<i64> {
    // evaluate constant variants
    match value {
        ConstValue::Integer(value) => Some(*value),
        ConstValue::Bigint(value) => Some(*value),
        ConstValue::Float(value) => {
            if value.is_finite() && value.fract() == 0.0 {
                Some(*value as i64)
            } else {
                None
            }
        }
        ConstValue::Boolean(value) => Some(i64::from(*value)),
        ConstValue::Null | ConstValue::Undefined => None,
    }
}

/// Return the utf16 length of a string literal.
pub fn string_literal_utf16_length(strings: &StringPool, value: StringId) -> usize {
    // count utf16 code units
    let text = strings.get(value);
    text.as_ref().encode_utf16().count()
}

/// Flip a comparison operator when the operands are swapped.
pub fn flip_binary_operator(operator: dir::BinaryOperator) -> Option<dir::BinaryOperator> {
    match operator {
        dir::BinaryOperator::GreaterThan => Some(dir::BinaryOperator::LessThan),
        dir::BinaryOperator::GreaterThanOrEqual => Some(dir::BinaryOperator::LessThanOrEqual),
        dir::BinaryOperator::LessThan => Some(dir::BinaryOperator::GreaterThan),
        dir::BinaryOperator::LessThanOrEqual => Some(dir::BinaryOperator::GreaterThanOrEqual),
        dir::BinaryOperator::Equal
        | dir::BinaryOperator::EqualStrict
        | dir::BinaryOperator::NotEqual
        | dir::BinaryOperator::NotEqualStrict => Some(operator),
        _ => None,
    }
}

/// Get the base of a reference path.
fn expression_reference_path_base(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    members: &mut Vec<StringId>,
) -> Option<ReferenceBase> {
    // inspect the expression node
    let expression = tree.get(expression_id);

    // match the base or member steps
    match expression {
        dir::Expression::Parenthesized { expression } => {
            expression_reference_path_base(tree, *expression, members)
        }
        dir::Expression::Member { left, name, .. } => {
            members.push(*name);
            expression_reference_path_base(tree, *left, members)
        }
        dir::Expression::This => Some(ReferenceBase::This),
        _ => expression.target_symbol().map(ReferenceBase::Symbol),
    }
}
