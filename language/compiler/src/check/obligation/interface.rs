use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, AutoInterface, CheckError, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Check whether one type satisfies one compiler-known auto interface.
    pub(in crate::check) fn check_auto_interface(
        &mut self,
        source: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
        interface: AutoInterface,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let ty = answer!(self.reduce_type_root(origin, ty)?);
        if answer!(self.satisfies_auto_interface(origin, ty, interface)?) {
            return Ok(Answer::Ready(None));
        }

        let (module, anchor) = self.source_anchor(source);
        let diagnostic = match interface {
            AutoInterface::DynamicSafe => {
                let ty = self.format_type(ty);
                let error = CheckError::DynamicSafetyNotSatisfied { anchor, module, ty };

                error.into()
            }
            AutoInterface::OverwriteStable => {
                let ty = self.format_type(ty);
                let error = CheckError::OverwriteStabilityNotSatisfied { anchor, module, ty };

                error.into()
            }
        };

        Ok(Answer::Ready(Some(diagnostic)))
    }
}
