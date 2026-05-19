use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalInstanceId, LocalTypeId};

/// Target selected by lexical or path lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameResolution {
    /// The selected symbol.
    pub symbol: GlobalSymbolId,
}

impl NameResolution {
    /// Create a name resolution.
    pub fn new(symbol: GlobalSymbolId) -> Self {
        Self { symbol }
    }
}

/// Target selected by a labeled transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelResolution {
    /// An explicit label target, such as `break outer`.
    Symbol(GlobalSymbolId),
    /// The nearest loop target.
    Loop,
    /// The nearest function target.
    Function,
}

/// Receiver member or protocol slot selected at a usage site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberResolution {
    /// The receiver type after inference.
    pub receiver: Option<LocalTypeId>,
    /// The selected member target.
    pub target: MemberTarget,
}

impl MemberResolution {
    /// Create a member resolution.
    pub fn new(receiver: Option<LocalTypeId>, target: MemberTarget) -> Self {
        Self { receiver, target }
    }
}

/// Member target selected at a usage site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemberTarget {
    /// Intrinsic primitive member or protocol slot.
    Intrinsic,
    /// Exactly one member known at compile time.
    Direct(MemberCandidate),
    /// Statically known variant member selection.
    Select(Vec<MemberCandidate>),
}

/// One member candidate after receiver lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: Option<LocalTypeId>,
    /// The selected member symbol.
    pub symbol: GlobalSymbolId,
    /// The instance of the member symbol, if statically applied.
    pub instance: Option<LocalInstanceId>,
}

/// Callable selected at a call site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallResolution {
    /// The selected callable target.
    pub target: CallTarget,
    /// The dynamic parameter types after static substitutions.
    pub parameters: Vec<LocalTypeId>,
    /// The return type after static substitutions.
    pub return_type: Option<LocalTypeId>,
}

impl CallResolution {
    /// Create a call resolution.
    pub fn new(
        target: CallTarget,
        parameters: Vec<LocalTypeId>,
        return_type: Option<LocalTypeId>,
    ) -> Self {
        Self {
            target,
            parameters,
            return_type,
        }
    }
}

/// Callable target selected at a call site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallTarget {
    /// Intrinsic primitive callable or protocol slot.
    Intrinsic {
        /// The receiver type.
        receiver: Option<LocalTypeId>,
    },
    /// Exactly one callable known at compile time.
    Direct(CallCandidate),
    /// Statically known variant callable selection.
    Select(Vec<CallCandidate>),
}

/// One callable candidate after overload selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: Option<LocalTypeId>,
    /// The selected callable symbol.
    pub symbol: GlobalSymbolId,
    /// The instance of the callable symbol, if statically applied.
    pub instance: Option<LocalInstanceId>,
}
