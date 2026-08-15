use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{Function, LocalNodeId, Storage, Type};

/// Drop table for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DropTable {
    /// Generated destructors keyed by type and storage.
    destructors: HashMap<(LocalNodeId<Type>, Storage), LocalNodeId<Function>>,
    /// User-authored drop hooks keyed by type and storage.
    hooks: HashMap<(LocalNodeId<Type>, Storage), LocalNodeId<Function>>,
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

        let hooks = self
            .hooks
            .iter()
            .filter_map(|(&(ty, storage), &function)| (ty == from).then_some((storage, function)))
            .collect::<Vec<_>>();
        for (storage, function) in hooks {
            self.set_hook(to, storage, function);
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

    /// Record the generated destructor for a type in one storage.
    pub fn set_destructor(
        &mut self,
        ty: LocalNodeId<Type>,
        storage: Storage,
        destructor: LocalNodeId<Function>,
    ) -> Option<LocalNodeId<Function>> {
        self.destructors.insert((ty, storage), destructor)
    }

    /// Return the user-authored drop hook for a type in one storage.
    pub fn hook(&self, ty: LocalNodeId<Type>, storage: Storage) -> Option<LocalNodeId<Function>> {
        self.hooks.get(&(ty, storage)).copied()
    }

    /// Return whether any user-authored drop hook exists for a type.
    pub fn has_hook(&self, ty: LocalNodeId<Type>) -> bool {
        self.hooks.keys().any(|(candidate, _)| *candidate == ty)
    }

    /// Record the user-authored drop hook for a type in one storage.
    pub fn set_hook(
        &mut self,
        ty: LocalNodeId<Type>,
        storage: Storage,
        function: LocalNodeId<Function>,
    ) -> Option<LocalNodeId<Function>> {
        self.hooks.insert((ty, storage), function)
    }
}
