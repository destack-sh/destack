use crate::check::{CheckModuleState, Constraint, StaticTerm, TypeTerm, VariableId};

impl CheckModuleState {
    /// Add one constraint.
    pub(in crate::check) fn add_constraint(&mut self, constraint: Constraint) {
        self.work.constraints.push(constraint);
    }

    /// Define one type variable from one term.
    pub(in crate::check) fn define_type_term(&mut self, variable: VariableId, term: TypeTerm) {
        let origin = self.variable(variable).source;
        let constraint = Constraint::DefineType {
            result: variable,
            term,
            origin,
        };

        self.add_constraint(constraint);
    }

    /// Define one static variable from one term.
    pub(in crate::check) fn define_static_term(&mut self, variable: VariableId, term: StaticTerm) {
        let origin = self.variable(variable).source;
        let constraint = Constraint::DefineStatic {
            result: variable,
            term,
            origin,
        };

        self.add_constraint(constraint);
    }
}
