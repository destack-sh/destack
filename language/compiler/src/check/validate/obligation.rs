use crate::check::{
    CheckError, CheckModuleState, CheckResult, Obligation, ObligationContext, ObligationKind,
    TypeRelation, VariableId, VariableKind,
};

impl CheckModuleState {
    /// Validate solved check obligations and emit diagnostics.
    pub(in crate::check) fn validate(&mut self) -> CheckResult<()> {
        for failure in self.take_failures() {
            self.report_failure(&failure)?;
        }

        for obligation in self.take_obligations() {
            self.validate_obligation(&obligation)?;
        }

        Ok(())
    }

    /// Validate one solved obligation.
    fn validate_obligation(&mut self, obligation: &Obligation) -> CheckResult<()> {
        if !obligation.guard.is_always() {
            return Ok(());
        }

        match &obligation.kind {
            ObligationKind::Type {
                relation,
                left,
                right,
            } => self.validate_type_obligation(*relation, *left, *right, &obligation.context),
            ObligationKind::Required { variable } => {
                self.validate_required_variable(*variable, &obligation.context)
            }
        }
    }

    /// Validate one solved type obligation.
    fn validate_type_obligation(
        &mut self,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
        context: &ObligationContext,
    ) -> CheckResult<()> {
        let left_type = self.variable_type_value(left);
        let right_type = self.variable_type_value(right);

        // require both sides to be solved before comparing
        let (Some(left_type), Some(right_type)) = (left_type, right_type) else {
            self.push_diagnostic(CheckError::CannotInferType {
                anchor: context.anchor.clone(),
                module: self.module(),
            });

            return Ok(());
        };

        // compare the relation modelled by this early scaffold
        if relation == TypeRelation::Equal && left_type != right_type {
            let diagnostic = self.type_relation_diagnostic(relation, context.anchor.clone());

            self.push_diagnostic(diagnostic);
        }

        Ok(())
    }

    /// Validate one required variable.
    fn validate_required_variable(
        &mut self,
        variable: VariableId,
        context: &ObligationContext,
    ) -> CheckResult<()> {
        if self.variable_value(variable).is_some() {
            return Ok(());
        }

        let diagnostic = match self.variable(variable).kind {
            VariableKind::Type => CheckError::CannotInferType {
                anchor: context.anchor.clone(),
                module: self.module(),
            },
            VariableKind::Static => CheckError::CannotEvaluateStatic {
                anchor: context.anchor.clone(),
                module: self.module(),
            },
            VariableKind::Layout => CheckError::CannotInferType {
                anchor: context.anchor.clone(),
                module: self.module(),
            },
            VariableKind::Resolution => CheckError::CannotInferType {
                anchor: context.anchor.clone(),
                module: self.module(),
            },
        };

        self.push_diagnostic(diagnostic);

        Ok(())
    }
}
