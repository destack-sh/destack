use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{Function, LocalNodeId, Type};

/// Drop table for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DropTable {
    /// Full drop functions keyed by type id.
    pub functions: HashMap<LocalNodeId<Type>, LocalNodeId<Function>>,
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
        if let Some(function) = self.function(from) {
            self.set_function(to, function);
        }
        if let Some(hook) = self.hook(from) {
            self.set_hook(to, hook);
        }
    }

    /// Return the full drop function for a type.
    pub fn function(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Function>> {
        self.functions.get(&ty).copied()
    }

    /// Record the full drop function for a type.
    pub fn set_function(
        &mut self,
        ty: LocalNodeId<Type>,
        function: LocalNodeId<Function>,
    ) -> Option<LocalNodeId<Function>> {
        self.functions.insert(ty, function)
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
