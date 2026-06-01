use crate::check::CheckState;

/// Derived size counters for one check component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CheckStats {
    /// The number of allocated solver variables.
    pub(in crate::check) variables: usize,
    /// The number of collected constraints.
    pub(in crate::check) constraints: usize,
    /// The number of collected obligations.
    pub(in crate::check) obligations: usize,
    /// The number of stored terms.
    pub(in crate::check) terms: usize,
    /// The number of solved variables.
    pub(in crate::check) solutions: usize,
    /// The total number of bounds.
    pub(in crate::check) bounds: usize,
    /// The number of solver decisions.
    pub(in crate::check) decisions: usize,
}

impl CheckStats {
    /// Render these stats as stable metadata lines.
    pub(in crate::check) fn render_metadata(self) -> String {
        format!(
            "\
check.stats.solve.variables={}
check.stats.solve.terms={}
check.stats.solve.constraints={}
check.stats.solve.obligations={}
check.stats.solve.solutions={}
check.stats.solve.bounds={}
check.stats.solve.decisions={}",
            self.variables,
            self.terms,
            self.constraints,
            self.obligations,
            self.solutions,
            self.bounds,
            self.decisions,
        )
    }
}

impl CheckState<'_> {
    /// Return derived size counters for this component.
    pub(in crate::check) fn stats(&self) -> CheckStats {
        let type_lower_bounds = self.inference.lower_type_bound_count();
        let type_upper_bounds = self.inference.upper_type_bound_count();
        let static_lower_bounds = self.inference.lower_static_bound_count();
        let static_upper_bounds = self.inference.upper_static_bound_count();
        let bounds =
            type_lower_bounds + type_upper_bounds + static_lower_bounds + static_upper_bounds;
        let decisions = self.inference.decision_count();

        CheckStats {
            variables: self.variable_count(),
            constraints: self.inference.constraint_count(),
            obligations: self.inference.obligation_count(),
            terms: self.inference.term_count_total(),
            solutions: self.inference.solution_count(),
            bounds,
            decisions,
        }
    }
}
