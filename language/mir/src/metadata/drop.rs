use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{DispatchSlot, Function, LocalNodeId, Type};

/// Drop metadata for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DropMetadata {
    /// Drop glue keyed by type id.
    pub glue_by_type: HashMap<LocalNodeId<Type>, DropGlue>,
}

impl DropMetadata {
    /// Create empty drop metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy drop metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(glue) = self.drop_glue(from).cloned() {
            self.set_drop_glue(to, glue);
        }
    }

    /// Return drop metadata for a type.
    pub fn drop_glue(&self, ty: LocalNodeId<Type>) -> Option<&DropGlue> {
        self.glue_by_type.get(&ty)
    }

    /// Record drop metadata for a type.
    pub fn set_drop_glue(&mut self, ty: LocalNodeId<Type>, glue: DropGlue) -> Option<DropGlue> {
        self.glue_by_type.insert(ty, glue)
    }
}

/// Drop glue selected for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropGlue {
    /// No custom drop work is required.
    None,
    /// Call a known drop function.
    Function {
        /// The drop function.
        function: LocalNodeId<Function>,
    },
    /// Dispatch through a runtime drop slot.
    Dynamic {
        /// The drop dispatch slot.
        slot: DispatchSlot,
    },
}
