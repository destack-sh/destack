use destack_dir as dir;
use indexmap::IndexSet;

use crate::check::{FlowBranch, FlowCheckpoint, ReceiverCapture, VariableId};

/// A function body currently being walked.
#[derive(Debug)]
pub(in crate::check) struct FunctionFrame {
    /// The function symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The flow position before entering the function.
    pub(in crate::check) checkpoint: FlowCheckpoint,
    /// The first control target visible inside this function.
    pub(in crate::check) target_start: usize,
    /// The first try target visible inside this function.
    pub(in crate::check) try_start: usize,
    /// The lexical receiver visible inside this function.
    pub(in crate::check) receiver: Option<ReceiverCapture>,
    /// The value accepted by `return` inside this function body.
    pub(in crate::check) return_type: VariableId,
    /// The yielded value type for generator functions.
    pub(in crate::check) yield_type: Option<VariableId>,
    /// The value received when a generator resumes after `yield`.
    pub(in crate::check) resume_type: Option<VariableId>,
    /// The function asynchrony.
    pub(in crate::check) asynchrony: dir::Asynchrony,
    /// Outer symbols read by this function.
    pub(in crate::check) captured_symbols: IndexSet<dir::GlobalSymbolId>,
    /// Outer receiver read by this function.
    pub(in crate::check) captured_receiver: Option<ReceiverCapture>,
}

/// A structured control target currently visible to flow analysis.
#[derive(Debug)]
pub(in crate::check) struct ControlTarget {
    /// The optional source label.
    pub(in crate::check) label: Option<dir::StringId>,
    /// Whether `continue` may target this control frame.
    pub(in crate::check) allows_continue: bool,
    /// The result type receiving break values.
    pub(in crate::check) result: VariableId,
    /// Break values collected while walking the control body.
    pub(in crate::check) break_values: Vec<VariableId>,
    /// Flow branches collected at break sites.
    pub(in crate::check) break_branches: Vec<FlowBranch>,
    /// Flow branches collected at continue sites.
    pub(in crate::check) continue_branches: Vec<FlowBranch>,
    /// The flow position before entering the control body.
    pub(in crate::check) checkpoint: FlowCheckpoint,
}

/// A `try` body that can receive propagated failures.
#[derive(Debug)]
pub(in crate::check) struct TryTarget {
    /// The result type receiving propagated failures.
    pub(in crate::check) failure: VariableId,
    /// Failure values collected while walking the try body.
    pub(in crate::check) failures: Vec<VariableId>,
}
