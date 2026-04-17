use destack_dir::{Declaration, LocalNodeIdAny, Member, NodeTree, NodeType, SymbolTable};

use crate::Compiler;

impl Compiler {
    /// Check whether a node is active for the current profile.
    pub(crate) fn is_node_active(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        node_id: LocalNodeIdAny,
    ) -> bool {
        // skip nodes explicitly marked inactive
        if tree.is_inactive(node_id.id) {
            return false;
        }

        // honor inactive symbols for declaration like nodes
        match node_id.ty {
            NodeType::Declaration => {
                // resolve declaration symbols
                let declaration_id = node_id.into_typed::<Declaration>();
                let declaration = tree.get(declaration_id);
                let symbol = symbols.get_symbol(declaration.symbol());
                if !symbol.is_active {
                    return false;
                }
            }
            NodeType::Member => {
                // resolve member symbols
                let member_id = node_id.into_typed::<Member>();
                let member = tree.get(member_id);
                let symbol = symbols.get_symbol(member.symbol());
                if !symbol.is_active {
                    return false;
                }
            }
            _ => {}
        }

        true
    }
}
