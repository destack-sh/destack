use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, ObligationCheck, ObligationFailure, Origin, answer};

impl CheckState<'_> {
    /// Check whether one type satisfies one compiler-known auto interface.
    pub(in crate::check) fn check_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        // hold without checking once an operand already reported an error
        if self.any_error_operand(&[ty])? {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }
        if answer!(self.satisfies_auto_interface(origin, ty, interface)?) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let source = self.origin_source(origin)?;
        let failure = ObligationFailure::AutoInterfaceNotSatisfied {
            source,
            ty,
            interface,
        };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }
}
