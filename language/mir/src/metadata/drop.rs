use destack_serde::Reflect;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{DispatchSlot, Function, LocalNodeId, Type};

/// Drop metadata for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DropMetadata {
    /// Full drop glue keyed by type id.
    pub glue_by_type: HashMap<LocalNodeId<Type>, DropGlue>,
    /// User-authored drop hooks keyed by type id.
    pub hooks_by_type: HashMap<LocalNodeId<Type>, DropHook>,
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
        if let Some(hook) = self.drop_hook(from).cloned() {
            self.set_drop_hook(to, hook);
        }
    }

    /// Return full drop glue for a type.
    pub fn drop_glue(&self, ty: LocalNodeId<Type>) -> Option<&DropGlue> {
        self.glue_by_type.get(&ty)
    }

    /// Record full drop glue for a type.
    pub fn set_drop_glue(&mut self, ty: LocalNodeId<Type>, glue: DropGlue) -> Option<DropGlue> {
        self.glue_by_type.insert(ty, glue)
    }

    /// Return the user-authored drop hook for a type.
    pub fn drop_hook(&self, ty: LocalNodeId<Type>) -> Option<&DropHook> {
        self.hooks_by_type.get(&ty)
    }

    /// Record the user-authored drop hook for a type.
    pub fn set_drop_hook(&mut self, ty: LocalNodeId<Type>, hook: DropHook) -> Option<DropHook> {
        self.hooks_by_type.insert(ty, hook)
    }
}

/// Full drop glue selected for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DropGlue {
    /// No drop glue is required.
    None,
    /// Call compiler-generated drop glue.
    Generated {
        /// The drop function.
        function: LocalNodeId<Function>,
    },
    /// Dispatch through a runtime drop slot.
    Dynamic {
        /// The drop dispatch slot.
        slot: DispatchSlot,
    },
}

impl DropGlue {
    /// Return the concrete function backing this glue.
    pub fn function(&self) -> Option<LocalNodeId<Function>> {
        match self {
            DropGlue::Generated { function } => Some(*function),
            DropGlue::None | DropGlue::Dynamic { .. } => None,
        }
    }

    /// Return whether this glue is generated for the given function.
    pub fn is_generated_function(&self, target: LocalNodeId<Function>) -> bool {
        matches!(self, DropGlue::Generated { function } if *function == target)
    }
}

/// User-authored drop hook for one MIR type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DropHook {
    /// The hook function.
    pub function: LocalNodeId<Function>,
}

impl DropHook {
    /// Return whether this hook calls the given function.
    pub fn is_function(&self, target: LocalNodeId<Function>) -> bool {
        self.function == target
    }
}
