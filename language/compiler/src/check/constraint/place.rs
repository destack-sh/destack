use crate::check::{CheckModuleState, Constraint, ConstraintOrigin, Place};

impl CheckModuleState {
    /// Require one place to accept a write.
    pub(in crate::check) fn require_writable_place(&mut self, place: Place) {
        let origin = ConstraintOrigin::Node(place.source);
        let constraint = Constraint::RequirePlaceWrite { place, origin };

        self.add_constraint(constraint);
    }
}
