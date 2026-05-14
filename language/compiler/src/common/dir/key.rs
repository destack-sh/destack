use destack_core::StringId;
use destack_dir as dir;

/// Resolve a static key from one DIR key when it is locally obvious.
pub(crate) fn static_key_from_key(
    tree: &dir::Tree,
    key: dir::Key,
    mut private_name: impl FnMut(StringId) -> StringId,
) -> Option<dir::StaticKey> {
    match key {
        dir::Key::Name(dir::Name::Identifier(name) | dir::Name::String(name)) => {
            Some(dir::StaticKey::Name(name))
        }
        dir::Key::Name(dir::Name::Number(name)) => Some(dir::StaticKey::Number(name)),
        dir::Key::Private(name) => Some(dir::StaticKey::Name(private_name(name))),
        dir::Key::Expression(expression_id) => static_key_from_expression(tree, expression_id),
    }
}

/// Resolve a static key from one locally constant expression.
fn static_key_from_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::StaticKey> {
    match tree.get(expression_id) {
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(name)) => {
            Some(dir::StaticKey::Name(*name))
        }
        _ => None,
    }
}
