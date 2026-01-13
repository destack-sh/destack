use crate::metadata::DeoptMapId;

/// Native machine state captured at a safepoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeState {
    /// Program counter.
    pub pc: u64,
    /// Frame pointer register.
    pub frame_pointer: u64,
    /// Stack pointer register.
    pub stack_pointer: u64,
    /// General-purpose register file.
    pub registers: Vec<u64>,
}

/// Deoptimization reason metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeoptReason {
    /// Reason code.
    pub code: u16,
    /// Site identifier for diagnostics.
    pub site_id: u32,
}

/// Deoptimization request payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeoptRequest {
    /// Deopt map index for reconstruction.
    pub deopt_map: DeoptMapId,
    /// Captured native machine state.
    pub state: NativeState,
    /// Deoptimization reason metadata.
    pub reason: DeoptReason,
}
