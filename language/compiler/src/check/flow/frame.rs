use destack_dir as dir;
use indexmap::IndexSet;

use crate::check::{FlowBranch, FlowCheckpoint, ReceiverBinding};

/// A function body currently being walked.
#[derive(Debug)]
pub(in crate::check) struct FunctionFrame {
    /// The function symbol.
    pub(in crate::check::flow) symbol: dir::GlobalSymbolId,
    /// The flow position before entering the function.
    pub(in crate::check::flow) checkpoint: FlowCheckpoint,
    /// The first control target visible inside this function.
    pub(in crate::check::flow) target_start: usize,
    /// The first try target visible inside this function.
    pub(in crate::check::flow) try_start: usize,
    /// The lexical receiver visible inside this function.
    pub(in crate::check::flow) receiver: Option<ReceiverBinding>,
    /// The value accepted by `return` inside this function body.
    pub(in crate::check::flow) return_target: dir::GlobalTypeId,
    /// The value accepted by `yield` inside this generator body.
    pub(in crate::check::flow) yield_target: Option<dir::GlobalTypeId>,
    /// The value produced when this generator resumes after `yield`.
    pub(in crate::check::flow) resume_target: Option<dir::GlobalTypeId>,
    /// The function asynchrony.
    pub(in crate::check::flow) asynchrony: dir::Asynchrony,
    /// Outer symbols read by this function.
    pub(in crate::check::flow) captured_symbols: IndexSet<dir::GlobalSymbolId>,
    /// Outer receiver read by this function.
    pub(in crate::check::flow) captured_receiver: Option<ReceiverBinding>,
}

/// A structured control target currently visible to flow analysis.
#[derive(Debug)]
pub(in crate::check) struct ControlTarget {
    /// The optional source label.
    pub(in crate::check::flow) label: Option<dir::StringId>,
    /// Whether `continue` may target this control frame.
    pub(in crate::check::flow) allows_continue: bool,
    /// The expression node that owns this control frame.
    pub(in crate::check::flow) source: dir::GlobalNodeIdAny,
    /// The result type receiving break values.
    pub(in crate::check::flow) result: dir::GlobalTypeId,
    /// Break values collected while walking the control body.
    pub(in crate::check::flow) break_values: Vec<dir::GlobalTypeId>,
    /// Flow branches collected at break sites.
    pub(in crate::check::flow) break_branches: Vec<FlowBranch>,
    /// Flow branches collected at continue sites.
    pub(in crate::check::flow) continue_branches: Vec<FlowBranch>,
    /// The flow position before entering the control body.
    pub(in crate::check::flow) checkpoint: FlowCheckpoint,
}

/// A `try` body that can receive propagated failures.
#[derive(Debug)]
pub(in crate::check) struct TryTarget {
    /// The result type receiving propagated failures.
    pub(in crate::check::flow) failure: dir::GlobalTypeId,
    /// Failure values collected while walking the try body.
    pub(in crate::check::flow) failures: Vec<dir::GlobalTypeId>,
}
