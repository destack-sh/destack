use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    CheckState, ObligationCheck, ObligationFailure, Origin, RuntimePredicateObligation,
};

impl CheckState<'_> {
    /// Check whether one selected runtime predicate is valid.
    pub(in crate::sema) fn check_runtime_predicate(
        &mut self,
        origin: Origin,
        obligation: &RuntimePredicateObligation,
    ) -> CompilerResult<ObligationCheck> {
        match &obligation.predicate {
            dir::GuardDecision::Is(predicate) => {
                self.check_is_predicate(origin, obligation, predicate)
            }
            dir::GuardDecision::InstanceOf(predicate) => {
                self.check_instanceof_predicate(origin, obligation, predicate)
            }
            dir::GuardDecision::In(predicate) => {
                self.check_in_predicate(origin, obligation.source, predicate)
            }
        }
    }

    /// Check whether one `is` predicate can execute.
    fn check_is_predicate(
        &mut self,
        origin: Origin,
        obligation: &RuntimePredicateObligation,
        predicate: &dir::IsGuardDecision,
    ) -> CompilerResult<ObligationCheck> {
        let anchored = self.origin_at(origin, obligation.right)?;
        match self.check_auto_interface(
            anchored,
            predicate.target_type,
            dir::AutoInterface::DynamicSafe,
        )? {
            ObligationCheck::Holds => {}
            ObligationCheck::Fails(failures) => {
                return Ok(ObligationCheck::Fails(failures));
            }
            check @ ObligationCheck::Ambiguous(_) => {
                return Ok(check);
            }
        }

        if self.types_may_overlap(origin, predicate.value_type, predicate.target_type)? {
            return Ok(ObligationCheck::holds());
        }

        let failure = ObligationFailure::ImpossibleIs {
            source: obligation.left,
            value: predicate.value_type,
            target: predicate.target_type,
        };

        Ok(ObligationCheck::fail(failure))
    }

    /// Check whether one `instanceof` predicate can execute.
    fn check_instanceof_predicate(
        &mut self,
        origin: Origin,
        obligation: &RuntimePredicateObligation,
        predicate: &dir::InstanceOfGuardDecision,
    ) -> CompilerResult<ObligationCheck> {
        // reject predicates whose source type cannot overlap the class
        if self.types_may_overlap(origin, predicate.value_type, predicate.target_type)? {
            return Ok(ObligationCheck::holds());
        }

        let failure = ObligationFailure::ImpossibleInstanceOf {
            source: obligation.left,
            value: predicate.value_type,
            target: predicate.target,
        };

        Ok(ObligationCheck::fail(failure))
    }

    /// Check whether one `in` predicate can execute.
    fn check_in_predicate(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        predicate: &dir::InGuardDecision,
    ) -> CompilerResult<ObligationCheck> {
        let is_key = self.is_property_key_type(predicate.key_type)?;
        let is_receiver = self.is_keyed_type(origin, predicate.receiver_type)?;

        if is_key && is_receiver {
            return Ok(ObligationCheck::holds());
        }

        let failure = ObligationFailure::InvalidInPredicate {
            source,
            key: predicate.key_type,
            receiver: predicate.receiver_type,
        };

        Ok(ObligationCheck::fail(failure))
    }
}
