use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one slice length projection.
    pub(super) fn emit_slice_length(
        &mut self,
        destination: mir::Value,
        slice: mir::Value,
    ) -> Result<(), EmitError> {
        let slice = self.register(slice)?;
        let length = bytecode::RegisterSpan::new(bytecode::RegisterId(slice.start.0 + 1), 1);
        let destination_type = self.register_type(destination)?;
        let destination = self.register(destination)?;

        self.emit_move(length, destination, destination_type)
    }
}
