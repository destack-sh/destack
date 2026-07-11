use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, ObligationCheck, ObligationFailure, Origin, RuntimePredicateObligation,
    answer,
};

impl CheckState<'_> {
    /// Check whether one selected runtime predicate is valid.
    pub(in crate::check) fn check_runtime_predicate(
        &mut self,
        origin: Origin,
        obligation: &RuntimePredicateObligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        match &obligation.predicate {
            dir::GuardResolution::Is(predicate) => {
                self.check_is_predicate(origin, obligation, predicate)
            }
            dir::GuardResolution::InstanceOf(predicate) => {
                self.check_instanceof_predicate(origin, obligation, predicate)
            }
            dir::GuardResolution::In(predicate) => {
                self.check_in_predicate(origin, obligation.source, predicate)
            }
        }
    }

    /// Check whether one `is` predicate can execute.
    fn check_is_predicate(
        &mut self,
        origin: Origin,
        obligation: &RuntimePredicateObligation,
        predicate: &dir::IsGuardResolution,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let anchored = self.origin_at(origin, obligation.right)?;
        match answer!(self.check_auto_interface(
            anchored,
            predicate.target_type,
            dir::AutoInterface::DynamicSafe,
        )?) {
            ObligationCheck::Holds => {}
            ObligationCheck::Fails(failures) => {
                return Ok(Answer::Ready(ObligationCheck::Fails(failures)));
            }
        }

        if answer!(self.types_may_overlap(origin, predicate.value_type, predicate.target_type)?) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let failure = ObligationFailure::ImpossibleIs {
            source: obligation.left,
            value: predicate.value_type,
            target: predicate.target_type,
        };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }

    /// Check whether one `instanceof` predicate can execute.
    fn check_instanceof_predicate(
        &mut self,
        origin: Origin,
        obligation: &RuntimePredicateObligation,
        predicate: &dir::InstanceOfGuardResolution,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        // reject predicates whose source type cannot overlap the class
        if answer!(self.types_may_overlap(origin, predicate.value_type, predicate.target_type)?) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let failure = ObligationFailure::ImpossibleInstanceOf {
            source: obligation.left,
            value: predicate.value_type,
            target: predicate.target,
        };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }

    /// Check whether one `in` predicate can execute.
    fn check_in_predicate(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        predicate: &dir::InGuardResolution,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let is_key = answer!(self.is_property_key_type(origin, predicate.key_type)?);
        let is_receiver = answer!(self.is_keyed_type(origin, predicate.receiver_type)?);

        if is_key && is_receiver {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let failure = ObligationFailure::InvalidInPredicate {
            source,
            key: predicate.key_type,
            receiver: predicate.receiver_type,
        };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }
}
