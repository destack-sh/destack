use destack_core::StringId;
use destack_dir as dir;

use crate::ModuleQueryContext;

impl ModuleQueryContext<'_> {
    /// Resolve the syntactic name of one decorator node.
    pub(crate) fn decorator_name(&self, decorator: dir::GlobalNodeIdAny) -> Option<String> {
        assert_eq!(
            decorator.module_id,
            self.module_id(),
            "decorator node belongs to another module"
        );

        let decorator_id = decorator
            .local_id
            .try_into_typed::<dir::Decorator>()
            .unwrap_or_else(|_| panic!("decorator source is not a decorator: {decorator:?}"));
        let decorator = self.view().get(decorator_id);
        let name = self.decorator_leaf_name_id(decorator)?;

        Some(self.strings().get(name).to_string())
    }

    /// Resolve the last segment of a decorator name when it is path-like.
    pub(crate) fn decorator_leaf_name_id(&self, decorator: &dir::Decorator) -> Option<StringId> {
        let mut expression_id = decorator.expression;
        loop {
            match self.tree().get(expression_id) {
                dir::Expression::Call { left, .. } => {
                    expression_id = *left;
                }
                dir::Expression::Member { name, .. } => return *name,
                dir::Expression::Identifier { name } => return Some(*name),
                _ => return None,
            }
        }
    }
}
