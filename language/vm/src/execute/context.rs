use std::ptr;

use tspp_bytecode::{Instruction, Opcode};
use tspp_heap::{HeapEdge, HeapReference, Payload};
use tspp_mir as mir;
use tspp_program::{AllocationSiteId, Context, ContextNode, Runtime, Word};

use crate::diagnostic::{Error, ExecutionResult, Result};
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one execution context operation.
    pub(crate) fn execute_context<const OBSERVE: bool, const PROFILE: bool>(
        &mut self,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        match instruction.opcode() {
            Opcode::CONTEXT_CURRENT => self
                .execute_context_current(instruction)
                .map_err(Into::into),
            Opcode::CONTEXT_REPLACE => self
                .execute_context_replace(instruction)
                .map_err(Into::into),
            Opcode::CONTEXT_BIND => self.execute_context_bind::<OBSERVE, PROFILE>(instruction),
            Opcode::CONTEXT_GET => self.execute_context_get(instruction).map_err(Into::into),
            _ => Err(self.invalid_instruction().into()),
        }
    }

    /// Load the current execution context.
    fn execute_context_current(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.register()?;
        let context = self.activation.context.reference();
        self.write(result.0, Word::from_bits(context.bits() as u64));

        Ok(())
    }

    /// Replace the current execution context.
    fn execute_context_replace(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.register()?;
        let context = operands.register()?;
        let context = HeapReference::from_bits(self.read(context.0).bits() as usize);
        let previous = std::mem::replace(self.activation.context, Context::new(context));
        self.write(
            result.0,
            Word::from_bits(previous.reference().bits() as u64),
        );

        Ok(())
    }

    /// Allocate and initialize one immutable context node.
    fn execute_context_bind<const OBSERVE: bool, const PROFILE: bool>(
        &mut self,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        let mut operands = self.operands(instruction);
        let result = operands.register()?;
        let site_id = AllocationSiteId(operands.u32()?);
        let context = operands.register()?;
        let variable = operands.register()?;
        let value = operands.span()?;
        let value_offset = operands.u32()? as usize;
        let site = self
            .machine
            .program
            .sites()
            .allocation_by_id(self.machine.program.sections(), site_id)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        if site.space != mir::Space::Local {
            return Err(self.invalid_instruction().into());
        }
        let plan = self.activation.memory.allocation_plan(site_id);
        let edge = self
            .activation
            .memory
            .allocate(
                site.space,
                plan,
                Payload::Zeroed,
                self.machine.program.trace_view(),
            )
            .map_err(Error::heap)?;
        let HeapEdge::Local(reference) = edge else {
            return Err(self.invalid_instruction().into());
        };
        let address = self.activation.memory.address(edge);
        let node = ContextNode {
            parent: Context::new(HeapReference::from_bits(
                self.read(context.0).bits() as usize
            )),
            variable: HeapReference::from_bits(self.read(variable.0).bits() as usize),
        };

        // initialize the fixed header and inline Copy value before publication
        // SAFETY: the linked allocation and value offset describe this complete node
        unsafe {
            ptr::write_unaligned(address as *mut ContextNode, node);
            self.cursor.store_words(
                value.start.0,
                (address + value_offset) as *mut Word,
                value.word_count,
            );
        }
        self.activation
            .memory
            .barrier(
                edge,
                0,
                plan.byte_len as usize,
                self.machine.program.trace_view(),
            )
            .map_err(Error::heap)?;
        self.write(result.0, Word::from_bits(reference.bits() as u64));

        // record the hidden allocation under its ordinary Program site
        if PROFILE {
            let Some(profile) = self.profile.as_deref_mut() else {
                unreachable!("profiled dispatch requires an active profile");
            };
            profile.record_allocation(site_id, plan.byte_len);
        }
        if OBSERVE {
            self.observe_allocation(site_id, edge, plan.byte_len())?;
        }

        Ok(())
    }

    /// Find one context variable value or copy its explicit default.
    fn execute_context_get(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.span()?;
        let context = operands.register()?;
        let variable = operands.register()?;
        let default = operands.span()?;
        let value_offset = operands.u32()? as usize;
        let variable = HeapReference::from_bits(self.read(variable.0).bits() as usize);
        let mut context = HeapReference::from_bits(self.read(context.0).bits() as usize);

        // walk immutable headers until the nearest matching variable is found
        while !context.is_null() {
            let address = self.activation.memory.base_address() + context.bits();

            // SAFETY: every nonempty context reference addresses one ContextNode allocation
            let node = unsafe { ptr::read_unaligned(address as *const ContextNode) };
            if node.variable == variable {
                // SAFETY: the opcode carries the matching concrete value field offset and width
                unsafe {
                    self.cursor.load_words(
                        (address + value_offset) as *const Word,
                        result.start.0,
                        result.word_count,
                    );
                }

                return Ok(());
            }
            context = node.parent.reference();
        }

        self.move_range(default, result);

        Ok(())
    }
}
