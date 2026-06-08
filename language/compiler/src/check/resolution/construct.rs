use destack_dir as dir;

use crate::check::{CallFailure, FunctionTerm, GenericInstance};

/// Runtime construct target selected before commit.
///
/// Examples:
/// ```ds
/// new User()
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ConstructTargetResolution {
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
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
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
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
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
pub(in crate::check) struct ConstructResolution {
    /// The source construct expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The resolved construct target.
    pub(in crate::check) target: ConstructTargetResolution,
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

impl ConstructTargetResolution {
    /// Return the symbol selected by this construct target.
    pub(in crate::check) fn symbol(&self) -> dir::GlobalSymbolId {
        match self {
            Self::Class { symbol, .. } | Self::Newtype { symbol, .. } => *symbol,
        }
    }

    /// Return the generic instance selected by this construct target.
    pub(in crate::check) fn instance(&self) -> Option<&GenericInstance> {
        match self {
            Self::Class { instance, .. } | Self::Newtype { instance, .. } => instance.as_ref(),
        }
    }

    /// Return this construct target with its final generic instance.
    pub(in crate::check) fn with_application(self, instance: Option<GenericInstance>) -> Self {
        match self {
            Self::Class {
                symbol,
                constructor,
                ..
            } => Self::Class {
                symbol,
                constructor,
                instance,
            },
            Self::Newtype { symbol, .. } => Self::Newtype { symbol, instance },
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
    Resolved(ConstructResolution),
    /// Construct resolution failed.
    ///
    /// Examples:
    /// ```ds
    /// new 1()
    /// ```
    Rejected(ConstructFailure),
}
