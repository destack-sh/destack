use destack_mir as mir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::Point;

/// The logical storage shape of one function frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Frame {
    /// The function owning this frame.
    pub function: mir::FunctionId,
    /// SSA values retained by this frame.
    pub values: Vec<FrameValue>,
    /// Locals retained by this frame.
    pub locals: Vec<FrameLocal>,
    /// The callable environment type when present.
    pub environment: Option<mir::TypeId>,
}

/// One SSA value retained by a logical frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameValue {
    /// The MIR value identity.
    pub value: mir::Value,
    /// The stored value type.
    pub ty: mir::TypeId,
}

/// One local retained by a logical frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameLocal {
    /// The MIR local identity.
    pub local: mir::LocalId,
    /// The stored local type.
    pub ty: mir::TypeId,
}

/// Live logical slots at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameState {
    /// The safepoint represented by this state.
    pub point: Point,
    /// Live SSA values in value identity order.
    pub values: Vec<mir::Value>,
    /// Live locals in local identity order.
    pub locals: Vec<mir::LocalId>,
    /// Whether the callable environment is live.
    pub environment: bool,
}
