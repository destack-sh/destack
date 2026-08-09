use destack_core::FxIndexSet;
use destack_dir as dir;

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
    /// The function asynchrony.
    pub(in crate::check) asynchrony: dir::Asynchrony,
    /// Outer symbols read by this function.
    pub(in crate::check::flow) captured_symbols: FxIndexSet<dir::GlobalSymbolId>,
    /// Outer receiver read by this function.
    pub(in crate::check::flow) captured_receiver: Option<ReceiverBinding>,
}

/// A structured control target currently visible to flow analysis.
#[derive(Debug)]
pub(in crate::check) struct ControlTarget {
    /// The optional source label.
    pub(in crate::check::flow) label: Option<ControlLabel>,
    /// The source form that introduced this target.
    pub(in crate::check::flow) form: ControlTargetForm,
    /// Flow branches collected at break sites.
    pub(in crate::check::flow) break_branches: Vec<FlowBranch>,
    /// Flow branches collected at continue sites.
    pub(in crate::check::flow) continue_branches: Vec<FlowBranch>,
    /// The flow position before entering the control body.
    pub(in crate::check::flow) checkpoint: FlowCheckpoint,
}

/// One authored label attached to a control target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ControlLabel {
    /// The authored label name.
    pub(in crate::check) name: dir::StringId,
    /// The labeled source statement.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

/// A source control form that accepts `break`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ControlTargetForm {
    /// A `loop` accepts breaks with values and `continue`.
    Loop {
        /// The output joined by break values.
        result: dir::GlobalTypeId,
    },
    /// A conditional or iterating loop accepts value-less `break` and `continue`.
    Iteration,
    /// A switch accepts value-less `break` but not `continue`.
    Switch,
}

impl ControlTargetForm {
    /// Return whether `continue` may target this form.
    pub(in crate::check) fn accepts_continue(self) -> bool {
        matches!(self, Self::Loop { .. } | Self::Iteration)
    }

    /// Return whether an unlabeled `break` may target this form.
    pub(in crate::check) fn accepts_unlabeled_break(self) -> bool {
        matches!(self, Self::Loop { .. } | Self::Iteration | Self::Switch)
    }
}

/// A `try` body that can receive propagated failures.
#[derive(Debug)]
pub(in crate::check) struct TryTarget {
    /// The residual types the body propagated, one per try site.
    pub(in crate::check::flow) residuals: Vec<dir::GlobalTypeId>,
}
