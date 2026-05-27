use destack_dir as dir;

use crate::check::{CheckState, Decision, VariableId};
use crate::{CheckError, CompilerResult};

impl CheckState<'_> {
    /// Check whether one try expression can propagate through its return type.
    pub(in crate::check) fn check_try_propagates(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: VariableId,
        return_type: Option<VariableId>,
    ) -> CompilerResult<()> {
        let Some(return_type) = return_type else {
            let (module, anchor) = self.source_anchor(source)?;
            let diagnostic = CheckError::InvalidControlFlow {
                anchor,
                module,
                message: "? can only propagate from a function body".to_owned(),
            };

            self.diagnostics_mut(source.module_id).push(diagnostic);

            return Ok(());
        };
        let decision = self.decide_try_propagation(source, value, return_type)?;

        // reject incompatible failure propagation
        if decision == Decision::No {
            let (module, anchor) = self.source_anchor(source)?;
            let diagnostic = CheckError::DoesNotImplement { anchor, module };

            self.diagnostics_mut(source.module_id).push(diagnostic);
        }

        // require more solved type information
        if decision == Decision::Undecidable {
            let (module, anchor) = self.source_anchor(source)?;
            let diagnostic = CheckError::CannotSolve { anchor, module };

            self.diagnostics_mut(source.module_id).push(diagnostic);
        }

        Ok(())
    }
}
