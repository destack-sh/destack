use cranelift_codegen::ir::InstBuilder;
use cranelift_module::Module;
use tspp_mir as mir;
use tspp_native as native;
use tspp_program::object::FramePoint;

use crate::{EmitError, ObjectEmitter};

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Emit one generated frame destructor call.
    pub(super) fn emit_drop(
        &mut self,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // select the destructor for the value type
        let ty = self.value_type(value)?;
        let Some(destructor) = self.optimized.drops.destructor(ty, mir::Storage::Frame) else {
            return Err(self.invalid("native drop has no frame destructor"));
        };

        // resolve the declared destructor and the value's canonical address
        let function = self
            .functions
            .get(&destructor)
            .copied()
            .ok_or_else(|| self.invalid("native frame destructor was not declared"))?;
        let function = self.output.declare_func_in_func(function, builder.func);
        let value_type = self.types.value(ty)?;
        let value = self.value(value)?;
        let address = self.materialize(value, value_type, builder)?;
        let reference = self.world_offset(address, builder)?;

        // pass one exclusive frame reference to the generated destructor
        let activation = self.activation()?;
        builder.ins().call(function, &[activation, reference]);

        Ok(())
    }

    /// Emit one unique allocation release, destroying its values first when the heap frees it now.
    pub(super) fn emit_release(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let owner = self.reference(value)?;

        // free an allocation whose values need no destruction
        let ty = self.value_type(value)?;
        let is_destroying = ObjectEmitter::release_destroys(self.module, self.optimized, ty)?;
        if !is_destroying {
            self.emit_runtime(native::abi::Operation::Free, &[owner], builder)?;

            return Ok(());
        }

        // retain this operation for deoptimization when its destructor is not native
        let point = self.object.instruction_point(instruction);
        let frame = self.stack_map(FramePoint::operation(point), builder)?;
        let id = self.frame_map_id(frame.id, builder)?;
        let call = self.emit_runtime(
            native::abi::Operation::Release,
            &[owner, id, frame.anchor],
            builder,
        )?;
        Self::attach_stack_map(frame.entries, call, builder);

        Ok(())
    }
}
