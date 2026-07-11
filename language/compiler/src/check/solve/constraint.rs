use destack_dir as dir;

use crate::check::{CauseId, OriginId, Relation};
use crate::{CompilerError, CompilerResult};

/// Component-global id of one collected constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ConstraintId(u32);

impl ConstraintId {
    /// Return the constraint id at one index.
    pub(in crate::check) fn at(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the constraint index.
    pub(in crate::check) fn index(self) -> usize {
        self.0 as usize
    }
}

/// One solver constraint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum Constraint {
    /// Pure relation between two types.
    Type(TypeConstraint),
    /// Relation between one source value occurrence and one target type.
    Value(ValueConstraint),
}

/// Pure relation between two types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TypeConstraint {
    /// The relation to enforce.
    pub(in crate::check) relation: Relation,
    /// The source operand.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// The target operand.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// Why this constraint exists.
    pub(in crate::check) cause: CauseId,
    /// The source subject blamed when this relation fails.
    pub(in crate::check) subject: Option<ConstraintSubject>,
}

/// Relation between one source value occurrence and one target type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct ValueConstraint {
    /// The relation to enforce.
    pub(in crate::check) relation: Relation,
    /// The source value type.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// The target value type.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// The source value occurrence used for value materialization.
    pub(in crate::check) value_origin: OriginId,
    /// Why this constraint exists.
    pub(in crate::check) cause: CauseId,
    /// The checked value role.
    pub(in crate::check) use_: Option<ValueUse>,
    /// Whether this relation only verifies and never bounds open variables.
    pub(in crate::check) is_check_only: bool,
}

/// Source subject blamed by one type constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintSubject {
    /// Generic application argument checked against its declared bound.
    ///
    /// Examples:
    /// ```ds
    /// Box<string>          // string satisfies Box<T: DynamicSafe>
    /// Borrowed<T, L, A>    // L satisfies Lifetime, A satisfies Access
    /// ```
    GenericArgument {
        /// The source node for the applied argument.
        source: dir::GlobalNodeIdAny,
    },
}

/// Runtime value use checked by one value constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum ValueUse {
    /// Value assigned into a storage or pattern target.
    ///
    /// Examples:
    /// ```ds
    /// const value: int32 = 1
    /// target = source
    /// const [first] = values
    /// ```
    Store,

    /// Value assigned into a call or subscript parameter.
    ///
    /// Examples:
    /// ```ds
    /// print(value)
    /// list[index]
    /// ```
    Argument,

    /// Function body value assigned into a return or yield result.
    ///
    /// Examples:
    /// ```ds
    /// return value
    /// yield value
    /// ```
    Output,

    /// Control-flow condition assigned to boolean.
    ///
    /// Examples:
    /// ```ds
    /// if (condition) {}
    /// while (condition) {}
    /// ```
    Condition,

    /// Value related against a written type without taking it.
    ///
    /// Examples:
    /// ```ds
    /// value satisfies Shape
    /// ```
    Satisfies,
}

impl Constraint {
    /// Create a pure relation between two types.
    pub(in crate::check) fn r#type(
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        cause: CauseId,
    ) -> Self {
        Self::Type(TypeConstraint {
            relation,
            source,
            target,
            cause,
            subject: None,
        })
    }

    /// Create a value constraint.
    pub(in crate::check) fn value(
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        value_origin: OriginId,
        cause: CauseId,
        use_: Option<ValueUse>,
    ) -> Self {
        Self::Value(ValueConstraint {
            relation,
            source,
            target,
            value_origin,
            cause,
            use_,
            is_check_only: false,
        })
    }

    /// Create a value constraint that verifies without bounding open variables.
    pub(in crate::check) fn check_only_value(
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        value_origin: OriginId,
        cause: CauseId,
        use_: Option<ValueUse>,
    ) -> Self {
        Self::Value(ValueConstraint {
            relation,
            source,
            target,
            value_origin,
            cause,
            use_,
            is_check_only: true,
        })
    }

    /// Return the relation to enforce.
    pub(in crate::check) fn relation(&self) -> Relation {
        match self {
            Self::Type(constraint) => constraint.relation,
            Self::Value(constraint) => constraint.relation,
        }
    }

    /// Return the source operand.
    pub(in crate::check) fn source(&self) -> dir::GlobalTypeId {
        match self {
            Self::Type(constraint) => constraint.source,
            Self::Value(constraint) => constraint.source,
        }
    }

    /// Return the target operand.
    pub(in crate::check) fn target(&self) -> dir::GlobalTypeId {
        match self {
            Self::Type(constraint) => constraint.target,
            Self::Value(constraint) => constraint.target,
        }
    }

    /// Return why this constraint exists.
    pub(in crate::check) fn cause(&self) -> CauseId {
        match self {
            Self::Type(constraint) => constraint.cause,
            Self::Value(constraint) => constraint.cause,
        }
    }

    /// Return the checked value use when this relation is a value constraint.
    pub(in crate::check) fn value_use(&self) -> Option<ValueUse> {
        match self {
            Self::Type(_) => None,
            Self::Value(constraint) => constraint.use_,
        }
    }
}

