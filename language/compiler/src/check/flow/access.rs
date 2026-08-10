use destack_dir as dir;

use crate::check::{AssignedPlace, CheckState};

impl CheckState<'_> {
    /// Return the lexical access path for one expression.
    pub(in crate::check) fn lexical_access_path(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::AccessPath> {
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
                    dir::Reference::Missing
                    | dir::Reference::Namespace { .. }
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
    pub(in crate::check) fn clear_mutated_expression_narrowings(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        let Some(path) = self.lexical_access_path(id) else {
            return;
        };

        // clear every dependent narrowing
        self.flow.clear_narrowings_under(&path);
    }
}

impl CheckState<'_> {
    /// Return the receiver type place assignments write through.
    fn assigned_receiver_type(&self) -> Option<dir::GlobalTypeId> {
        if let Some((index, receiver)) = self.flow.lexical_receiver()
            && self.flow.is_current_function(index)
        {
            return Some(receiver.receiver.ty);
        }

        if self.flow.current_function().is_none() {
            return self.flow.current_receiver().map(|receiver| receiver.ty);
        }

        None
    }

    /// Derive the assigned place one expression writes, when trackable.
    pub(in crate::check) fn assigned_place(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<AssignedPlace> {
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
            // this.member
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                let receiver = self.assigned_receiver_type()?;
                let left = self.module(self.module_id).view().get(left).clone();
                matches!(left, dir::Expression::This).then(|| AssignedPlace::Member {
                    receiver,
                    key: dir::StaticKey::Name(name),
                })
            }
            // (place as T), (place satisfies T)
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => self.assigned_place(expression),

            _ => None,
        }
    }
}
