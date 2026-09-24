use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::sema::{FlowBranch, FlowCheckpoint, GeneratorTargets, InferMode, ReceiverBinding};

/// A function body currently being walked.
#[derive(Debug)]
pub(in crate::sema) struct FunctionFrame {
    /// The function symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The flow position before entering the function.
    pub(in crate::sema::flow) checkpoint: FlowCheckpoint,
    /// The first control target visible inside this function.
    pub(in crate::sema::flow) target_start: usize,
    /// The first try target visible inside this function.
    pub(in crate::sema::flow) try_start: usize,
    /// The receiver this function binds itself.
    pub(in crate::sema::flow) receiver: Option<ReceiverBinding>,
    /// The enclosing receiver a function value closes over.
    pub(in crate::sema::flow) enclosing_receiver: Option<ReceiverBinding>,
    /// The value accepted by `return` inside this function body, absent in constructors.
    pub(in crate::sema::flow) return_target: Option<dir::GlobalTypeId>,
    /// The yield targets when the body is a generator.
    pub(in crate::sema::flow) generator: Option<GeneratorTargets>,
    /// The declaration whose fields this constructor initializes.
    pub(in crate::sema::flow) initializes: Option<dir::GlobalSymbolId>,
    /// Literal inference applied to inferred returns and yields.
    pub(in crate::sema::flow) output_mode: InferMode,
    /// The function asynchrony.
    pub(in crate::sema) asynchrony: dir::Asynchrony,
    /// Outer symbols read by this function.
    pub(in crate::sema::flow) captured_symbols: FxIndexSet<dir::GlobalSymbolId>,
    /// Outer receiver read by this function.
    pub(in crate::sema::flow) captured_receiver: Option<ReceiverBinding>,
}

/// A structured control target currently visible to flow analysis.
#[derive(Debug)]
pub(in crate::sema) struct ControlTarget {
    /// The expression that introduced this target.
    pub(in crate::sema::flow) source: dir::GlobalNodeId<dir::Expression>,
    /// The optional source label.
    pub(in crate::sema::flow) label: Option<ControlLabel>,
    /// The source form that introduced this target.
    pub(in crate::sema::flow) form: ControlTargetForm,
    /// Flow branches collected at break sites.
    pub(in crate::sema::flow) break_branches: Vec<FlowBranch>,
    /// Flow branches collected at continue sites.
    pub(in crate::sema::flow) continue_branches: Vec<FlowBranch>,
    /// The flow position before entering the control body.
    pub(in crate::sema::flow) checkpoint: FlowCheckpoint,
}

/// One authored label attached to a control target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct ControlLabel {
    /// The authored label name.
    pub(in crate::sema) name: dir::StringId,
    /// The exact label binding.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
}

/// A source control form that accepts `break`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum ControlTargetForm {
    /// A `loop` accepts breaks with values and `continue`.
    Loop {
        /// The output joined by break values.
        result: dir::GlobalTypeId,
        /// The context the break values store under.
        mode: InferMode,
    },
    /// A conditional or iterating loop accepts value-less `break` and `continue`.
    Iteration,
    /// A switch accepts value-less `break` alone.
    Switch,
}

impl ControlTargetForm {
    /// Return whether `continue` may target this form.
    pub(in crate::sema) fn is_continue_target(self) -> bool {
        matches!(self, Self::Loop { .. } | Self::Iteration)
    }

    /// Return whether an unlabeled `break` may target this form.
    pub(in crate::sema) fn is_unlabeled_break_target(self) -> bool {
        matches!(self, Self::Loop { .. } | Self::Iteration | Self::Switch)
    }
}

/// A `try` body that can receive propagated failures.
#[derive(Debug)]
pub(in crate::sema) struct TryTarget {
    /// The receiving try expression.
    pub(in crate::sema::flow) node: dir::GlobalNodeId<dir::Expression>,
    /// The failure types the body propagated, one per try site, the values a catch binds.
    pub(in crate::sema::flow) failures: Vec<dir::GlobalTypeId>,
}
