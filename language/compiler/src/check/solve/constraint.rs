use crate::check::{Origin, Relation};
use crate::{CompilerError, CompilerResult};
use destack_dir as dir;
use indexmap::IndexMap;

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
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Constraint {
    /// Pure relation between two types.
    Type(TypeConstraint),
    /// Relation between one source value occurrence and one target type.
    Value(ValueConstraint),
}

/// Pure relation between two types.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TypeConstraint {
    /// The relation to enforce.
    pub(in crate::check) relation: Relation,
    /// The left operand, the source for directed relations.
    pub(in crate::check) left: dir::GlobalTypeId,
    /// The right operand, the target for directed relations.
    pub(in crate::check) right: dir::GlobalTypeId,
    /// The source that produced the constraint.
    pub(in crate::check) origin: Origin,
    /// The source subject blamed when this relation fails.
    pub(in crate::check) subject: Option<ConstraintSubject>,
}

/// Relation between one source value occurrence and one target type.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ValueConstraint {
    /// The relation to enforce.
    pub(in crate::check) relation: Relation,
    /// The source value type.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// The target value type.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// The source value occurrence used for value materialization.
    pub(in crate::check) value_origin: Origin,
    /// The source that produced the relation.
    pub(in crate::check) origin: Origin,
    /// The checked value role.
    pub(in crate::check) use_: Option<ValueUse>,
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
}

/// Reason one closed constraint did not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintFailure {
    /// The relation itself did not hold.
    Relation,
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

/// Result of checking one closed constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintCheck {
    /// The constraint holds.
    Holds,
    /// The constraint failed for one known reason.
    Fails(ConstraintFailure),
}

impl ConstraintCheck {
    /// Return the stored state for this completed check.
    pub(in crate::check) fn state(self) -> ConstraintState {
        match self {
            Self::Holds => ConstraintState::Holds,
            Self::Fails(_) => ConstraintState::Fails,
        }
    }
}

impl Constraint {
    /// Create a pure relation between two types.
    pub(in crate::check) fn r#type(
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        origin: Origin,
    ) -> Self {
        Self::Type(TypeConstraint {
            relation,
            left,
            right,
            origin,
            subject: None,
        })
    }

    /// Create a value constraint.
    pub(in crate::check) fn value(
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        value_origin: Origin,
        origin: Origin,
        use_: Option<ValueUse>,
    ) -> Self {
        Self::Value(ValueConstraint {
            relation,
            source,
            target,
            value_origin,
            origin,
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

    /// Return the left operand, the source for directed relations.
    pub(in crate::check) fn left(&self) -> dir::GlobalTypeId {
        match self {
            Self::Type(constraint) => constraint.left,
            Self::Value(constraint) => constraint.source,
        }
    }

    /// Return the right operand, the target for directed relations.
    pub(in crate::check) fn right(&self) -> dir::GlobalTypeId {
        match self {
            Self::Type(constraint) => constraint.right,
            Self::Value(constraint) => constraint.target,
        }
    }

    /// Return the source that produced the relation.
    pub(in crate::check) fn origin(&self) -> Origin {
        match self {
            Self::Type(constraint) => constraint.origin,
            Self::Value(constraint) => constraint.origin,
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

/// Collected constraints with solver state.
#[derive(Debug)]
pub(in crate::check) struct ConstraintTable {
    /// The collected constraints keyed by absolute constraint id.
    constraints: IndexMap<ConstraintId, Constraint>,
    /// Constraint states keyed by absolute constraint id.
    states: IndexMap<ConstraintId, ConstraintState>,
}

impl ConstraintTable {
    /// Create an empty constraint table.
    pub(in crate::check) fn new() -> Self {
        Self {
            constraints: IndexMap::new(),
            states: IndexMap::new(),
        }
    }

    /// Insert one exact constraint id.
    pub(in crate::check) fn insert(&mut self, id: ConstraintId, constraint: Constraint) {
        self.constraints.insert(id, constraint);
        self.states.insert(id, ConstraintState::Pending);
    }

    /// Remove one exact constraint id.
    pub(in crate::check) fn remove(
        &mut self,
        id: ConstraintId,
    ) -> (Option<Constraint>, Option<ConstraintState>) {
        let constraint = self.constraints.swap_remove(&id);
        let state = self.states.swap_remove(&id);

        (constraint, state)
    }

    /// Return one constraint.
    pub(in crate::check) fn get(&self, id: ConstraintId) -> CompilerResult<&Constraint> {
        self.constraints
            .get(&id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} is not allocated"),
            })
    }

    /// Iterate over the collected constraints with their ids.
    pub(in crate::check) fn iter(&self) -> impl Iterator<Item = (ConstraintId, &Constraint)> {
        self.constraints
            .iter()
            .map(|(id, constraint)| (*id, constraint))
    }

    /// Return one constraint state.
    pub(in crate::check) fn state(&self, id: ConstraintId) -> CompilerResult<ConstraintState> {
        self.states
            .get(&id)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} has no solver state"),
            })
    }

    /// Set one constraint state.
    pub(in crate::check) fn set_state(&mut self, id: ConstraintId, state: ConstraintState) {
        self.states.insert(id, state);
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
