use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, FunctionTerm, GenericInstance, TypeOperand};

/// Runtime call target resolved by the solver.
///
/// Examples:
/// ```ds
/// fn()
/// value()
/// receiver.method()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum CallTargetResolution {
    /// Callable expression without a declaration symbol.
    ///
    /// Examples:
    /// ```ds
    /// callback()
    /// ```
    Expression {
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
    },
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
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
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
        candidates: Vec<CandidateResolution>,
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
pub(in crate::check) struct CandidateResolution {
    /// The selected declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The selected generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
}

/// Runtime call selected by the solver before commit.
///
/// Examples:
/// ```ds
/// fn(value)
/// receiver.method(value)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallResolution {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The resolved call target.
    pub(in crate::check) target: CallTargetResolution,
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
    Resolved(CallResolution),
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
    ) -> CompilerResult<()> {
        if let Some(existing) = self.inference.call(source) {
            if existing == decision {
                return Ok(());
            }

            return Err(self.selection_conflict_error("call", source, &existing, &decision));
        }

        self.inference.select_call(source, decision);

        Ok(())
    }
}
