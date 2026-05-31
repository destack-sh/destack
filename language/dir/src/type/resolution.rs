use serde::{Deserialize, Serialize};

use crate::{
    BinaryOperator, GlobalSymbolId, LocalGenericApplicationId, LocalTypeId, StaticKey,
    UnaryOperator,
};

/// Receiver selected by contextual lookup, such as `this` or `super`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiverResolution {
    /// The receiver syntax kind.
    pub kind: ReceiverKind,
    /// The declaration that introduces the receiver, when symbol-backed.
    pub owner: Option<GlobalSymbolId>,
    /// The receiver type after inference.
    pub ty: Option<LocalTypeId>,
}

/// Receiver syntax resolved by contextual lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiverKind {
    /// The active `this` receiver.
    This,
    /// The active superclass receiver.
    Super,
}

/// Target selected by lexical or path lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameResolution {
    /// The selected symbols in declaration order.
    pub symbols: Vec<GlobalSymbolId>,
}

impl NameResolution {
    /// Create a single-symbol name resolution.
    pub fn new(symbol: GlobalSymbolId) -> Self {
        Self {
            symbols: vec![symbol],
        }
    }

    /// Create a name resolution from selected symbols.
    pub fn from_symbols(symbols: Vec<GlobalSymbolId>) -> Self {
        Self { symbols }
    }

    /// Return the first selected symbol.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        self.symbols.first().copied()
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
    /// Compiler builtin selected at a member usage site.
    Builtin(BuiltinMember),
    /// Structural field selected from a shape type.
    Field(StaticKey),
    /// Exactly one symbol-backed member selected at compile time.
    Symbol(MemberCandidate),
    /// Statically known variant member selection.
    Select(Vec<MemberCandidate>),
}

/// Compiler builtin member selected at a usage site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltinMember {
    /// Indexed element access, such as `value[index]`.
    Index,
    /// Range slice access, such as `value[start..end]`.
    Slice,
}

/// One member candidate after receiver lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: Option<LocalTypeId>,
    /// The selected member symbol.
    pub symbol: GlobalSymbolId,
    /// The instance of the member symbol, if statically applied.
    pub instance: Option<LocalGenericApplicationId>,
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
    /// Compiler builtin selected at a usage site.
    Builtin(BuiltinCall),
    /// Callable value without a declaration symbol.
    Value,
    /// Exactly one symbol-backed constructor selected at compile time.
    Construct(CallCandidate),
    /// Exactly one symbol-backed callable selected at compile time.
    Symbol(CallCandidate),
    /// Statically known variant callable selection.
    Select(Vec<CallCandidate>),
}

/// Compiler builtin callable selected at a usage site.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BuiltinCall {
    /// Builtin unary operator behavior.
    UnaryOperator {
        /// The source operator.
        operator: UnaryOperator,
    },
    /// Builtin binary operator behavior.
    BinaryOperator {
        /// The source operator.
        operator: BinaryOperator,
    },
}

/// One callable candidate after overload selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallCandidate {
    /// The receiver type that selects this candidate.
    pub receiver: Option<LocalTypeId>,
    /// The selected callable symbol.
    pub symbol: GlobalSymbolId,
    /// The instance of the callable symbol, if statically applied.
    pub instance: Option<LocalGenericApplicationId>,
}
