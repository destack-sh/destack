use serde::{Deserialize, Serialize};

use crate::FrameMaterialization;

/// Logical frame state id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameStateId(pub u32);

/// Function id inside one program layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionId(pub u32);

/// Block id inside one function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u32);

/// Instruction index inside one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstructionIndex(pub u32);

/// Logical instruction point inside one program layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstructionPoint {
    /// The owning function.
    pub function: FunctionId,
    /// The owning block.
    pub block: BlockId,
    /// The instruction boundary inside the block.
    pub instruction: InstructionIndex,
}

/// Logical frame state at one resumable or reconstructable program point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameState {
    /// The frame state id.
    pub id: FrameStateId,
    /// The logical instruction point.
    pub point: InstructionPoint,
    /// The single frame reconstruction recipe.
    pub materialization: FrameMaterialization,
}
