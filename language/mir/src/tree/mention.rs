use destack_core::FxIndexSet;

use crate::{FunctionId, NodeVisitor, Tree, Type, TypeId, walk_type};

/// Return the types some functions mention, through anonymous types up to declared ones.
pub fn mentioned_types(tree: &Tree, functions: &[FunctionId]) -> FxIndexSet<TypeId> {
    // walk each function once, recording the types it reaches
    let mut mentions = TypeMentions {
        types: FxIndexSet::default(),
    };
    for id in functions {
        mentions.visit_function(tree, *id, tree.get(*id));
    }

    mentions.types
}

/// Collect every type a walk reaches once.
struct TypeMentions {
    /// The types reached so far, in walk order.
    types: FxIndexSet<TypeId>,
}

impl NodeVisitor for TypeMentions {
    fn visit_type(&mut self, tree: &Tree, id: TypeId, ty: &Type) {
        if self.types.insert(id) && tree.type_declaration(id).is_none() {
            walk_type(self, tree, ty);
        }
    }
}
