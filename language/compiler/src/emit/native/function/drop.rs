use cranelift_codegen::ir::InstBuilder;
use cranelift_module::Module;
use destack_mir as mir;
use destack_native as native;
use destack_program::object::FramePoint;

use crate::EmitError;

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Emit one generated frame destructor call.
    pub(super) fn emit_drop(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // select the destructor for the value type
        let ty = self.value_type(value)?;
        if let Some(destructor) = self.optimized.drops.destructor(ty, mir::Storage::Frame) {
            let function = self
                .functions
                .get(&destructor)
                .copied()
                .ok_or_else(|| self.invalid("native frame destructor was not declared"))?;
            let function = self.output.declare_func_in_func(function, builder.func);
            let value_type = self.types.value(ty)?;
            let value = self.value(value)?;
            let address = self.materialize(value, value_type, builder)?;

            // pass one exclusive frame reference to the generated destructor
            let activation = self.activation()?;
            builder.ins().call(function, &[activation, address]);

            return Ok(());
        }

        // require one erased unique representation for allocation-selected destruction
        let storage_type = self.optimized.tree.storage_type(ty);
        if !matches!(
            self.optimized.tree.get(storage_type),
            mir::Type::Dynamic {
                kind: mir::ReferenceKind::Unique,
                ..
            } | mir::Type::Function {
                kind: mir::ReferenceKind::Unique,
                ..
            }
        ) {
            return Err(self.invalid("native drop has no destructor"));
        }

        // retain this operation for deoptimization when its destructor is not native
        let point = self.object.instruction_point(instruction);
        let frame = self.stack_map(FramePoint::operation(point), builder)?;
        let id = self.frame_map_id(frame.id, builder)?;
        let owner = self.reference(value, builder)?;
        let call = self.emit_runtime(
            native::abi::Operation::Drop,
            &[owner, id, frame.anchor],
            builder,
        )?;
        Self::attach_stack_map(frame.entries, call, builder);

        Ok(())
    }
}