/// Solved state of one constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintState {
    /// The constraint has not finished solving.
    Pending,
    /// The constraint relation holds.
    Holds,
    /// The constraint relation failed and reported its diagnostic.
    Fails,
}

impl ConstraintState {
    /// Return whether the constraint is done.
    pub(in crate::check) fn is_done(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

/// Collected constraints with solver state, in allocation order.
#[derive(Debug, Default)]
pub(in crate::check) struct ConstraintTable {
    /// The collected constraints indexed by constraint id.
    constraints: Vec<Constraint>,
    /// Constraint states indexed by constraint id.
    states: Vec<ConstraintState>,
}

impl ConstraintTable {
    /// Create an empty constraint table.
    pub(in crate::check) fn new() -> Self {
        Self::default()
    }

    /// Append one constraint at the next id.
    pub(in crate::check) fn insert(&mut self, id: ConstraintId, constraint: Constraint) {
        debug_assert_eq!(self.constraints.len(), id.index());
        self.constraints.push(constraint);
        self.states.push(ConstraintState::Pending);
    }

    /// Truncate constraints undone by one probe rollback.
    pub(in crate::check) fn truncate(&mut self, count: usize) {
        self.constraints.truncate(count);
        self.states.truncate(count);
    }

    /// Return one constraint.
    pub(in crate::check) fn get(&self, id: ConstraintId) -> CompilerResult<&Constraint> {
        self.constraints
            .get(id.index())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} is not allocated"),
            })
    }

    /// Iterate over the collected constraints with their ids.
    pub(in crate::check) fn iter(&self) -> impl Iterator<Item = (ConstraintId, &Constraint)> {
        self.constraints
            .iter()
            .enumerate()
            .map(|(index, constraint)| (ConstraintId::at(index), constraint))
    }

    /// Return one constraint state.
    pub(in crate::check) fn state(&self, id: ConstraintId) -> CompilerResult<ConstraintState> {
        self.states
            .get(id.index())
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} has no solver state"),
            })
    }

    /// Set one constraint state.
    pub(in crate::check) fn set_state(&mut self, id: ConstraintId, state: ConstraintState) {
        self.states[id.index()] = state;
    }

    /// Return whether one constraint finished solving.
    pub(in crate::check) fn is_complete(&self, id: ConstraintId) -> bool {
        self.state(id).is_ok_and(ConstraintState::is_done)
    }

    /// Return the number of collected constraints.
    pub(in crate::check) fn count(&self) -> usize {
        self.constraints.len()
    }
}

/// Reason one closed check did not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckFailure {
    /// The relation itself did not hold.
    Relation,
    /// The failure was already reported at a finer judgment.
    Reported,
    /// Direct property literal missed one required key.
    MissingRequiredProperty {
        /// The missing key.
        key: dir::StaticKey,
    },
    /// Direct property literal supplied one unknown key.
    ExcessProperty {
        /// The excess key.
        key: dir::StaticKey,
    },
    /// Source type cannot satisfy one writable index signature target.
    WritableIndexRequiresIndexSet {
        /// The required writable index signature.
        signature: dir::TypeIndexSignature,
    },
}

/// Result of checking one source against one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckOutcome {
    /// The check holds.
    Holds,
    /// The check failed for one known reason.
    Fails(CheckFailure),
}

impl CheckOutcome {
    /// Return this check followed by another check.
    pub(in crate::check) fn and(self, next: Self) -> Self {
        match self {
            Self::Holds => next,
            Self::Fails(_) => self,
        }
    }

    /// Return the stored state for this completed check.
    pub(in crate::check) fn state(self) -> ConstraintState {
        match self {
            Self::Holds => ConstraintState::Holds,
            Self::Fails(_) => ConstraintState::Fails,
        }
    }
}

/// Applicability of one target-sensitive check path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckAttempt {
    /// The expression form does not use this target directly.
    NotApplicable,
    /// The expression form checked against this target.
    Checked(CheckOutcome),
}

// lock the queued constraint shapes
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Constraint>() == 80);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ValueConstraint>() == 64);
