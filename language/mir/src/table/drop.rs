use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{Function, LocalNodeId, Type};

/// Drop table for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DropTable {
    /// Generated destructors keyed by type id.
    pub destructors: HashMap<LocalNodeId<Type>, LocalNodeId<Function>>,
    /// User-authored drop hooks keyed by type id.
    pub hooks: HashMap<LocalNodeId<Type>, LocalNodeId<Function>>,
}

impl DropTable {
    /// Create an empty drop table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy drop table entries from one type id to another.
    pub fn copy_type_entries(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(destructor) = self.destructor(from) {
            self.set_destructor(to, destructor);
        }
        if let Some(hook) = self.hook(from) {
            self.set_hook(to, hook);
        }
    }

    /// Return the generated destructor for a type.
    pub fn destructor(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Function>> {
        self.destructors.get(&ty).copied()
    }

    /// Record the generated destructor for a type.
    pub fn set_destructor(
        &mut self,
        ty: LocalNodeId<Type>,
        destructor: LocalNodeId<Function>,
    ) -> Option<LocalNodeId<Function>> {
        self.destructors.insert(ty, destructor)
    }

    /// Return the user-authored drop hook for a type.
    pub fn hook(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Function>> {
        self.hooks.get(&ty).copied()
    }

    /// Record the user-authored drop hook for a type.
    pub fn set_hook(
        &mut self,
        ty: LocalNodeId<Type>,
        function: LocalNodeId<Function>,
    ) -> Option<LocalNodeId<Function>> {
        self.hooks.insert(ty, function)
    }
}
