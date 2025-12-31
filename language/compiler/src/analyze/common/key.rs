use destack_dir::{DynamicKey, Expression, NodeTree, StaticKey};
use destack_workspace::ProfileId;

use crate::Compiler;

impl Compiler {
    /// Resolve a static key from a dynamic key when possible.
    pub(crate) fn static_key_from_dynamic_key(
        &self,
        profile: ProfileId,
        key: DynamicKey,
        tree: &NodeTree,
    ) -> Option<StaticKey> {
        match key {
            DynamicKey::Name(name) => Some(StaticKey::Name(name)),
            DynamicKey::Number(name) => Some(StaticKey::Number(name)),
            DynamicKey::Expression(expression_id) => {
                self.static_key_from_expression(profile, expression_id, tree)
            }
            DynamicKey::NamedExpression { .. } => None,
        }
    }

    /// Resolve a static key from a key expression when possible.
    fn static_key_from_expression(
        &self,
        profile: ProfileId,
        expression_id: destack_dir::LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<StaticKey> {
        let Expression::Member { left, name, .. } = tree.get(expression_id) else {
            return None;
        };
        let base_symbol = tree.get(*left).target_symbol()?;
        let well_known = self.get_well_known_symbols(profile)?;
        let global_name = well_known.global_symbol_key_for_member(base_symbol, *name)?;
        Some(StaticKey::GlobalSymbol(global_name))
    }
}
