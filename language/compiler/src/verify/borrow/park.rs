use destack_mir::{Function, LoanId, LocalNodeId, PlaceOrigin, Point, ReferenceKind, Storage};

use super::checker::BorrowChecker;

impl BorrowChecker<'_, '_> {
    /// Pin the local managed storage borrowed across one parking call.
    pub(super) fn pin_park(&mut self, callsite: Point, direct: Option<LocalNodeId<Function>>) {
        let target = self.resolved_target(callsite, direct);
        if !self.call_may_park(callsite, target) {
            return;
        }

        // pin every managed loan live at the call through its borrowed reference
        let values = self
            .active_loans
            .iter()
            .copied()
            .filter(|loan| self.may_move(*loan))
            .map(|loan| self.origin.loans().get(loan).representation)
            .collect::<Vec<_>>();

        // record the call as a safepoint once it pins something
        if !values.is_empty() {
            self.safepoints.insert(callsite, values);
        }
    }

    /// Return whether one active loan borrows local managed storage.
    pub(super) fn holds_local_managed_loan(&self) -> bool {
        self.active_loans.iter().any(|loan| self.may_move(*loan))
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

    /// Return whether the collector may move the storage one loan borrows.
    fn may_move(&self, loan: LoanId) -> bool {
        let loan = self.origin.loans().get(loan);
        let Some(place) = loan.place() else {
            return false;
        };
        let PlaceOrigin::Value(value) = place.origin else {
            return false;
        };
        let ty = self.function.expect_value_type(value);
        let definition = self.tree.get(ty);

        // treat a managed reference without declared storage as local
        definition.reference_kind() == Some(ReferenceKind::Managed)
            && matches!(
                definition.reference_storage(),
                None | Some(Storage::LocalHeap)
            )
    }
}
