use destack_mir as mir;

use super::borrow::BorrowMap;
use super::loan::LoanSet;
use super::r#move::MoveSet;

/// Ownership state at one control-flow point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct FlowState {
    /// Moved places.
    pub(super) moves: MoveSet,
    /// Active loans.
    pub(super) loans: LoanSet,
    /// Borrow sources.
    pub(super) borrows: BorrowMap,
    /// Borrow references already reported as invalid.
    pub(super) invalid_borrows: Vec<mir::Value>,
}

impl FlowState {
    /// Create an empty flow state.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Merge predecessor flow states.
    pub(super) fn merge_predecessors(predecessors: &[FlowState]) -> Self {
        if predecessors.is_empty() {
            return Self::new();
        }

        let mut borrows = BorrowMap::default();
        let mut loans = LoanSet::default();
        let mut invalid_borrows = Vec::new();

        // merge borrow sources conservatively across incoming edges
        for predecessor in predecessors {
            for binding in &predecessor.borrows.bindings {
                borrows.merge_sources_at(binding.value, &binding.path, &binding.sources);
            }
        }

        // keep active loans carried by any incoming edge
        for predecessor in predecessors {
            for loan in &predecessor.loans.loans {
                if !loans.loans.contains(loan) {
                    loans.loans.push(loan.clone());
                }
            }
        }

        // keep invalid references carried by any incoming edge
        for predecessor in predecessors {
            for value in &predecessor.invalid_borrows {
                if !invalid_borrows.contains(value) {
                    invalid_borrows.push(*value);
                }
            }
        }

        Self {
            moves: MoveSet::merge_predecessors(predecessors),
            loans,
            borrows,
            invalid_borrows,
        }
    }

    /// Bind successor parameters to predecessor arguments.
    pub(super) fn bind(&mut self, argument: mir::Value, parameter: mir::Value) {
        self.moves.bind(argument, parameter);
        self.loans.bind(argument, parameter);
        self.borrows.bind(argument, parameter);

        for value in &mut self.invalid_borrows {
            if *value == argument {
                *value = parameter;
            }
        }
    }

    /// Return whether one borrow reference is invalid.
    pub(super) fn is_invalid_borrow(&self, value: mir::Value) -> bool {
        self.invalid_borrows.contains(&value)
    }

    /// Mark one borrow reference invalid.
    pub(super) fn invalidate_borrow(&mut self, value: mir::Value) {
        if !self.invalid_borrows.contains(&value) {
            self.invalid_borrows.push(value);
        }
    }
}
