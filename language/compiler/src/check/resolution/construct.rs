use destack_dir as dir;

use crate::check::{CallFailure, CheckState, FunctionTerm, GenericApplication};

/// Runtime construct target selected before commit.
///
/// Examples:
/// ```ds
/// new User()
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ConstructTargetSelection {
    /// Class construction selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// new User(name)
    /// new User()
    /// ```
    Class {
        /// The resolved class symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved explicit constructor symbol, when declared.
        constructor: Option<dir::GlobalSymbolId>,
        /// The resolved generic application.
        application: Option<GenericApplication>,
    },
    /// Newtype wrapper constructor selected at compile time.
    ///
    /// Examples:
    /// ```ds
    /// UserId(raw)
    /// ```
    Newtype {
        /// The resolved newtype symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic application.
        application: Option<GenericApplication>,
    },
}

/// Runtime construct selected by the solver before commit.
///
/// Examples:
/// ```ds
/// new User(name)
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ConstructSelection {
    /// The source construct expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The resolved construct target.
    pub(in crate::check) target: ConstructTargetSelection,
    /// The resolved constructor signature.
    pub(in crate::check) function: FunctionTerm,
}

/// Runtime construct failure resolved by the solver.
///
/// Examples:
/// ```ds
/// new Missing()
/// UserId(wrong)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstructFailure {
    /// The constructed value has no construct signature.
    ///
    /// Examples:
    /// ```ds
    /// new 1()
    /// ```
    NotConstructible,
    /// No construct overload accepts the arguments.
    ///
    /// Examples:
    /// ```ds
    /// UserId(wrong)
    /// ```
    NoMatch,
}

impl From<CallFailure> for ConstructFailure {
    /// Convert call failure detail to construct failure detail.
    fn from(failure: CallFailure) -> Self {
        match failure {
            CallFailure::NotCallable => Self::NotConstructible,
            CallFailure::NoMatch | CallFailure::ArgumentType { .. } => Self::NoMatch,
        }
    }
}

impl ConstructTargetSelection {
    /// Return the symbol selected by this construct target.
    pub(in crate::check) fn symbol(&self) -> dir::GlobalSymbolId {
        match self {
            Self::Class { symbol, .. } | Self::Newtype { symbol, .. } => *symbol,
        }
    }

    /// Return the generic application selected by this construct target.
    pub(in crate::check) fn application(&self) -> Option<&GenericApplication> {
        match self {
            Self::Class { application, .. } | Self::Newtype { application, .. } => {
                application.as_ref()
            }
        }
    }

    /// Return this construct target with its final generic application.
    pub(in crate::check) fn with_application(
        self,
        application: Option<GenericApplication>,
    ) -> Self {
        match self {
            Self::Class {
                symbol,
                constructor,
                ..
            } => Self::Class {
                symbol,
                constructor,
                application,
            },
            Self::Newtype { symbol, .. } => Self::Newtype {
                symbol,
                application,
            },
        }
    }
}

/// Runtime construct decision resolved by the solver.
///
/// Examples:
/// ```ds
/// new User(name)
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ConstructDecision {
    /// One construct target resolved.
    ///
    /// Examples:
    /// ```ds
    /// new User(name)
    /// ```
    Resolved(ConstructSelection),
    /// Construct resolution failed.
    ///
    /// Examples:
    /// ```ds
    /// new 1()
    /// ```
    Rejected(ConstructFailure),
}

impl CheckState<'_> {
    /// Select one construct decision.
    pub(in crate::check) fn select_construct(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: ConstructDecision,
    ) {
        if let Some(previous) = self.inference.construct(source) {
            assert_eq!(
                previous, decision,
                "check construct {source:?} already has a different decision"
            );

            return;
        }

        self.inference.insert_construct(source, decision);
    }
}
