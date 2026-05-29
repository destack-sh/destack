use smallvec::SmallVec;

use crate::check::{
    CheckState, Condition, Constraint, Origin, StaticTerm, TermId, TypeTerm, VariableId,
};

/// One check variable definition.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Definition {
    /// Define one type variable from one type term.
    ///
    /// ```ts
    /// value.name
    /// ```
    Type {
        /// The type variable being solved.
        result: VariableId,
        /// The type term assigned to it.
        term: TermId<TypeTerm>,
        /// The source that produced this definition.
        origin: Origin,
        /// The static condition under which this definition exists.
        condition: Condition,
    },
    /// Define one static variable from one static term.
    ///
    /// ```ts
    /// type Both = L | R;
    /// ```
    Static {
        /// The static variable being solved.
        result: VariableId,
        /// The static term assigned to it.
        term: TermId<StaticTerm>,
        /// The source that produced this definition.
        origin: Origin,
        /// The static condition under which this definition exists.
        condition: Condition,
    },
}

impl Definition {
    /// Return the variable defined by this definition.
    pub(in crate::check) fn result(&self) -> VariableId {
        match self {
            Self::Type { result, .. } | Self::Static { result, .. } => *result,
        }
    }

    /// Return variables watched by this definition.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Type {
                result,
                term,
                origin: _,
                condition,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(state.terms.get(*term).referenced_variables(state));
                variables.extend(condition.referenced_variables(state));

                variables
            }
            Self::Static {
                result,
                term,
                origin: _,
                condition,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(state.terms.get(*term).referenced_variables(state));
                variables.extend(condition.referenced_variables(state));

                variables
            }
        }
    }
}

impl CheckState<'_> {
    /// Add one definition.
    pub(in crate::check) fn add_definition(&mut self, definition: Definition) {
        self.variables.defined.insert(definition.result());
        self.variables.definitions.push(definition);
    }

    /// Add one constraint.
    pub(in crate::check) fn add_constraint(&mut self, constraint: Constraint) {
        self.variables.constraints.push(constraint);
    }

    /// Add one type variable definition under one static condition.
    pub(in crate::check) fn add_type_definition(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
        condition: Condition,
    ) {
        let origin = self.variable(variable).source;
        let term = self.terms.push(term);
        let definition = Definition::Type {
            result: variable,
            term,
            origin,
            condition,
        };

        self.add_definition(definition);
    }

    /// Add one static variable definition under one static condition.
    pub(in crate::check) fn add_static_definition(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
        condition: Condition,
    ) {
        let origin = self.variable(variable).source;
        let term = self.terms.push(term);
        let definition = Definition::Static {
            result: variable,
            term,
            origin,
            condition,
        };

        self.add_definition(definition);
    }
}
