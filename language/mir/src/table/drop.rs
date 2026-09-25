use destack_core::{FxIndexMap, FxIndexSet};

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{Function, LocalNodeId, Reference, Space, Storage, Substitution, Tree, Type, TypeId};

/// Drop table for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DropTable {
    /// Generated destructors keyed by type and storage.
    destructors: FxIndexMap<(TypeId, Storage), LocalNodeId<Function>>,
    /// User-authored drop hooks keyed by type.
    hooks: FxIndexMap<TypeId, LocalNodeId<Function>>,
}

impl DropTable {
    /// Create an empty drop table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy drop table entries from one type id to another.
    pub fn copy_type_entries(&mut self, from: TypeId, to: TypeId) {
        let destructors = self
            .destructors
            .iter()
            .filter_map(|(&(ty, storage), &function)| (ty == from).then_some((storage, function)))
            .collect::<Vec<_>>();
        for (storage, function) in destructors {
            self.set_destructor(to, storage, function);
        }

        if let Some(function) = self.hooks.get(&from).copied() {
            self.set_hook(to, function);
        }
    }

    /// Return the generated destructor for a type in one storage.
    pub fn destructor(&self, ty: TypeId, storage: Storage) -> Option<LocalNodeId<Function>> {
        self.destructors.get(&(ty, storage)).copied()
    }

    /// Iterate generated destructors.
    pub fn destructors(
        &self,
    ) -> impl Iterator<Item = (TypeId, Storage, LocalNodeId<Function>)> + '_ {
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
    pub fn requires_destructor(&self, ty: TypeId, storage: Storage, tree: &Tree) -> bool {
        if self.destructor(ty, storage).is_some() || self.has_hook(ty) {
            return true;
        }

        self.children_require_destructor(ty, storage, tree, &mut FxIndexSet::default())
    }

    /// Return whether releasing one unique allocation destroys values, none for other types.
    pub fn release_destroys(&self, ty: TypeId, tree: &Tree) -> Option<bool> {
        // read the answer in one heap, the same in every heap
        let storage = Storage::Heap(Space::Local);
        match tree.type_definition(tree.storage_type(ty)) {
            Type::Reference {
                kind: Reference::Unique,
                pointee,
                ..
            } => Some(self.requires_destructor(*pointee, storage, tree)),
            Type::Slice {
                kind: Reference::Unique,
                element,
                ..
            } => Some(self.requires_destructor(*element, storage, tree)),
            Type::Dynamic { .. } | Type::Function { .. } => Some(true),
            _ => None,
        }
    }

    /// Record the generated destructor for a type in one storage.
    pub fn set_destructor(
        &mut self,
        ty: TypeId,
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

    /// Return the user-authored drop hook for a type.
    pub fn hook(&self, ty: TypeId) -> Option<LocalNodeId<Function>> {
        self.hooks.get(&ty).copied()
    }

    /// Return whether a user-authored drop hook exists for a type.
    pub fn has_hook(&self, ty: TypeId) -> bool {
        self.hooks.contains_key(&ty)
    }

    /// Iterate user-authored drop hooks.
    pub fn hooks(&self) -> impl Iterator<Item = (TypeId, LocalNodeId<Function>)> + '_ {
        self.hooks.iter().map(|(&ty, &function)| (ty, function))
    }

    /// Record the user-authored drop hook for a type.
    pub fn set_hook(
        &mut self,
        ty: TypeId,
        function: LocalNodeId<Function>,
    ) -> Option<LocalNodeId<Function>> {
        self.hooks.insert(ty, function)
    }

    /// Return whether one inline child requires destruction.
    fn children_require_destructor(
        &self,
        ty: TypeId,
        storage: Storage,
        tree: &Tree,
        seen: &mut FxIndexSet<TypeId>,
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
                ..
            } => self.child_requires_destructor(*element, Storage::Heap(Space::Local), tree, seen),
            _ => false,
        }
    }

    /// Return whether one owned child requires destruction.
    fn child_requires_destructor(
        &self,
        ty: TypeId,
        storage: Storage,
        tree: &Tree,
        seen: &mut FxIndexSet<TypeId>,
    ) -> bool {
        let representation = Substitution::resolve(ty, tree);
        if self.destructor(ty, storage).is_some()
            || self.has_hook(ty)
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
