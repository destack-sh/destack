use smallvec::SmallVec;

use destack_dir as dir;

use super::{CheckModuleState, StaticTerm, TypeTerm, VariableId, VariableOrigin};

/// One check constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Constraint {
    /// Define one type variable from one type term.
    ///
    /// ```ts
    /// value.name
    /// ```
    DefineType {
        /// The type variable being solved.
        result: VariableId,
        /// The type term assigned to it.
        term: TypeTerm,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
    /// Define one static variable from one static term.
    ///
    /// ```ts
    /// type Both = L | R;
    /// ```
    DefineStatic {
        /// The static variable being solved.
        result: VariableId,
        /// The static term assigned to it.
        term: StaticTerm,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
    /// Relate two type variables.
    ///
    /// ```ts
    /// const value: int32 = 1;
    /// ```
    RelateType {
        /// The required relation.
        relation: TypeRelation,
        /// The left type.
        left: VariableId,
        /// The right type.
        right: VariableId,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
    /// Relate two static variables.
    ///
    /// ```ts
    /// const size: 4 = value.length;
    /// ```
    RelateStatic {
        /// The required relation.
        relation: StaticRelation,
        /// The left static value.
        left: VariableId,
        /// The right static value.
        right: VariableId,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
    /// Require one place to accept a write.
    ///
    /// ```ts
    /// const value = 1;
    /// value = 2;
    /// ```
    RequirePlaceWrite {
        /// The place being written.
        place: Place,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
}

impl Constraint {
    /// Return variables whose changes should wake this constraint.
    pub(in crate::check) fn wake_variables(&self) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::DefineType {
                result,
                term,
                origin: _,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(term.referenced_variables());

                variables
            }
            Self::DefineStatic {
                result,
                term,
                origin: _,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(term.referenced_variables());

                variables
            }
            Self::RelateType {
                relation: _,
                left,
                right,
                origin: _,
            }
            | Self::RelateStatic {
                relation: _,
                left,
                right,
                origin: _,
            } => smallvec::smallvec![*left, *right],
            Self::RequirePlaceWrite { place, origin: _ } => place.referenced_variables(),
        }
    }
}

/// A relation between two type variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TypeRelation {
    /// Types must be equal.
    Equal,
    /// Source must be assignable to target.
    Assignable,
    /// Source must be explicitly castable to target.
    Castable,
    /// Value must satisfy a constraint.
    Satisfies,
    /// Subtype must extend supertype.
    Extends,
    /// Implementor must implement contract.
    Implements,
}

/// A relation between two static variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum StaticRelation {
    /// Static values must be equal.
    Equal,
}

/// Source location that produced one constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintOrigin {
    /// Constraint came from one source node.
    Node(dir::GlobalNodeIdAny),
    /// Constraint came from one source symbol.
    Symbol(dir::GlobalSymbolId),
    /// Constraint came from compiler induced work.
    Synthetic,
}

/// A (writable) place for storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct Place {
    /// The place value type variable.
    pub(in crate::check) ty: VariableId,
    /// How source syntax selected the place.
    pub(in crate::check) target: PlaceTarget,
    /// The source syntax node for diagnostics.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl Place {
    /// Create a place.
    pub(in crate::check) fn new(
        ty: VariableId,
        target: PlaceTarget,
        source: dir::GlobalNodeIdAny,
    ) -> Self {
        Self { ty, target, source }
    }

    /// Return variables that must be solved before this place can be checked.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        match self.target {
            PlaceTarget::Binding { symbol: _ } | PlaceTarget::Pattern { pattern: _ } => {
                smallvec::smallvec![self.ty]
            }
            PlaceTarget::Member { owner, key: _ } => smallvec::smallvec![self.ty, owner],
            PlaceTarget::Index { receiver, index } => {
                smallvec::smallvec![self.ty, receiver, index]
            }
            PlaceTarget::Dereference { output } => smallvec::smallvec![self.ty, output],
        }
    }
}

/// How source syntax selects a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum PlaceTarget {
    /// Local or imported value binding.
    Binding {
        /// The local binding symbol selected by syntax.
        symbol: dir::GlobalSymbolId,
    },
    /// Structural or nominal member target.
    Member {
        /// The receiver type.
        owner: VariableId,
        /// The selected member key.
        key: dir::StaticKey,
    },
    /// Protocol-backed index target.
    Index {
        /// The indexed receiver type.
        receiver: VariableId,
        /// The index expression type.
        index: VariableId,
    },
    /// Protocol-backed dereference target.
    Dereference {
        /// The dereference output type.
        output: VariableId,
    },
    /// Destructuring pattern target.
    Pattern {
        /// The assignment pattern node.
        pattern: dir::GlobalNodeIdAny,
    },
}

impl CheckModuleState {
    /// Add one constraint.
    pub(in crate::check) fn push_constraint(&mut self, constraint: Constraint) {
        self.work.constraints.push(constraint);
    }

    /// Define one type variable from one term.
    pub(in crate::check) fn define_type_term(&mut self, variable: VariableId, term: TypeTerm) {
        let origin = self.constraint_origin(variable);
        let constraint = Constraint::DefineType {
            result: variable,
            term,
            origin,
        };

        self.push_constraint(constraint);
    }

    /// Define one static variable from one term.
    pub(in crate::check) fn define_static_term(&mut self, variable: VariableId, term: StaticTerm) {
        let origin = self.constraint_origin(variable);
        let constraint = Constraint::DefineStatic {
            result: variable,
            term,
            origin,
        };

        self.push_constraint(constraint);
    }

    /// Add one type relation constraint.
    pub(in crate::check) fn push_type_relation(
        &mut self,
        origin: ConstraintOrigin,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
    ) {
        let constraint = Constraint::RelateType {
            relation,
            left,
            right,
            origin,
        };

        self.push_constraint(constraint);
    }

    /// Add one static relation constraint.
    pub(in crate::check) fn push_static_relation(
        &mut self,
        origin: ConstraintOrigin,
        relation: StaticRelation,
        left: VariableId,
        right: VariableId,
    ) {
        let constraint = Constraint::RelateStatic {
            relation,
            left,
            right,
            origin,
        };

        self.push_constraint(constraint);
    }

    /// Require one place to accept a write.
    pub(in crate::check) fn require_writable_place(&mut self, place: Place) {
        let origin = ConstraintOrigin::Node(place.source);
        let constraint = Constraint::RequirePlaceWrite { place, origin };

        self.push_constraint(constraint);
    }

    /// Return one diagnostic origin for a variable constraint.
    pub(in crate::check) fn constraint_origin(&self, variable: VariableId) -> ConstraintOrigin {
        match &self.variable(variable).origin {
            VariableOrigin::Node(node) => ConstraintOrigin::Node(*node),
            VariableOrigin::Symbol(symbol) => ConstraintOrigin::Symbol(*symbol),
            VariableOrigin::Generic(generic) => ConstraintOrigin::Symbol(generic.slot().owner),
            VariableOrigin::Generated { origin } => *origin,
        }
    }

    /// Return collected constraints.
    pub(in crate::check) fn constraints(&self) -> &[Constraint] {
        &self.work.constraints
    }
}
