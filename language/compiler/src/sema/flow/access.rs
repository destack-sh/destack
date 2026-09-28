use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{AssignedPlace, CheckState, Origin};

impl CheckState<'_> {
    /// Return the lexical access path for one expression.
    pub(in crate::sema) fn lexical_access_path(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::AccessPath> {
        // build the path by matching the expression
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

    /// Clear the narrowings under one written place.
    pub(in crate::sema) fn record_expression_write(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        if let Some(path) = self.lexical_access_path(id) {
            self.flow.clear_narrowings_under(&path);
        }
    }

    /// Return whether other frames may write one place.
    pub(in crate::sema) fn is_aliased_place(
        &mut self,
        origin: Origin,
        id: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        // find the base of the projection
        let view = self.module(id.module_id).view();
        let base = match view.get(id.local_id) {
            dir::Expression::Member { left, .. } | dir::Expression::Index { left, .. } => *left,
            dir::Expression::Chain { expression } => {
                let expression = expression.into_global(id.module_id);

                return self.is_aliased_place(origin, expression);
            }
            dir::Expression::Identifier { .. } => return Ok(self.is_static_binding(id)),
            _ => return Ok(false),
        };
        let base = base.into_global(id.module_id);

        // treat an untyped base as static storage
        let Some(ty) = self.committed_node_type(base.into_any()) else {
            return Ok(true);
        };

        // check the base's storage and place
        Ok(self.is_aliased_storage(origin, ty)? || self.is_aliased_place(origin, base)?)
    }

    /// Return whether one name reads a module or foreign binding.
    fn is_static_binding(&self, id: dir::GlobalNodeId<dir::Expression>) -> bool {
        // treat a foreign name as static
        if id.module_id != self.module_id {
            return true;
        }
        let root = self
            .lexical_access_path(id.local_id)
            .map(|path| path.root());
        let Some(dir::AccessRoot::Symbol(symbol)) = root else {
            return false;
        };
        if symbol.module_id != self.module_id {
            return true;
        }

        self.module(symbol.module_id)
            .binding_table()
            .function_scope(symbol.local_id)
            .is_none()
    }

    /// Return whether other frames may write one value's storage.
    fn is_aliased_storage(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        match self.borrow_access(origin, ty)? {
            // share a borrow's storage
            Some(access) => Ok(!access.excludes()),
            // follow the form chain for other values
            None => self.type_is_aliased(origin, ty),
        }
    }
}

impl CheckState<'_> {
    /// Return the receiver type place assignments write through.
    pub(in crate::sema) fn assigned_receiver_type(
        &self,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // take the lexical receiver inside the function that captured it
        let receiver = if let Some((true, receiver)) = self.flow.lexical_receiver() {
            receiver.receiver
        }
        // take the contextual receiver outside any function body
        else if self.flow.current_function().is_none()
            && let Some(receiver) = self.flow.current_receiver()
        {
            receiver
        }
        // leave writes without a receiver untracked
        else {
            return Ok(None);
        };

        // name the object beneath the receiver's memory forms
        let mut ty = receiver.ty;
        while let dir::Type::Form(form) = self.ty(ty)? {
            ty = form.value;
        }

        Ok(Some(ty))
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
    ) -> CompilerResult<Option<AssignedPlace>> {
        // derive the place by matching the expression
        let expression = self.module(self.module_id).view().get(id).clone();
        let place = match expression {
            // x
            dir::Expression::Identifier { .. } => {
                let node = id.into_global_any(self.module_id);
                let Some(dir::Reference::Bound(symbols)) =
                    self.module(self.module_id).resolved.references.get(node)
                else {
                    return Ok(None);
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
                    return Ok(None);
                }

                // name the type the write lands on
                self.assigned_receiver_type()?
                    .map(|receiver| AssignedPlace::Member {
                        receiver,
                        key: dir::StaticKey::Name(name),
                    })
            }
            // (place as T), (place satisfies T)
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => self.assigned_place(expression)?,

            // leave every other target untracked
            _ => None,
        };

        Ok(place)
    }
}
