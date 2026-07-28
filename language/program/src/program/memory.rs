use std::fmt;

use destack_heap::{
    AllocationCache, AllocationPlan, AllocationShape, Heap, HeapEdge, HeapResult, Payload,
    SharedHeap, SharedMarkWorker, TraceView,
};
use destack_mir::Space;

use crate::{AllocationSiteId, StaticImage, StaticSpace};

/// Memory available to one program activation.
pub struct Memory<'a> {
    /// Runtime allocation plans indexed by Program allocation site id.
    pub allocation_plans: &'a [Option<AllocationPlan>],
    /// Worker heap.
    pub heap: &'a mut Heap,
    /// Runtime heap.
    pub shared_heap: &'a SharedHeap,
    /// Worker-local shared allocation cache.
    pub shared_cache: &'a mut AllocationCache,
    /// Shared heap mark worker.
    pub shared_mark_worker: &'a SharedMarkWorker,
    /// Local static memory.
    pub local_static: &'a mut StaticSpace,
    /// Shared static memory.
    pub shared_static: &'a mut StaticSpace,
    /// Program constant memory.
    pub constant_space: &'a StaticImage,
}

impl Memory<'_> {
    /// Reborrow this memory for one nested activation.
    pub fn reborrow(&mut self) -> Memory<'_> {
        Memory {
            allocation_plans: self.allocation_plans,
            heap: self.heap,
            shared_heap: self.shared_heap,
            shared_cache: self.shared_cache,
            shared_mark_worker: self.shared_mark_worker,
            local_static: self.local_static,
            shared_static: self.shared_static,
            constant_space: self.constant_space,
        }
    }

    /// Return one runtime allocation plan by Program allocation site id.
    #[inline(always)]
    pub fn allocation_plan(&self, site: AllocationSiteId) -> Option<AllocationPlan> {
        self.allocation_plans[site.index()]
    }

    /// Build one dynamically sized allocation plan for the selected heap.
    pub fn plan_allocation(&self, space: Space, shape: &AllocationShape) -> AllocationPlan {
        match space {
            Space::Local => self.heap.options().allocation_plan(shape),
            Space::Shared => self.shared_heap.options().allocation_plan(shape),
            _ => unreachable!("program allocations use local or shared storage"),
        }
    }

    /// Allocate one payload through the selected heap.
    pub fn allocate(
        &mut self,
        space: Space,
        plan: AllocationPlan,
        payload: Payload<'_>,
        trace_view: TraceView<'_>,
    ) -> HeapResult<HeapEdge> {
        match space {
            Space::Local => self.allocate_local(plan, payload, trace_view),
            Space::Shared => self.allocate_shared(plan, payload, trace_view),
            _ => unreachable!("program allocations use local or shared storage"),
        }
    }

    /// Resolve one stable heap edge into an ephemeral native address.
    pub fn address(&self, edge: HeapEdge) -> usize {
        match edge {
            HeapEdge::Local(reference) => self.heap.heap_base_address() + reference.offset(),
            HeapEdge::Shared(reference) => {
                self.shared_heap.heap_base_address() + reference.offset()
            }
        }
    }

    /// Return one heap address as an offset within its selected space.
    pub fn heap_offset(&self, space: Space, address: usize) -> Option<usize> {
        match space {
            Space::Local => address.checked_sub(self.heap.heap_base_address()),
            Space::Shared => address.checked_sub(self.shared_heap.heap_base_address()),
            Space::Frame | Space::Static => None,
        }
    }

    /// Free one uniquely owned heap allocation.
    pub fn free(&mut self, edge: HeapEdge) -> HeapResult<()> {
        match edge {
            HeapEdge::Local(reference) => self.heap.free(reference),
            HeapEdge::Shared(reference) => self.shared_heap.free(self.shared_cache, reference),
        }
    }

    /// Pin one managed heap allocation against movement.
    pub fn pin(&mut self, edge: HeapEdge) -> HeapResult<HeapEdge> {
        match edge {
            HeapEdge::Local(reference) => self.heap.pin(reference).map(HeapEdge::Local),
            HeapEdge::Shared(reference) => Ok(HeapEdge::Shared(reference)),
        }
    }

    /// Release one managed heap pin.
    pub fn unpin(&mut self, edge: HeapEdge) -> HeapResult<()> {
        match edge {
            HeapEdge::Local(reference) => self.heap.unpin(reference),
            HeapEdge::Shared(_) => Ok(()),
        }
    }

    /// Record one completed managed-reference write.
    pub fn barrier(
        &mut self,
        edge: HeapEdge,
        start: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        match edge {
            HeapEdge::Local(reference) => self
                .heap
                .write_barrier(reference, start, byte_len, trace_view),
            HeapEdge::Shared(reference) => self
                .shared_heap
                .write_barrier(reference, start, byte_len, trace_view),
        }
    }

    /// Allocate one worker-local payload.
    fn allocate_local(
        &mut self,
        plan: AllocationPlan,
        payload: Payload<'_>,
        trace_view: TraceView<'_>,
    ) -> HeapResult<HeapEdge> {
        // reserve directly from the active small-allocation cursor
        if !matches!(payload, Payload::Bytes(_))
            && let Some(small) = plan.small_allocation()
        {
            let reference = if plan.is_noscan() {
                self.heap.reserve_small_noscan(small)
            } else if plan.has_shared_reference() {
                self.heap.reserve_small_shared_edge(small)
            } else {
                self.heap.reserve_small_scan(small)
            };
            if let Some(reference) = reference {
                if payload == Payload::Zeroed {
                    self.heap.zero(reference, plan.byte_len())?;
                }

                return Ok(HeapEdge::Local(reference));
            }
        }

        // materialize trace rows only on the cold allocation path
        let trace_map = plan.trace_map(trace_view)?;
        let reference = match payload {
            Payload::Bytes(bytes) => self.heap.allocate_bytes(plan, &trace_map, bytes)?,
            Payload::Zeroed => self.heap.allocate_zeroed(plan, &trace_map)?,
            Payload::Uninit => self.heap.allocate_uninit(plan, &trace_map)?,
        };

        Ok(HeapEdge::Local(reference))
    }

    /// Allocate one runtime-shared payload.
    fn allocate_shared(
        &mut self,
        plan: AllocationPlan,
        payload: Payload<'_>,
        trace_view: TraceView<'_>,
    ) -> HeapResult<HeapEdge> {
        // reserve directly from the worker-owned shared allocation cursor
        if !matches!(payload, Payload::Bytes(_))
            && let Some(small) = plan.small_allocation()
            && let Some(reference) = self
                .shared_heap
                .reserve_small_from_cache(self.shared_cache, small.small)
        {
            if payload == Payload::Zeroed {
                self.shared_heap.zero(reference, plan.byte_len())?;
            }

            return Ok(HeapEdge::Shared(reference));
        }

        // materialize trace rows only on the cold allocation path
        let trace_map = plan.trace_map(trace_view)?;
        let reference = match payload {
            Payload::Bytes(bytes) => self.shared_heap.allocate_bytes(
                self.shared_mark_worker,
                self.shared_cache,
                plan,
                &trace_map,
                bytes,
                trace_view,
            )?,
            Payload::Zeroed => self.shared_heap.allocate_zeroed(
                self.shared_mark_worker,
                self.shared_cache,
                plan,
                &trace_map,
                trace_view,
            )?,
            Payload::Uninit => self.shared_heap.allocate_uninit(
                self.shared_mark_worker,
                self.shared_cache,
                plan,
                &trace_map,
                trace_view,
            )?,
        };

        Ok(HeapEdge::Shared(reference))
    }
}

impl fmt::Debug for Memory<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Memory")
            .field("allocation_plan_count", &self.allocation_plans.len())
            .field("heap", &"<heap>")
            .field("shared_heap", &"<shared heap>")
            .field("shared_cache", &"<shared allocation cache>")
            .field("shared_mark_worker", &"<shared mark worker>")
            .field("local_static", &self.local_static.byte_len())
            .field("shared_static", &self.shared_static.byte_len())
            .field("constant_space", &"<constant image>")
            .finish()
    }
}
