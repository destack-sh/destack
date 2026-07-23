use std::ptr;

use bytecode::{CodeOffset, Initialization, Instruction, New, NewKind, RegisterRange};
use destack_bytecode as bytecode;
use destack_heap::{AllocationPlan, HeapEdge, HeapError, Payload};
use destack_mir as mir;
use destack_program::{LayoutShape, TypeId, VirtualTableId, Word};

use crate::diagnostic::{Error, Result};
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one local or shared heap allocation.
    pub(crate) fn execute_new<const PROFILE: bool>(
        &mut self,
        instruction: Instruction<'_>,
        instruction_offset: CodeOffset,
        operation: New,
    ) -> Result<()> {
        let frame = self.frame();
        let point = self.point(frame, instruction_offset)?;
        let (site_id, site) = self
            .machine
            .program
            .sites()
            .allocation(self.machine.program.sections(), point)
            .map(|(id, site)| (id, *site))
            .ok_or_else(|| self.invalid_instruction())?;
        let plan = self
            .call
            .memory
            .allocation_plan(site_id)
            .ok_or_else(|| self.invalid_instruction())?;

        // decode the allocation operands and optional branches
        let mut operands = instruction.operands();
        let results = if operation.kind == NewKind::Slice {
            operands.range().map_err(|_| self.invalid_instruction())?
        } else {
            let result = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;

            RegisterRange::new(result, 1)
        };
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let length = if operation.kind == NewKind::Slice {
            let length = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;

            self.read(length.0).as_u64() as usize
        } else {
            1
        };
        let branches = if operation.is_fallible {
            let success = operands.i32().map_err(|_| self.invalid_instruction())?;
            let failure = operands.i32().map_err(|_| self.invalid_instruction())?;

            Some((success, failure))
        } else {
            None
        };

        // require the linked site to select the same storage
        let is_expected_space = matches!(
            (operation.space, site.space),
            (bytecode::Space::LOCAL, mir::Space::Local)
                | (bytecode::Space::SHARED, mir::Space::Shared)
        );
        if !is_expected_space || site.storage_type != ty {
            return Err(self.invalid_instruction());
        }

        // derive repeated storage only for variable-length slice allocation
        let plan = if operation.kind == NewKind::Slice {
            self.repeated_plan(site.space, plan, length)?
        } else {
            plan
        };
        let payload = if operation.initialization == Initialization::Zeroed {
            Payload::Zeroed
        } else {
            Payload::Uninit
        };
        let allocation =
            self.call
                .memory
                .allocate(site.space, plan, payload, self.machine.program.trace_view());

        // route explicit allocation failure without changing result registers
        let reference = match (allocation, branches) {
            (Ok(reference), Some((success, _))) => {
                self.frame_mut().branch(success);

                reference
            }
            (Err(error), Some((_, failure))) if Self::is_allocation_failure(&error) => {
                self.frame_mut().branch(failure);

                return Ok(());
            }
            (Err(error), _) => return Err(Error::heap(error)),
            (Ok(reference), None) => reference,
        };

        // initialize the dispatch word only for virtual objects
        if let Some(table) = site.virtual_table.get() {
            self.initialize_dispatch(reference, site.storage_type, table)?;
        }

        // materialize the one-word reference or two-word slice descriptor
        self.write(results.start.0, Word::from_bits(reference.bits() as u64));
        if operation.kind == NewKind::Slice {
            self.write(results.start.0 + 1, Word::uint64(length as u64));
        }

        // record only successful allocations in observed execution
        if PROFILE {
            let Some(profile) = self.profile.as_deref_mut() else {
                unreachable!("profiled dispatch requires an active profile");
            };

            profile.record_allocation(site_id, plan.byte_len);
        }

        Ok(())
    }

    /// Initialize one virtual object's durable dispatch table id.
    fn initialize_dispatch(&self, edge: HeapEdge, ty: TypeId, table: VirtualTableId) -> Result<()> {
        let layout = self
            .machine
            .program
            .layout(ty)
            .ok_or_else(|| self.invalid_instruction())?;
        let LayoutShape::Object(object) = layout.shape else {
            return Err(self.invalid_instruction());
        };
        let Some(offset) = object.dispatch_offset.get() else {
            return Err(self.invalid_instruction());
        };
        let address = self.call.memory.native_address(edge) + offset as usize;

        // SAFETY: the linked object layout reserves this field inside the new allocation
        unsafe { ptr::write_unaligned(address as *mut u32, table.0) };

        Ok(())
    }

    /// Complete one initialized allocation.
    pub(crate) fn execute_new_complete(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let results = operands.range().map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;

        self.move_range(source, results);

        Ok(())
    }

    /// Build one runtime-sized slice allocation plan.
    fn repeated_plan(
        &self,
        space: mir::Space,
        element: AllocationPlan,
        length: usize,
    ) -> Result<AllocationPlan> {
        let trace_map = element
            .trace_map(self.machine.program.trace_view())
            .map_err(Error::heap)?;
        let shape = element.repeat(&trace_map, length).map_err(Error::heap)?;
        let plan = self.call.memory.plan_allocation(space, &shape);

        Ok(plan)
    }

    /// Return whether one heap failure follows a fallible allocation edge.
    fn is_allocation_failure(error: &HeapError) -> bool {
        matches!(
            error,
            HeapError::LimitExceeded { .. } | HeapError::Memory { .. }
        )
    }
}
