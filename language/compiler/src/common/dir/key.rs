use destack_core::StringId;
use destack_dir::{Expression, Key, Name, NodeTree, ScalarLiteral, StaticKey};

/// Resolve a static key from one DIR key when it is locally obvious.
pub(crate) fn static_key_from_key(
    tree: &NodeTree,
    key: Key,
    mut private_name: impl FnMut(StringId) -> StringId,
) -> Option<StaticKey> {
    match key {
        Key::Name(Name::Identifier(name) | Name::String(name)) => Some(StaticKey::Name(name)),
        Key::Name(Name::Number(name)) => Some(StaticKey::Number(name)),
        Key::Private(name) => Some(StaticKey::Name(private_name(name))),
        Key::Expression(expression_id) => static_key_from_expression(tree, expression_id),
    }
}

/// Resolve a static key from one locally constant expression.
fn static_key_from_expression(
    tree: &NodeTree,
    expression_id: destack_dir::LocalNodeId<Expression>,
) -> Option<StaticKey> {
    match tree.get(expression_id) {
        Expression::ScalarLiteral {
            value: ScalarLiteral::String(name),
        } => Some(StaticKey::Name(*name)),
        _ => None,
    }
}
