use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;

use crate::check::{CauseId, CheckState, Relation};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Constraint {
    /// Pure relation between two types.
    Type(TypeConstraint),
    /// Relation between one source value occurrence and one target type.
    Value(ValueConstraint),
}

/// Pure relation between two types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct TypeConstraint {
    /// The relation to enforce.
    pub(in crate::check) relation: Relation,
    /// The source operand.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// The target operand.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// Why this constraint exists.
    pub(in crate::check) cause: CauseId,
    /// The generic application invalidated when this relation fails.
    pub(in crate::check) invalidated_application: Option<dir::GlobalTypeId>,
    /// Whether this constraint derives from a primary constraint.
    ///
    /// Derived constraints verify and reject candidates, but their failures
    /// duplicate the primary constraint and never report.
    pub(in crate::check) is_derived: bool,
}

/// Relation between one source value occurrence and one target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ValueConstraint {
    /// The relation to enforce.
    pub(in crate::check) relation: Relation,
    /// The value node being checked.
    pub(in crate::check) node: dir::GlobalNodeIdAny,
    /// The expected target.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// Why this constraint exists.
    pub(in crate::check) cause: CauseId,
    /// The checked value role.
    pub(in crate::check) use_: ValueUse,
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

impl ValueUse {
    /// Return whether this use stores the value in a runtime destination.
    pub(in crate::check) fn is_stored(self) -> bool {
        matches!(self, Self::Store | Self::Argument | Self::Output)
    }
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
            invalidated_application: None,
            is_derived: false,
        })
    }

    /// Create a derived relation forwarding one primary constraint's evidence.
    pub(in crate::check) fn derived(
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
            invalidated_application: None,
            is_derived: true,
        })
    }

    /// Create a generic argument bound constraint.
    pub(in crate::check) fn generic_bound(
        argument: dir::GlobalTypeId,
        bound: dir::GlobalTypeId,
        application: dir::GlobalTypeId,
        cause: CauseId,
    ) -> Self {
        Self::Type(TypeConstraint {
            relation: Relation::Satisfies,
            source: argument,
            target: bound,
            cause,
            invalidated_application: Some(application),
            is_derived: false,
        })
    }

    /// Create a value constraint.
    pub(in crate::check) fn value(
        relation: Relation,
        node: dir::GlobalNodeIdAny,
        target: dir::GlobalTypeId,
        cause: CauseId,
        use_: ValueUse,
    ) -> Self {
        Self::Value(ValueConstraint {
            relation,
            node,
            target,
            cause,
            use_,
        })
    }

    /// Return the relation to enforce.
    pub(in crate::check) fn relation(&self) -> Relation {
        match self {
            Self::Type(constraint) => constraint.relation,
            Self::Value(constraint) => constraint.relation,
        }
    }

    /// Return why this constraint exists.
    pub(in crate::check) fn cause(&self) -> CauseId {
        match self {
            Self::Type(constraint) => constraint.cause,
            Self::Value(constraint) => constraint.cause,
        }
    }

    /// Return whether this constraint derives from a primary constraint.
    pub(in crate::check) fn is_derived(&self) -> bool {
        match self {
            Self::Type(constraint) => constraint.is_derived,
            Self::Value(_) => false,
        }
    }

    /// Return the checked value use when this relation is a value constraint.
    pub(in crate::check) fn value_use(&self) -> Option<ValueUse> {
        match self {
            Self::Type(_) => None,
            Self::Value(constraint) => Some(constraint.use_),
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
    /// Runtime coercions produced by completed value checks.
    coercions: Vec<Option<Box<dir::Coercion>>>,
    /// Ids of collected constraints by value, so one task collects once.
    interned: FxIndexMap<Constraint, ConstraintId>,
}

impl ConstraintTable {
    /// Create an empty constraint table.
    pub(in crate::check) fn new() -> Self {
        Self::default()
    }

    /// Return the id one constraint already collected under.
    pub(in crate::check) fn lookup(&self, constraint: &Constraint) -> Option<ConstraintId> {
        self.interned.get(constraint).copied()
    }

    /// Append one constraint at the next id.
    pub(in crate::check) fn insert(&mut self, id: ConstraintId, constraint: Constraint) {
        debug_assert_eq!(self.constraints.len(), id.index());
        self.constraints.push(constraint);
        self.states.push(ConstraintState::Pending);
        self.coercions.push(None);
        self.interned.insert(constraint, id);
    }

    /// Truncate constraints undone by one probe rollback.
    pub(in crate::check) fn truncate(&mut self, count: usize) {
        self.constraints.truncate(count);
        self.states.truncate(count);
        self.coercions.truncate(count);
        // every id interns one entry in allocation order, so the tables truncate together
        self.interned.truncate(count);
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

    /// Return the runtime coercion produced by one completed value check.
    pub(in crate::check) fn coercion(
        &self,
        id: ConstraintId,
    ) -> CompilerResult<Option<&dir::Coercion>> {
        self.coercions
            .get(id.index())
            .map(|coercion| coercion.as_deref())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} has no coercion slot"),
            })
    }

    /// Set one constraint result.
    pub(in crate::check) fn set_result(
        &mut self,
        id: ConstraintId,
        state: ConstraintState,
        coercion: Option<Box<dir::Coercion>>,
    ) {
        self.states[id.index()] = state;
        self.coercions[id.index()] = coercion;
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

impl CheckState<'_> {
    /// Collect generic applications whose declared argument bounds failed.
    pub(in crate::check) fn failed_generic_applications(
        &self,
    ) -> CompilerResult<FxIndexSet<dir::GlobalTypeId>> {
        let mut applications = FxIndexSet::default();
        for (id, constraint) in self.solver.constraints.iter() {
            if self.solver.constraints.state(id)? != ConstraintState::Fails {
                continue;
            }
            let Constraint::Type(TypeConstraint {
                invalidated_application: Some(application),
                ..
            }) = constraint
            else {
                continue;
            };
            applications.insert(*application);
        }

        Ok(applications)
    }
}

/// Reason one closed check did not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckFailure {
    /// The relation itself did not hold.
    Relation,
    /// The failure was already reported at a finer constraint.
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

/// Result of checking one source node against a contextual target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ValueCheck {
    /// Whether the target-directed check held.
    pub(in crate::check) outcome: CheckOutcome,
    /// The concrete contextual target checked against the value.
    pub(in crate::check) target: dir::GlobalTypeId,
}

/// Solved relation between one runtime value and its target type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ValueRelation {
    /// Whether the value relation held.
    pub(in crate::check) outcome: CheckOutcome,
    /// The runtime coercion required by the related value.
    pub(in crate::check) coercion: Option<Box<dir::Coercion>>,
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
    Checked(ValueCheck),
}

