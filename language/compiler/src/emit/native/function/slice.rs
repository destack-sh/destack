use tspp_mir as mir;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl FunctionEmitter<'_> {
    /// Emit one slice descriptor length.
    pub(super) fn emit_slice_length(
        &mut self,
        destination: mir::Value,
        slice: mir::Value,
    ) -> Result<(), EmitError> {
        let Value::ScalarPair([_, length]) = self.value(slice)? else {
            return Err(self.invalid("native slice is not a scalar pair"));
        };
        self.set(destination, Value::Direct(length))?;

        Ok(())
    }
}
