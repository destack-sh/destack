use destack_mir as mir;

/// One active borrow loan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Loan {
    /// The borrowed place.
    pub(super) place: mir::Place,
    /// Access granted by the borrow.
    pub(super) access: mir::Access,
    /// The reference value created by the borrow.
    pub(super) reference: mir::Value,
    /// MIR node for diagnostics.
    pub(super) created_at: mir::LocalNodeIdAny,
}

/// Active loans at one program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct LoanSet {
    /// The active loans.
    pub(super) loans: Vec<Loan>,
}

impl LoanSet {
    /// Return the active loan conflicting with a new loan.
    pub(super) fn conflict(
        &self,
        loan: &Loan,
        parent: Option<mir::Value>,
        mut may_overlap: impl FnMut(&mir::Place, &mir::Place) -> bool,
    ) -> Option<&Loan> {
        for active in &self.loans {
            if Some(active.reference) == parent {
                continue;
            }
            if !may_overlap(&active.place, &loan.place) {
                continue;
            }
            if !active.access.is_exclusive() && !loan.access.is_exclusive() {
                continue;
            }

            return Some(active);
        }

        None
    }

    /// Insert one loan when it is not already tracked.
    pub(super) fn insert(&mut self, loan: Loan) {
        if !self.loans.contains(&loan) {
            self.loans.push(loan);
        }
    }

    /// Return the loan blocking a place change.
    pub(super) fn blocking_change(
        &self,
        place: &mir::Place,
        mut may_overlap: impl FnMut(&mir::Place, &mir::Place) -> bool,
    ) -> Option<&Loan> {
        self.loans
            .iter()
            .find(|loan| may_overlap(&loan.place, place))
    }

    /// Return the exclusive loan blocking a write through one reference.
    pub(super) fn blocking_write(
        &self,
        place: &mir::Place,
        writer: Option<mir::Value>,
        mut may_overlap: impl FnMut(&mir::Place, &mir::Place) -> bool,
    ) -> Option<&Loan> {
        self.loans.iter().find(|loan| {
            loan.access.is_exclusive()
                && Some(loan.reference) != writer
                && may_overlap(&loan.place, place)
        })
    }

    /// Retain loans whose reference remains live.
    pub(super) fn retain_live(&mut self, mut is_live: impl FnMut(mir::Value) -> bool) {
        self.loans.retain(|loan| is_live(loan.reference));
    }

    /// Bind successor parameter loans.
    pub(super) fn bind(&mut self, argument: mir::Value, parameter: mir::Value) {
        for loan in &mut self.loans {
            if loan.reference == argument {
                loan.reference = parameter;
            }
            loan.place.replace_value(argument, parameter);
        }
    }
}
