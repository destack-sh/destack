use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    AllocationSize, Block, CallArgumentEffect, Function, FunctionBehavior, Instruction,
    LocalNodeId, MemoryEffect,
};

/// Function and call metadata derived from semantic MIR.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FunctionMetadataTable {
    /// Metadata keyed by function id.
    pub functions: HashMap<LocalNodeId<Function>, FunctionMetadata>,
    /// Metadata keyed by callsite.
    pub calls: HashMap<CallSite, CallMetadata>,
}

impl FunctionMetadataTable {
    /// Create an empty function metadata table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return function metadata when present.
    pub fn function(&self, function: LocalNodeId<Function>) -> Option<&FunctionMetadata> {
        self.functions.get(&function)
    }

    /// Return mutable function metadata, inserting conservative defaults when absent.
    pub fn function_mut(&mut self, function: LocalNodeId<Function>) -> &mut FunctionMetadata {
        self.functions.entry(function).or_default()
    }

    /// Return call metadata when present.
    pub fn call(&self, callsite: CallSite) -> Option<&CallMetadata> {
        self.calls.get(&callsite)
    }

    /// Return mutable call metadata, inserting conservative defaults when absent.
    pub fn call_mut(&mut self, callsite: CallSite) -> &mut CallMetadata {
        self.calls.entry(callsite).or_default()
    }
}

/// Metadata for one function body or declaration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionMetadata {
    /// Memory touched by this function.
    pub memory: MemoryEffect,
    /// Behavioral effects of this function.
    pub behavior: FunctionBehavior,
    /// Allocation result size relation when known.
    pub allocation_size: Option<AllocationSize>,
}

/// Metadata for one callsite.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallMetadata {
    /// Memory touched by this call.
    pub memory: MemoryEffect,
    /// Behavioral effects of this call.
    pub behavior: FunctionBehavior,
    /// Allocation result size relation when known.
    pub allocation_size: Option<AllocationSize>,
    /// Resolved direct target when dispatch analysis proves one.
    pub target: Option<LocalNodeId<Function>>,
    /// Argument memory behavior when known.
    pub arguments: Vec<CallArgumentEffect>,
}

/// Stable identifier for one callsite inside a function body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallSite {
    /// Callsite stored as an instruction.
    Instruction(LocalNodeId<Instruction>),
    /// Callsite stored as a block terminator.
    Terminator(LocalNodeId<Block>),
}
