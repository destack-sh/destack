use destack_dir as dir;

use crate::check::{CheckState, FunctionTerm, GenericApplication, TypeOperand};

/// Runtime call target resolved by the solver.
///
/// Examples:
/// ```ds
/// fn()
/// value()
/// receiver.method()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum CallTargetSelection {
    /// Callable expression without a declaration symbol.
    ///
    /// Examples:
    /// ```ds
    /// callback()
    /// ```
    Expression,
    /// Symbol-backed callable selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// print(value)
    /// receiver.method()
    /// ```
    Symbol {
        /// The resolved callable symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic application.
        application: Option<GenericApplication>,
        /// The resolved receiver type for method calls.
        receiver: Option<TypeOperand>,
    },
    /// Symbol-backed callable variants selected from a union receiver.
    ///
    /// Examples:
    /// ```ds
    /// value.method()
    /// ```
    Union {
        /// The resolved callable candidates.
        candidates: Vec<CandidateSelection>,
        /// The resolved receiver type for method calls.
        receiver: Option<TypeOperand>,
    },
}

/// One symbol-backed candidate selected by check.
///
/// Examples:
/// ```ds
/// value.method()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CandidateSelection {
    /// The selected declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The selected generic application.
    pub(in crate::check) application: Option<GenericApplication>,
}

/// Runtime call selected by the solver before commit.
///
/// Examples:
/// ```ds
/// fn(value)
/// receiver.method(value)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallSelection {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The resolved call target.
    pub(in crate::check) target: CallTargetSelection,
    /// The resolved function signature.
    pub(in crate::check) function: FunctionTerm,
}

/// Runtime call failure resolved by the solver.
///
/// Examples:
/// ```ds
/// value()
/// fn(value)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum CallFailure {
    /// The callee value has no call signature.
    ///
    /// Examples:
    /// ```ds
    /// 1()
    /// ```
    NotCallable,
    /// No callable overload accepts the arguments.
    ///
    /// Examples:
    /// ```ds
    /// fn(wrong)
    /// ```
    NoMatch,
    /// One selected callable target rejected an argument type.
    ///
    /// Examples:
    /// ```ds
    /// fn(wrong)
    /// ```
    ArgumentType {
        /// The incompatible argument type operand.
        argument: TypeOperand,
        /// The expected parameter type operand.
        parameter: TypeOperand,
    },
}

/// Runtime call decision resolved by the solver.
///
/// Examples:
/// ```ds
/// fn(value)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum CallDecision {
    /// One call target resolved.
    ///
    /// Examples:
    /// ```ds
    /// fn(value)
    /// ```
    Resolved(CallSelection),
    /// Call resolution failed.
    ///
    /// Examples:
    /// ```ds
    /// 1()
    /// ```
    Rejected(CallFailure),
}

impl CheckState<'_> {
    /// Select one call decision.
    pub(in crate::check) fn select_call(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: CallDecision,
    ) {
        if let Some(previous) = self.inference.call(source) {
            assert_eq!(
                previous, decision,
                "check call {source:?} already has a different decision"
            );

            return;
        }

        self.inference.insert_call(source, decision);
    }
}
