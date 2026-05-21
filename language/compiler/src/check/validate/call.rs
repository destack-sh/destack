use crate::check::{
    CheckError, CheckModuleState, CheckResult, ObligationContext, ObligationSource, TypeInferId,
};

impl CheckModuleState {
    /// Validate that one call selected a callable target.
    pub(in crate::check) fn validate_callable(
        &mut self,
        callee: TypeInferId,
        arguments: &[TypeInferId],
        context: &ObligationContext,
    ) -> CheckResult<()> {
        let ObligationSource::Call { call } = &context.source else {
            return Ok(());
        };

        if self.resolutions().call_resolution(*call).is_some() {
            return Ok(());
        }

        let Some(callee_type) = self.infer_type_id(callee) else {
            return Ok(());
        };

        let candidates = self.resolve_call_candidates(callee_type, None);
        if candidates.is_empty() {
            self.push_diagnostic(CheckError::NotCallable {
                anchor: context.anchor.clone(),
                module: self.module(),
            });

            return Ok(());
        }

        let mut argument_types = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let Some(argument_type) = self.infer_type_id(*argument) else {
                return Ok(());
            };
            argument_types.push(argument_type);
        }

        if self
            .select_call_candidate(&candidates, &argument_types)
            .is_none()
        {
            self.push_diagnostic(CheckError::NoMatchingCall {
                anchor: context.anchor.clone(),
                module: self.module(),
            });
        }

        Ok(())
    }
}
