use bytecode::{Initialization, Instruction, New, NewKind, RegisterSpan};
use tspp_bytecode as bytecode;
use tspp_heap::{AllocationPlan, HeapError, Payload};
use tspp_mir as mir;
use tspp_program::{AllocationSiteId, Runtime, Word};

use crate::diagnostic::{Error, ExecutionResult, Result};
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one local or shared heap allocation.
    pub(crate) fn execute_new<const OBSERVE: bool, const PROFILE: bool>(
        &mut self,
        instruction: Instruction<'_>,
        operation: New,
    ) -> ExecutionResult<(), R::Error> {
        // decode the allocation operands and optional branches
        let mut operands = self.operands(instruction);
        let results = if operation.kind == NewKind::Slice {
            operands.span()?
        } else {
            let result = operands.register()?;

            RegisterSpan::new(result, 1)
        };
        let site_id = AllocationSiteId(operands.u32()?);
        let site = self
            .machine
            .program
            .sites()
            .allocation_by_id(self.machine.program.sections(), site_id)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        let plan = self.activation.memory.allocation_plan(site_id);
        let length = if operation.kind == NewKind::Slice {
            let length = operands.register()?;

            self.read(length.0).as_u64() as usize
        } else {
            1
        };
        let branches = if operation.is_fallible {
            let success = operands.i32()?;
            let failure = operands.i32()?;

            Some((success, failure))
        } else {
            None
        };

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
        let allocation = self.activation.memory.allocate(
            site.space,
            plan,
            payload,
            self.machine.program.trace_view(),
        );

        // route explicit allocation failure without changing result registers
        let reference = match (allocation, branches) {
            (Ok(reference), Some((success, _))) => {
                self.branch(success);

                reference
            }
            (Err(error), Some((_, failure))) if Self::is_allocation_failure(&error) => {
                self.branch(failure);

                return Ok(());
            }
            (Err(error), _) => return Err(Error::heap(error).into()),
            (Ok(reference), None) => reference,
        };

        // lead a class object with its virtual table id
        if let Some(table) = site.virtual_table.get() {
            self.activation.memory.write_virtual_table(reference, table);
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
        if OBSERVE {
            self.observe_allocation(site_id, reference, plan.byte_len())?;
        }

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
        let plan = self.activation.memory.plan_allocation(space, &shape);

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
