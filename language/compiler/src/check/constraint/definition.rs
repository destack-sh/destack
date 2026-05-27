use smallvec::SmallVec;

use destack_source::ModuleId;

use crate::check::{
    CheckState, Constraint, ConstraintOrigin, StaticCondition, StaticTerm, TermId, TermTable,
    TypeTerm, VariableId,
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
        origin: ConstraintOrigin,
        /// The static condition under which this definition exists.
        condition: StaticCondition,
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
        origin: ConstraintOrigin,
        /// The static condition under which this definition exists.
        condition: StaticCondition,
    },
}

impl Definition {
    /// Return variables whose changes should wake this definition.
    pub(in crate::check) fn wake_variables(
        &self,
        terms: &TermTable,
    ) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Type {
                result,
                term,
                origin: _,
                condition,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(terms.get(*term).referenced_variables());
                variables.extend(condition.referenced_variables());

                variables
            }
            Self::Static {
                result,
                term,
                origin: _,
                condition,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(terms.get(*term).referenced_variables());
                variables.extend(condition.referenced_variables());

                variables
            }
        }
    }
}

impl CheckState<'_> {
    /// Add one definition.
    pub(in crate::check) fn add_definition(&mut self, definition: Definition) {
        self.variables.definitions.push(definition);
    }

    /// Add one constraint.
    pub(in crate::check) fn add_constraint(&mut self, constraint: Constraint) {
        self.variables.constraints.push(constraint);
    }

    /// Define one type variable from one term.
    pub(in crate::check) fn define_type(
        &mut self,
        module: ModuleId,
        variable: VariableId,
        term: TypeTerm,
    ) {
        let origin = self.variable(variable).source;
        let term = self.intern_term(term);
        let condition = self.flow(module).current_static_condition();
        let definition = Definition::Type {
            result: variable,
            term,
            origin,
            condition,
        };

        self.add_definition(definition);
    }

    /// Define one static variable from one term.
    pub(in crate::check) fn define_static(
        &mut self,
        module: ModuleId,
        variable: VariableId,
        term: StaticTerm,
    ) {
        let origin = self.variable(variable).source;
        let term = self.intern_term(term);
        let condition = self.flow(module).current_static_condition();
        let definition = Definition::Static {
            result: variable,
            term,
            origin,
            condition,
        };

        self.add_definition(definition);
    }
}
