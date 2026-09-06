use destack_mir::{
    Function, LoanId, LocalNodeId, PlaceOrigin, Point, ReferenceKind, SafepointKind, Value,
};

use destack_core::FxIndexSet;

use super::checker::FunctionChecker;

impl FunctionChecker<'_, '_> {
    /// Hold the handles of the managed storage borrowed across one parking call.
    pub(super) fn hold_park(&mut self, callsite: Point, direct: Option<LocalNodeId<Function>>) {
        // act only on calls that may park
        let target = self.resolved_target(callsite, direct);
        if !self.call_may_park(callsite, target) {
            return;
        }

        // record the call as a safepoint once it holds something
        let handles = self.live_handles();
        if !handles.is_empty() {
            self.safepoints
                .insert(callsite, SafepointKind::Park, handles);
        }
    }

    /// Return the handles the live managed loans were taken through, the owners that outlive them.
    pub(super) fn live_handles(&self) -> Vec<Value> {
        let mut handles = FxIndexSet::default();
        for loan in self.active_loans.iter().copied() {
            if let Some(handle) = self.managed_handle(loan) {
                handles.insert(handle);
            }
        }

        handles.into_iter().collect()
    }

    /// Return whether one call may park the current fiber.
    fn call_may_park(&self, callsite: Point, target: Option<LocalNodeId<Function>>) -> bool {
        if let Some(call) = self.verification.effects.call(callsite)
            && call.behavior.park.may_park()
        {
            return true;
        }

        target
            .and_then(|target| self.verification.effects.function(target))
            .is_some_and(|effect| effect.behavior.park.may_park())
    }

    /// Return the managed handle one loan borrows through, absent for other storage.
    fn managed_handle(&self, loan: LoanId) -> Option<Value> {
        // resolve the value the loan borrows from
        let loan = self.origin.loans().get(loan);
        let place = loan.place()?;
        let PlaceOrigin::Value(value) = place.origin else {
            return None;
        };
        let ty = self.function.expect_value_type(value);
        let definition = self.tree.get(ty);

        // a borrow through a managed handle into heap storage
        (definition.reference_kind() == Some(ReferenceKind::Managed)
            && definition
                .reference_storage()
                .is_none_or(|storage| storage.heap_space().is_some()))
        .then_some(value)
    }
}
