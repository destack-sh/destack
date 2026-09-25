use smallvec::SmallVec;
use tspp_dir as dir;

use crate::sema::{CheckFailure, OverloadRule, ValueUse, Verdict};

/// One decided goal over closed operands, keying its remembered choice.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::sema) struct GoalKey {
    /// The decided subject.
    pub(in crate::sema) goal: Goal,
    /// The operands, in goal order.
    pub(in crate::sema) operands: dir::TypeListId,
    /// The assuming template parameter and this content decide under.
    pub(in crate::sema) scope: Option<dir::GlobalGenericTemplateId>,
}

/// The subject one goal decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Goal {
    /// One implementation decision over [source, target].
    Implementation,
    /// One extension's deduced arguments over [receiver, subject].
    Extension {
        /// The extension declaration the match decides.
        extension: dir::GlobalSymbolId,
    },
    /// The callable selected over [expected.., callee, arguments..].
    Selection {
        /// The callable identity canonicalized.
        callee: Callee,
        /// Whether an expectation leads the canonical operands.
        expected: bool,
    },
    /// The union arm selected over [source, targets..].
    Arm {
        /// The expected value use.
        value_use: ValueUse,
        /// Whether the converted value owns an addressable place.
        is_placed: bool,
    },
}

/// The callable identity one selection decides for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Callee {
    /// A declared callable symbol.
    Symbol(dir::GlobalSymbolId),
    /// A newtype construction under one backing selection rule.
    Newtype(dir::GlobalSymbolId, OverloadRule),
}

/// One decided choice, re-derived at each goal site.
#[derive(Debug, Clone)]
pub(in crate::sema) enum Answer {
    /// The extension implementing the goal, none when every candidate fails.
    Implement(Option<dir::GlobalSymbolId>),
    /// The extension's match with its deduced arguments, none when it fails.
    Extension(Option<ExtensionSource>),
    /// The selected union arm by target position, exact or converted, or its failure.
    Arm(Result<(u16, bool), CheckFailure>),
    /// The candidate position one selection chose.
    Selection(usize),
}

/// One decided extension implementation verdict with its winner's match.
#[derive(Debug, Clone)]
pub(in crate::sema) struct Implementation {
    /// The decided verdict.
    pub(in crate::sema) verdict: Verdict,
    /// The winning extension declaration with the arguments its match binds.
    pub(in crate::sema) winner: Option<ExtensionSource>,
    /// The winner's substituted target, constrained by each goal site's receiver.
    pub(in crate::sema) target: Option<dir::GlobalTypeId>,
    /// The winner's matched interface application, related to each goal site's request.
    pub(in crate::sema) interface: Option<dir::GlobalTypeId>,
}

impl Implementation {
    /// Return the decision every candidate fails.
    pub(in crate::sema) fn fails() -> Self {
        Self {
            verdict: Verdict::Fails,
            winner: None,
            target: None,
            interface: None,
        }
    }
}

/// One matched extension with the arguments its template deduces.
#[derive(Debug, Clone)]
pub(in crate::sema) struct ExtensionSource {
    /// The matched extension declaration.
    pub(in crate::sema) extension: dir::GlobalSymbolId,
    /// The deduced template arguments, in declaration order.
    pub(in crate::sema) arguments: SmallVec<[dir::GlobalTypeId; 4]>,
}
