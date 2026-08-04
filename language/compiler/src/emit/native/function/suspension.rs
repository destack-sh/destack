use cranelift_codegen::ir::InstBuilder;
use destack_mir as mir;
use destack_native as native;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl FunctionEmitter<'_> {
    /// Queue one waiter with a canonical typed value.
    pub(super) fn emit_waiter_queue(
        &mut self,
        destination: mir::Value,
        waiter: mir::Value,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let ty = self.value_type(value)?;
        let value_type = self.types.value(ty)?;
        let words = self.allocate_words(value_type.word_count(), builder);
        let value = self.value(value, builder)?;
        self.store_words(words, value, ty, value_type, builder)?;

        // call the scheduler with one linked Program type identity
        let ty = u32::try_from(ty.get())
            .map_err(|_| self.invalid("native waiter type identity exceeds u32"))?;
        let ty = self.index_u32(native::Index::Type { ty }, builder)?;
        let waiter = self.scalar(waiter, builder)?;
        let call = self.emit_runtime(
            native::abi::Operation::WaiterQueue,
            &[waiter, ty, words],
            builder,
        )?;
        let result = builder.inst_results(call)[0];
        let target = self
            .types
            .value(self.value_type(destination)?)?
            .direct()
            .ok_or_else(|| self.invalid("native waiter result is not scalar"))?;
        let result = builder.ins().ireduce(target, result);
        self.set(destination, Value::Direct(result), builder)?;

        Ok(())
    }

    /// Cancel one waiter through the runtime scheduler.
    pub(super) fn emit_waiter_cancel(
        &mut self,
        destination: mir::Value,
        waiter: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let waiter = self.scalar(waiter, builder)?;
        let call = self.emit_runtime(native::abi::Operation::WaiterCancel, &[waiter], builder)?;
        let result = builder.inst_results(call)[0];
        let target = self
            .types
            .value(self.value_type(destination)?)?
            .direct()
            .ok_or_else(|| self.invalid("native waiter result is not scalar"))?;
        let result = builder.ins().ireduce(target, result);
        self.set(destination, Value::Direct(result), builder)?;

        Ok(())
    }

    /// Create one already completed runtime task.
    pub(super) fn emit_task_resolve(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let ty = self.value_type(value)?;
        let value_type = self.types.value(ty)?;
        let words = self.allocate_words(value_type.word_count(), builder);
        let value = self.value(value, builder)?;
        self.store_words(words, value, ty, value_type, builder)?;

        // create the task through its linked Program value type
        let ty = u32::try_from(ty.get())
            .map_err(|_| self.invalid("native task type identity exceeds u32"))?;
        let ty = self.index_u32(native::Index::Type { ty }, builder)?;
        let call = self.emit_runtime(native::abi::Operation::TaskResolve, &[ty, words], builder)?;
        let task = builder.inst_results(call)[0];
        self.set(destination, Value::Direct(task), builder)?;

        Ok(())
    }

    /// Park one waiter until its task settles.
    pub(super) fn emit_task_park(
        &mut self,
        task: mir::Value,
        waiter: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let task = self.scalar(task, builder)?;
        let waiter = self.scalar(waiter, builder)?;
        self.emit_runtime(native::abi::Operation::TaskPark, &[task, waiter], builder)?;

        Ok(())
    }

    /// Execute one scalar task operation.
    pub(super) fn emit_task(
        &mut self,
        operation: native::abi::Operation,
        task: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let task = self.scalar(task, builder)?;
        self.emit_runtime(operation, &[task], builder)?;

        Ok(())
    }
}
