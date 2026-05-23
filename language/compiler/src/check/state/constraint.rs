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
    ///
    /// This defines the expression type as a member projection over `value`.
    TypeDefine {
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
    ///
    /// This defines the static lifetime value as a lifetime join.
    StaticDefine {
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
    TypeRelate {
        /// The required relation.
        relation: TypeRelation,
        /// The left type.
        left: VariableId,
        /// The right type.
        right: VariableId,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
    /// Check whether one assignment target accepts a write.
    ///
    /// ```ts
    /// const value = 1;
    /// value = 2;
    /// ```
    TargetWrite {
        /// The target being written.
        target: AssignmentTarget,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
}

impl Constraint {
    /// Return variables that must be solved before this constraint can finish.
    pub(in crate::check) fn input_variables(&self) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::TypeDefine {
                result,
                term,
                origin: _,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(term.referenced_variables());

                variables
            }
            Self::StaticDefine {
                result,
                term,
                origin: _,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(term.referenced_variables());

                variables
            }
            Self::TypeRelate {
                relation: _,
                left,
                right,
                origin: _,
            } => smallvec::smallvec![*left, *right],
            Self::TargetWrite { target, origin: _ } => smallvec::smallvec![target.ty],
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

/// A value that can appear on the left side of an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct AssignmentTarget {
    /// The target type variable.
    pub(in crate::check) ty: VariableId,
    /// The selected target key.
    pub(in crate::check) key: AssignmentTargetKey,
    /// The assignment syntax node for diagnostics.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl AssignmentTarget {
    /// Create an assignment target.
    pub(in crate::check) fn new(
        ty: VariableId,
        key: AssignmentTargetKey,
        source: dir::GlobalNodeIdAny,
    ) -> Self {
        Self { ty, key, source }
    }
}

/// The selected assignment target key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum AssignmentTargetKey {
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
    /// Destructuring or invalid target.
    Unknown,
}

impl CheckModuleState {
    /// Add one constraint.
    pub(in crate::check) fn push_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Define one type variable from one term.
    pub(in crate::check) fn define_type_term(&mut self, variable: VariableId, term: TypeTerm) {
        let origin = self.constraint_origin(variable);
        let constraint = Constraint::TypeDefine {
            result: variable,
            term,
            origin,
        };

        self.push_constraint(constraint);
    }

    /// Define one static variable from one term.
    pub(in crate::check) fn define_static_term(&mut self, variable: VariableId, term: StaticTerm) {
        let origin = self.constraint_origin(variable);
        let constraint = Constraint::StaticDefine {
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
        let constraint = Constraint::TypeRelate {
            relation,
            left,
            right,
            origin,
        };

        self.push_constraint(constraint);
    }

    /// Add one target write constraint.
    pub(in crate::check) fn push_target_write(&mut self, target: AssignmentTarget) {
        let origin = ConstraintOrigin::Node(target.source);
        let constraint = Constraint::TargetWrite { target, origin };

        self.push_constraint(constraint);
    }

    /// Return one diagnostic origin for a variable constraint.
    pub(in crate::check) fn constraint_origin(&self, variable: VariableId) -> ConstraintOrigin {
        match &self.variable(variable).origin {
            VariableOrigin::Node(node) => ConstraintOrigin::Node(*node),
            VariableOrigin::Symbol(symbol) => ConstraintOrigin::Symbol(*symbol),
            VariableOrigin::Generic(generic) => ConstraintOrigin::Symbol(generic.slot().owner),
            VariableOrigin::Synthetic => ConstraintOrigin::Synthetic,
        }
    }

    /// Return collected constraints.
    pub(in crate::check) fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }
}
