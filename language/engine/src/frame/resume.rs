use crate::{BlockId, FrameLayoutId, FrameRegionId, FunctionId};

/// The identifier for one lowered resume point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResumePointId(pub u32);

/// The identifier for one resume-transfer table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResumeTransferId(pub u32);

/// One frame-region copy applied when resuming into one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResumeCopy {
    /// The source frame region.
    pub source: FrameRegionId,
    /// The destination frame region.
    pub destination: FrameRegionId,
}

/// One transfer recipe applied when resuming into one block.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResumeTransfer {
    /// The transfer identifier.
    pub id: ResumeTransferId,
    /// The block parameter copies applied when resuming here.
    pub copies: Vec<ResumeCopy>,
    /// The destination region for the resumed value when present.
    pub resume_value: Option<FrameRegionId>,
}

/// One continuation point inside one lowered function.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResumePoint {
    /// The resume point identifier.
    pub id: ResumePointId,
    /// The owning function.
    pub function: FunctionId,
    /// The owning frame layout.
    pub frame_layout: FrameLayoutId,
    /// The owning block.
    pub block: BlockId,
    /// The lowered instruction offset within the block.
    pub instruction_offset: u32,
    /// The source instruction boundary represented by this point.
    pub source_instruction_offset: u32,
    /// The transfer recipe applied when resuming here.
    pub transfer: Option<ResumeTransferId>,
}
