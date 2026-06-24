use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, AutoInterface, CheckError, CheckState, Origin, RuntimePredicateObligation, answer,
};

impl CheckState<'_> {
    /// Check whether one selected runtime predicate is valid.
    pub(in crate::check) fn check_runtime_predicate(
        &mut self,
        obligation: &RuntimePredicateObligation,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        match &obligation.predicate {
            dir::GuardResolution::Is(predicate) => self.check_is_predicate(obligation, predicate),
            dir::GuardResolution::InstanceOf(predicate) => {
                self.check_instanceof_predicate(obligation, predicate)
            }
            dir::GuardResolution::In(predicate) => {
                self.check_in_predicate(obligation.source, predicate)
            }
        }
    }

    /// Check whether one `is` predicate can execute.
    fn check_is_predicate(
        &mut self,
        obligation: &RuntimePredicateObligation,
        predicate: &dir::IsGuardResolution,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        if let Some(error) = answer!(self.check_auto_interface(
            obligation.right,
            predicate.target_type,
            AutoInterface::DynamicSafe,
        )?) {
            return Ok(Answer::Ready(Some(error)));
        }

        let source = obligation.source;
        let origin = Origin::Node(source);
        if answer!(self.types_may_overlap(origin, predicate.value_type, predicate.target_type)?) {
            return Ok(Answer::Ready(None));
        }

        let module = obligation.left.module_id;
        let anchor = self.diagnostic_anchor(module, obligation.left.local_id);
        let source = self.format_type(predicate.value_type);
        let target = self.format_type(predicate.target_type);
        let error = CheckError::ImpossibleIs {
            anchor,
            module,
            source,
            target,
        };

        Ok(Answer::Ready(Some(error.into())))
    }

    /// Check whether one `instanceof` predicate can execute.
    fn check_instanceof_predicate(
        &mut self,
        obligation: &RuntimePredicateObligation,
        predicate: &dir::InstanceOfGuardResolution,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let source = obligation.source;
        let origin = Origin::Node(source);

        // reject predicates whose source type cannot overlap the class
        if answer!(self.types_may_overlap(origin, predicate.value_type, predicate.target_type)?) {
            return Ok(Answer::Ready(None));
        }

        let module = obligation.left.module_id;
        let anchor = self.diagnostic_anchor(module, obligation.left.local_id);
        let source = self.format_type(predicate.value_type);
        let target = self.format_symbol(predicate.target);
        let error = CheckError::ImpossibleInstanceOf {
            anchor,
            module,
            source,
            target,
        };

        Ok(Answer::Ready(Some(error.into())))
    }

    /// Check whether one `in` predicate can execute.
    fn check_in_predicate(
        &mut self,
        source: dir::GlobalNodeIdAny,
        predicate: &dir::InGuardResolution,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        if matches!(predicate.predicate.test, dir::PredicateTest::Call(_)) {
            return Ok(Answer::Ready(None));
        }

        let origin = Origin::Node(source);

        let is_key = answer!(self.is_property_key_type(origin, predicate.key_type)?);
        let is_receiver = answer!(self.is_keyed_type(origin, predicate.receiver_type)?);

        if is_key && is_receiver {
            return Ok(Answer::Ready(None));
        }

        let (module, anchor) = self.source_anchor(source);
        let key = self.format_type(predicate.key_type);
        let receiver = self.format_type(predicate.receiver_type);
        let error = CheckError::NoMatchingOperator {
            anchor,
            module,
            operator: "in".to_string(),
            operands: format!("'{key}' and '{receiver}'"),
        };

        Ok(Answer::Ready(Some(error.into())))
    }
}
