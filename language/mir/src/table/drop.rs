use destack_core::{FxIndexMap, FxIndexSet};

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{Function, LocalNodeId, Reference, Storage, Substitution, Tree, Type};

/// Drop table for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DropTable {
    /// Generated destructors keyed by type and storage.
    destructors: FxIndexMap<(LocalNodeId<Type>, Storage), LocalNodeId<Function>>,
    /// User-authored drop hooks keyed by type.
    hooks: FxIndexMap<LocalNodeId<Type>, LocalNodeId<Function>>,
}

impl DropTable {
    /// Create an empty drop table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy drop table entries from one type id to another.
    pub fn copy_type_entries(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        let destructors = self
            .destructors
            .iter()
            .filter_map(|(&(ty, storage), &function)| (ty == from).then_some((storage, function)))
            .collect::<Vec<_>>();
        for (storage, function) in destructors {
            self.set_destructor(to, storage, function);
        }

        if let Some(function) = self.hook(from) {
            self.set_hook(to, function);
        }
    }

    /// Return the generated destructor for a type in one storage.
    pub fn destructor(
        &self,
        ty: LocalNodeId<Type>,
        storage: Storage,
    ) -> Option<LocalNodeId<Function>> {
        self.destructors.get(&(ty, storage)).copied()
    }

    /// Return whether any generated destructor exists for a type.
    pub fn has_destructor(&self, ty: LocalNodeId<Type>) -> bool {
        self.destructors
            .keys()
            .any(|(candidate, _)| *candidate == ty)
    }

    /// Iterate generated destructors.
    pub fn destructors(
        &self,
    ) -> impl Iterator<Item = (LocalNodeId<Type>, Storage, LocalNodeId<Function>)> + '_ {
        self.destructors
            .iter()
            .map(|(&(ty, storage), &function)| (ty, storage, function))
    }

    /// Return whether one function is generated drop glue.
    pub fn is_destructor(&self, function: LocalNodeId<Function>) -> bool {
        self.destructors
            .values()
            .any(|destructor| *destructor == function)
    }

    /// Return whether one stored value requires a generated destructor.
    pub fn requires_destructor(
        &self,
        ty: LocalNodeId<Type>,
        storage: Storage,
        tree: &mut Tree,
    ) -> bool {
        if self.destructor(ty, storage).is_some() || self.hook(ty).is_some() {
            return true;
        }

        self.children_require_destructor(ty, storage, tree, &mut FxIndexSet::default())
    }

    /// Record the generated destructor for a type in one storage.
    pub fn set_destructor(
        &mut self,
        ty: LocalNodeId<Type>,
        storage: Storage,
        destructor: LocalNodeId<Function>,
    ) -> Option<LocalNodeId<Function>> {
        self.destructors.insert((ty, storage), destructor)
    }

    /// Remove entries that reference one function.
    pub fn remove_function(&mut self, function: LocalNodeId<Function>) {
        self.destructors
            .retain(|_, destructor| *destructor != function);
        self.hooks.retain(|_, hook| *hook != function);
    }

    /// Return the user-authored drop hook for a type in one storage.
    pub fn hook(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Function>> {
        self.hooks.get(&ty).copied()
    }

    /// Return whether any user-authored drop hook exists for a type.
    pub fn has_hook(&self, ty: LocalNodeId<Type>) -> bool {
        self.hooks.contains_key(&ty)
    }

    /// Iterate user-authored drop hooks.
    pub fn hooks(&self) -> impl Iterator<Item = (LocalNodeId<Type>, LocalNodeId<Function>)> + '_ {
        self.hooks.iter().map(|(&ty, &function)| (ty, function))
    }

    /// Record the user-authored drop hook for a type.
    pub fn set_hook(
        &mut self,
        ty: LocalNodeId<Type>,
        function: LocalNodeId<Function>,
    ) -> Option<LocalNodeId<Function>> {
        self.hooks.insert(ty, function)
    }

    /// Return whether one inline child requires destruction.
    fn children_require_destructor(
        &self,
        ty: LocalNodeId<Type>,
        storage: Storage,
        tree: &mut Tree,
        seen: &mut FxIndexSet<LocalNodeId<Type>>,
    ) -> bool {
        let ty = Substitution::resolve(ty, tree);
        let definition = tree.get(ty).clone();

        match &definition {
            Type::Struct { fields, .. } => fields.iter().any(|field| {
                let ty = tree.get(*field).ty;

                self.child_requires_destructor(ty, storage, tree, seen)
            }),
            Type::Tuple { elements, .. } => elements
                .iter()
                .any(|element| self.child_requires_destructor(*element, storage, tree, seen)),
            Type::Newtype { inner, .. } => {
                self.child_requires_destructor(*inner, storage, tree, seen)
            }
            Type::FixedArray {
                element, length, ..
            } => {
                tree.static_value(*length).length() != Some(0)
                    && self.child_requires_destructor(*element, storage, tree, seen)
            }
            Type::Variant { cases, .. } => cases
                .iter()
                .any(|case| self.child_requires_destructor(case.ty, storage, tree, seen)),
            Type::Slice {
                kind: Reference::Unique,
                element,
                storage,
                ..
            } => self.child_requires_destructor(*element, *storage, tree, seen),
            _ => false,
        }
    }

    /// Return whether one owned child requires destruction.
    fn child_requires_destructor(
        &self,
        ty: LocalNodeId<Type>,
        storage: Storage,
        tree: &mut Tree,
        seen: &mut FxIndexSet<LocalNodeId<Type>>,
    ) -> bool {
        let representation = Substitution::resolve(ty, tree);
        if self.destructor(ty, storage).is_some()
            || self.hook(ty).is_some()
            || tree.get(representation).is_unique_storage()
        {
            return true;
        }

        if !seen.insert(ty) {
            return false;
        }

        let requires = self.children_require_destructor(ty, storage, tree, seen);
        seen.swap_remove(&ty);

        requires
    }
}
