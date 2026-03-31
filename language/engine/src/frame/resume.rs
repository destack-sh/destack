use destack_mir as mir;

use crate::FrameLayoutId;

/// The identifier for one semantic resume point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResumePointId(pub u32);

/// The identifier for one resume-transfer table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResumeTransferId(pub u32);

/// One copy edge applied when resuming into one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResumeCopy {
    /// The source value slot in the suspended frame.
    pub source: u32,
    /// The destination value slot in the resumed block.
    pub destination: u32,
}

/// One transfer recipe applied when resuming into one block.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResumeTransfer {
    /// The transfer identifier.
    pub id: ResumeTransferId,
    /// The block parameter copies applied when resuming here.
    pub copies: Vec<ResumeCopy>,
    /// The destination for the resumed value when present.
    pub resume_value: Option<u32>,
}

/// One semantic continuation point inside one lowered function.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResumePoint {
    /// The resume point identifier.
    pub id: ResumePointId,
    /// The owning MIR function.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The owning frame layout.
    pub frame_layout: FrameLayoutId,
    /// The owning MIR block.
    pub block: mir::LocalNodeId<mir::Block>,
    /// The lowered instruction offset within the block.
    pub instruction_offset: u32,
    /// The MIR instruction boundary represented by this point.
    pub mir_instruction_offset: u32,
    /// The transfer recipe applied when resuming here.
    pub transfer: Option<ResumeTransferId>,
}
