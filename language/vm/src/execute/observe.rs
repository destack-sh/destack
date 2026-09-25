use tspp_bytecode::CodeOffset;
use tspp_heap::HeapEdge;
use tspp_mir::Storage;
use tspp_program::{AllocationSiteId, Event, EventKind, FunctionId, MemoryAccess, Runtime};

use crate::diagnostic::{ExecutionError, ExecutionResult};
use crate::machine::{Activation, Frame};

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Observe one selected Program execution event.
    pub(crate) fn observe(&mut self, event: Event) -> ExecutionResult<(), R::Error> {
        if !self.events.contains(event.kind()) {
            return Ok(());
        }

        self.activation
            .runtime
            .observe(self.fiber.fiber_id, event)
            .map_err(ExecutionError::runtime)
    }

    /// Observe one completed allocation.
    pub(crate) fn observe_allocation(
        &mut self,
        site_id: AllocationSiteId,
        edge: HeapEdge,
        byte_len: usize,
    ) -> ExecutionResult<(), R::Error> {
        if !self.events.contains(EventKind::Allocation) {
            return Ok(());
        }

        let site = self
            .machine
            .program
            .sites()
            .allocation_by_id(self.machine.program.sections(), site_id)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        let address = self.activation.memory.address(edge);
        let storage = Storage::heap(site.space);
        let range = self.executed_memory_range(Some(storage), address, byte_len)?;

        self.observe(Event::Allocation { site, range })
    }

    /// Observe one function call.
    pub(crate) fn observe_call(
        &mut self,
        frame: Frame,
        pc: CodeOffset,
        function: FunctionId,
    ) -> ExecutionResult<(), R::Error> {
        if !self.events.contains(EventKind::Call) {
            return Ok(());
        }

        let point = self.point(frame, pc)?;
        let site = self
            .machine
            .program
            .sites()
            .call(self.machine.program.sections(), point)
            .map(|(_, site)| *site)
            .ok_or_else(|| self.invalid_instruction())?;

        self.observe(Event::Call { site, function })
    }

    /// Observe one taken control-flow edge.
    pub(crate) fn observe_edge(
        &mut self,
        frame: Frame,
        source: CodeOffset,
        target: CodeOffset,
    ) -> ExecutionResult<(), R::Error> {
        if !self.events.contains(EventKind::Edge) {
            return Ok(());
        }

        let source = self.point(frame, source)?;
        let target = self.point(frame, target)?;
        let site = self
            .machine
            .program
            .sites()
            .edge(self.machine.program.sections(), source, target)
            .map(|(_, site)| *site)
            .ok_or_else(|| self.invalid_instruction())?;

        self.observe(Event::Edge { site })
    }

    /// Observe one completed memory access.
    pub(crate) fn observe_memory(
        &mut self,
        frame: Frame,
        pc: CodeOffset,
        site_index: usize,
        access: MemoryAccess,
        address: Option<(usize, usize)>,
    ) -> ExecutionResult<(), R::Error> {
        let point = self.point(frame, pc)?;
        let sites = self
            .machine
            .program
            .sites()
            .memory(self.machine.program.sections(), point);
        let Some(site) = sites.get(site_index).copied() else {
            return Err(self.invalid_instruction().into());
        };
        if site.access != access {
            return Err(self.invalid_instruction().into());
        }
        let range = address
            .map(|(address, byte_len)| {
                self.executed_memory_range(site.storage.get(), address, byte_len)
            })
            .transpose()?;

        self.observe(Event::Memory {
            site,
            access,
            range,
        })
    }
}
