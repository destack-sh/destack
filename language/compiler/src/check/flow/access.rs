use destack_dir as dir;

use crate::check::WalkState;

impl WalkState<'_, '_> {
    /// Return the lexical access path for one expression.
    pub(in crate::check) fn lexical_access_path(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::AccessPath> {
        match self.tree.get(id) {
            // value
            dir::Expression::Identifier { .. } => {
                let node = id.into_global_any(self.module);
                let reference = self
                    .check
                    .module(self.module)
                    .resolved
                    .references
                    .get(node)?;
                let symbol = match reference {
                    dir::Reference::Bound(symbols) => {
                        let symbols = self.check.present_symbols(symbols);
                        match symbols.as_slice() {
                            [symbol] => *symbol,
                            _ => return None,
                        }
                    }
                    dir::Reference::Missing
                    | dir::Reference::Namespace(_)
                    | dir::Reference::Projected { .. }
                    | dir::Reference::Ambiguous(_) => return None,
                };

                Some(dir::AccessPath::symbol(symbol))
            }
            // receiver
            dir::Expression::This => Some(dir::AccessPath::receiver(dir::ReceiverKind::This)),
            // value.member
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                // extend root path with the member key
                let mut path = self.lexical_access_path(*left)?;
                path.push(dir::StaticKey::Name(*name));

                Some(path)
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                // extend root path with a static key
                let key = self.tree.get(*index).static_key()?;
                let mut path = self.lexical_access_path(*left)?;

                path.push(key);

                Some(path)
            }
            // value?.member
            dir::Expression::Chain { expression } => self.lexical_access_path(*expression),
            // no lexical access path
            _ => None,
        }
    }

    /// Clear flow narrowings invalidated by mutating an expression.
    pub(in crate::check) fn clear_mutated_expression_narrowings(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        let Some(path) = self.lexical_access_path(id) else {
            return;
        };

        // clear every dependent narrowing
        self.flow_mut().clear_narrowings_under(&path);
    }
}
