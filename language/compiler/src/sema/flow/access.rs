use destack_dir as dir;

use crate::sema::{AssignedPlace, CheckState};

impl CheckState<'_> {
    /// Return the lexical access path for one expression.
    pub(in crate::sema) fn lexical_access_path(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::AccessPath> {
        // build the path by the expression's own syntax
        match self.module(self.module_id).view().get(id) {
            // value
            dir::Expression::Identifier { .. } => {
                let node = id.into_global_any(self.module_id);
                let reference = self.module(self.module_id).resolved.references.get(node)?;
                let symbol = match reference {
                    dir::Reference::Bound(symbols) => {
                        let symbols = self.present_symbols(symbols);
                        match symbols.as_slice() {
                            [symbol] => *symbol,
                            _ => return None,
                        }
                    }
                    dir::Reference::TypeLiteral(_)
                    | dir::Reference::Missing
                    | dir::Reference::Namespace { .. }
                    | dir::Reference::Projected { .. }
                    | dir::Reference::Ambiguous(_) => return None,
                };

                Some(dir::AccessPath::symbol(symbol))
            }
            // root `this` and `super` at the receiver instance
            dir::Expression::This | dir::Expression::Super => Some(dir::AccessPath::receiver()),
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
                let key = self
                    .module(self.module_id)
                    .view()
                    .get(*index)
                    .static_key()?;
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
    pub(in crate::sema) fn clear_mutated_expression_narrowings(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        // require a lexical path for the mutated expression
        let Some(path) = self.lexical_access_path(id) else {
            return;
        };

        // clear every dependent narrowing
        self.flow.clear_narrowings_under(&path);
    }
}

impl CheckState<'_> {
    /// Return the receiver type place assignments write through.
    pub(in crate::sema) fn assigned_receiver_type(&self) -> Option<dir::GlobalTypeId> {
        // take the lexical receiver inside the function that captured it
        let receiver = if let Some((true, receiver)) = self.flow.lexical_receiver() {
            receiver.receiver
        }
        // take the contextual receiver outside any function body
        else if self.flow.current_function().is_none() {
            self.flow.current_receiver()?
        }
        // leave writes without a receiver untracked
        else {
            return None;
        };

        Some(receiver.ty)
    }

    /// Return whether one expression names the current function's receiver.
    pub(in crate::sema) fn is_receiver_expression(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        matches!(
            self.module(self.module_id).view().get(id),
            dir::Expression::This | dir::Expression::Super
        )
    }

    /// Derive the assigned place one expression writes, when trackable.
    pub(in crate::sema) fn assigned_place(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<AssignedPlace> {
        // derive the place by the expression's own syntax
        let expression = self.module(self.module_id).view().get(id).clone();
        match expression {
            // x
            dir::Expression::Identifier { .. } => {
                let node = id.into_global_any(self.module_id);
                let reference = self.module(self.module_id).resolved.references.get(node)?;
                let dir::Reference::Bound(symbols) = reference else {
                    return None;
                };
                let symbols = self.present_symbols(symbols);
                match symbols.as_slice() {
                    [symbol] => Some(AssignedPlace::Symbol(*symbol)),
                    _ => None,
                }
            }
            // this.member, super.member
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                // keep writes that go through the receiver instance
                if !self.is_receiver_expression(left) {
                    return None;
                }

                // name the type the write lands on
                let receiver = self.assigned_receiver_type()?;

                Some(AssignedPlace::Member {
                    receiver,
                    key: dir::StaticKey::Name(name),
                })
            }
            // (place as T), (place satisfies T)
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => self.assigned_place(expression),

            // leave every other target untracked
            _ => None,
        }
    }
}
