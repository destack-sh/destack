use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one call instruction.
    pub(super) fn emit_call(
        &mut self,
        destination: Option<mir::Value>,
        call: &mir::Call,
    ) -> Result<(), EmitError> {
        let destinations = destination
            .map(|value| self.register(value))
            .transpose()?
            .into_iter()
            .collect::<Vec<_>>();

        let opcode = Self::call_opcode(false, &call.callee);

        self.emit_call_operation(call, opcode, &destinations, None)
    }

    /// Emit one invoked call with normal and unwind continuations.
    pub(super) fn emit_invoke(
        &mut self,
        terminator: &mir::Terminator,
        call: &mir::Call,
        target: &mir::BlockTarget,
        unwind: &mir::BlockTarget,
    ) -> Result<(), EmitError> {
        // read the result type from the call signature
        let result = self
            .optimized
            .tree
            .get(call.signature)
            .function_signature_parts()
            .map(|(_, _, result)| result)
            .ok_or_else(|| self.internal("call has no callable signature"))?;

        // select registers for the returned value
        let destinations = if matches!(self.optimized.tree.get(result), mir::Type::Void) {
            Vec::new()
        } else {
            self.successor_destinations(terminator, mir::Successor::InvokeNormal, target)?
        };

        // resolve the normal and unwind continuation labels
        let target = self.edge_label(terminator, mir::Successor::InvokeNormal, target)?;
        let unwind = self.edge_label(terminator, mir::Successor::InvokeUnwind, unwind)?;

        // select the invoke opcode for the callee
        let opcode = Self::call_opcode(true, &call.callee);

        self.emit_call_operation(call, opcode, &destinations, Some((target, unwind)))
    }

    /// Emit one terminal call without retaining a caller frame.
    pub(super) fn emit_tail_call(&mut self, call: &mir::Call) -> Result<(), EmitError> {
        let opcode = match call.callee {
            mir::Callee::Direct { .. } => bytecode::Opcode::TAIL_CALL,
            mir::Callee::Indirect { .. } => bytecode::Opcode::TAIL_CALL_INDIRECT,
            mir::Callee::Virtual { .. } => bytecode::Opcode::TAIL_CALL_VIRTUAL,
            mir::Callee::Dynamic { .. } => bytecode::Opcode::TAIL_CALL_DYNAMIC,
            mir::Callee::Witness { .. } => {
                return Err(self.internal("witness calls resolve at instantiation"));
            }
        };

        self.emit_call_operation(call, opcode, &[], None)
    }

    /// Emit one call family operation in physical operand order.
    fn emit_call_operation(
        &mut self,
        call: &mir::Call,
        opcode: bytecode::Opcode,
        destinations: &[bytecode::RegisterSpan],
        branches: Option<(bytecode::Label, bytecode::Label)>,
    ) -> Result<(), EmitError> {
        // move the call arguments into their registers
        let arguments = self.optimized.tree.get_values(call.arguments);
        let arguments = self.emit_arguments(arguments)?;

        // anchor the logical Program point to the call after argument moves
        self.builder
            .anchor_operation()
            .map_err(|error| self.bytecode_error(error))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        self.emit_callee(&call.callee, &mut instruction)?;
        instruction.span(arguments);

        // append explicit control edges only for invoked calls
        if let Some((target, unwind)) = branches {
            instruction.branch(target);
            instruction.branch(unwind);
        }

        self.encode(instruction, destinations)
    }

    /// Encode one direct or dispatched callee.
    fn emit_callee(
        &self,
        callee: &mir::Callee,
        instruction: &mut bytecode::InstructionBuilder,
    ) -> Result<(), EmitError> {
        match callee {
            mir::Callee::Witness { .. } => {
                return Err(self.internal("witness calls resolve at instantiation"));
            }
            mir::Callee::Direct { function, .. } => {
                let function = self.types.function_id(*function)?;
                instruction.relocation(bytecode::RelocationTag::FUNCTION, function.0);
            }
            mir::Callee::Indirect { value } => {
                instruction.span(self.register(*value)?);
            }
            mir::Callee::Virtual {
                receiver,
                class,
                slot,
            } => {
                let reference = self
                    .register_type(*receiver)?
                    .reference_type()
                    .ok_or_else(|| self.internal("virtual receiver is not a reference"))?;
                let layout = self
                    .optimized
                    .layouts
                    .type_layout(*class)
                    .ok_or_else(|| self.internal("virtual class has no layout"))?;
                let mir::LayoutShape::Object(layout) = &layout.shape else {
                    return Err(self.internal("virtual class has no object layout"));
                };
                let dispatch_offset = layout
                    .dispatch_offset
                    .ok_or_else(|| self.internal("virtual class has no dispatch field"))?;
                let slot = u16::try_from(slot.0)
                    .map_err(|_| self.internal("virtual dispatch slot exceeds bytecode"))?;

                instruction.register(self.word(*receiver)?);
                instruction.reference(reference.kind(), reference.storage());
                instruction.u32(dispatch_offset);
                instruction.u16(slot);
            }
            mir::Callee::Dynamic { receiver, slot, .. } => {
                let slot = u16::try_from(slot.0)
                    .map_err(|_| self.internal("dynamic dispatch slot exceeds bytecode"))?;

                instruction.span(self.register(*receiver)?);
                instruction.u16(slot);
            }
        }

        Ok(())
    }

    /// Select one physical opcode from its control and dispatch forms.
    fn call_opcode(is_invoke: bool, callee: &mir::Callee) -> bytecode::Opcode {
        match (is_invoke, callee) {
            (false, mir::Callee::Direct { .. }) => bytecode::Opcode::CALL,
            (false, mir::Callee::Indirect { .. }) => bytecode::Opcode::CALL_INDIRECT,
            (false, mir::Callee::Virtual { .. }) => bytecode::Opcode::CALL_VIRTUAL,
            (false, mir::Callee::Dynamic { .. }) => bytecode::Opcode::CALL_DYNAMIC,
            (true, mir::Callee::Direct { .. }) => bytecode::Opcode::INVOKE,
            (true, mir::Callee::Indirect { .. }) => bytecode::Opcode::INVOKE_INDIRECT,
            (true, mir::Callee::Virtual { .. }) => bytecode::Opcode::INVOKE_VIRTUAL,
            (true, mir::Callee::Dynamic { .. }) => bytecode::Opcode::INVOKE_DYNAMIC,
            (_, mir::Callee::Witness { .. }) => {
                unreachable!("witness calls resolve at instantiation")
            }
        }
    }
}
