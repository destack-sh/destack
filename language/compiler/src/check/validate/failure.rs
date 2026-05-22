use crate::check::{CheckError, CheckFailure, CheckModuleState, CheckResult};

impl CheckModuleState {
    /// Report one check failure.
    pub(in crate::check) fn report_failure(&mut self, failure: &CheckFailure) -> CheckResult<()> {
        match failure {
            CheckFailure::VariableConflict { variable } => {
                let anchor = self.anchor_node(self.variable_source_node(*variable));

                self.push_diagnostic(CheckError::CannotInferType {
                    anchor,
                    module: self.module(),
                });
            }
            CheckFailure::StaticEvaluation { node } => {
                self.push_diagnostic(CheckError::CannotEvaluateStatic {
                    anchor: self.anchor_global_node(*node),
                    module: self.module(),
                });
            }
            CheckFailure::TypeEvaluation { node } => {
                self.push_diagnostic(CheckError::CannotInferType {
                    anchor: self.anchor_global_node(*node),
                    module: self.module(),
                });
            }
            CheckFailure::TypeRelation { relation, left, .. } => {
                let anchor = self.anchor_node(self.variable_source_node(*left));
                let diagnostic = self.type_relation_diagnostic(*relation, anchor);

                self.push_diagnostic(diagnostic);
            }
        }

        Ok(())
    }
}
