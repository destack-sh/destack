use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_mir as mir;

use crate::{FunctionId, TypeId};

/// Native code map for entries, safepoints, roots, and deoptimization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CodeMap {
    /// Native function code ranges.
    function: Vec<FunctionCode>,
    /// Native continuation resume code ranges.
    resume: Vec<ResumeCode>,
    /// Native safepoints keyed by safepoint id.
    safepoint: Vec<Option<Safepoint>>,
}

impl CodeMap {
    /// Create one native code map.
    pub fn new(
        function: Vec<FunctionCode>,
        resume: Vec<ResumeCode>,
        safepoint: Vec<Option<Safepoint>>,
    ) -> Self {
        Self {
            function,
            resume,
            safepoint,
        }
    }

    /// Create one empty native code map.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return native function code ranges.
    pub fn functions(&self) -> &[FunctionCode] {
        &self.function
    }

    /// Return native resume code ranges.
    pub fn resumes(&self) -> &[ResumeCode] {
        &self.resume
    }

    /// Return native safepoints.
    pub fn safepoints(&self) -> &[Option<Safepoint>] {
        &self.safepoint
    }

    /// Return one safepoint by id.
    pub fn safepoint(&self, id: u32) -> Option<&Safepoint> {
        self.safepoint.get(id as usize).and_then(Option::as_ref)
    }
}

/// One native function code range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionCode {
    /// The function covered by this range.
    pub function: FunctionId,
    /// The native code byte range.
    pub range: CodeRange,
}

impl FunctionCode {
    /// Create one native function code range.
    pub fn new(function: FunctionId, range: CodeRange) -> Self {
        Self { function, range }
    }
}

/// One native resume entry code range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ResumeCode {
    /// The frame state resumed by this range.
    pub frame_state: mir::FrameStateId,
    /// The native code byte range.
    pub range: CodeRange,
}

impl ResumeCode {
    /// Create one native resume code range.
    pub fn new(frame_state: mir::FrameStateId, range: CodeRange) -> Self {
        Self { frame_state, range }
    }
}

/// One native image byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CodeRange {
    /// The byte offset from the native image base.
    pub offset: u32,
    /// The byte length of this range.
    pub byte_len: u32,
}

impl CodeRange {
    /// Create one native image byte range.
    pub const fn new(offset: u32, byte_len: u32) -> Self {
        Self { offset, byte_len }
    }
}

/// One native safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Safepoint {
    /// The safepoint id passed through the native ABI.
    pub id: u32,
    /// The function containing this safepoint.
    pub function: FunctionId,
    /// The byte offset from the native image base.
    pub offset: u32,
    /// The VM frame state corresponding to this safepoint.
    pub frame_state: mir::FrameStateId,
    /// Native roots live at this safepoint.
    pub roots: Vec<NativeRoot>,
    /// Materialization target when this safepoint can deoptimize.
    pub deopt: Option<mir::FrameStateId>,
}

impl Safepoint {
    /// Create one native safepoint.
    pub fn new(
        id: u32,
        function: FunctionId,
        offset: u32,
        frame_state: mir::FrameStateId,
        roots: Vec<NativeRoot>,
        deopt: Option<mir::FrameStateId>,
    ) -> Self {
        Self {
            id,
            function,
            offset,
            frame_state,
            roots,
            deopt,
        }
    }
}

/// One native root location at one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NativeRoot {
    /// Signed byte offset from the native frame base.
    pub offset: i32,
    /// The root value type.
    pub ty: TypeId,
}

impl NativeRoot {
    /// Create one native root location.
    pub fn new(offset: i32, ty: TypeId) -> Self {
        Self { offset, ty }
    }
}
