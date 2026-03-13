use super::heap::FIRST_ALLOCATED_REFERENCE_ID;
use super::{INLINE_ALLOCATION_VALUES, ManagedHeap};
use crate::ManagedReference;
use crate::value::Value;
use std::mem::size_of;

use serde::{Deserialize, Serialize};

// TODO #Architecture: revisit whether gc.rs should move out of language/heap during the backend-neutral payload redesign

/// Phase of the garbage collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GcPhase {
    /// GC is idle.
    Idle,
    /// GC is marking reachable objects.
    Mark,
    /// GC is draining remaining work and finalizing the mark phase.
    MarkTermination,
    /// GC is sweeping unreachable objects.
    Sweep,
}

impl GcPhase {
    /// Report whether the GC is currently marking.
    pub fn is_marking(self) -> bool {
        matches!(self, Self::Mark | Self::MarkTermination)
    }
}

/// Summary statistics for a garbage collection cycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcStats {
    /// Number of allocations freed by the collection.
    pub freed_allocations: usize,
    /// Number of live allocations after the collection.
    pub live_allocations: usize,
    /// Number of bytes freed by the collection.
    pub freed_bytes: u64,
    /// Number of live bytes after the collection.
    pub live_bytes: u64,
    /// Total heap bytes after the collection.
    pub heap_bytes: u64,
}

/// GC state tracked across collection cycles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GcState {
    /// Number of completed GC cycles.
    pub cycles: u64,
    /// Current GC phase.
    pub phase: GcPhase,
    /// Stats from the last completed cycle.
    pub last_stats: Option<GcStats>,
}

impl Default for GcState {
    fn default() -> Self {
        Self {
            cycles: 0,
            phase: GcPhase::Idle,
            last_stats: None,
        }
    }
}

impl GcState {
    /// Begin a new GC cycle.
    pub fn begin_cycle(&mut self) {
        self.cycles = self.cycles.saturating_add(1);
        self.phase = GcPhase::Mark;
    }

    /// Finish a GC cycle and record its stats.
    pub fn finish_cycle(&mut self, stats: GcStats) {
        self.phase = GcPhase::Idle;
        self.last_stats = Some(stats);
    }
}

impl ManagedHeap {
    /// Begin one GC cycle.
    pub fn begin_gc_cycle(&mut self, roots: impl IntoIterator<Item = Value>) {
        // ignore duplicate begin requests while one cycle is still active
        if self.gc_state.phase != GcPhase::Idle {
            return;
        }

        // start one new cycle and clear stale page marks
        self.gc_state.begin_cycle();
        self.clear_mark_queue();

        // clear stale mark bits across all live pages
        for page in self.pages.iter_mut() {
            page.clear_marks();
        }

        // seed the mark queue from the provided roots
        for root in roots {
            self.mark_value(root);
        }
    }

    /// Advance one incremental GC step.
    pub fn gc_step(&mut self, budget: usize) {
        let mut remaining = budget;

        // zero budget leaves the current phase unchanged
        if remaining == 0 {
            return;
        }

        // mark phase
        if self.gc_state.phase == GcPhase::Mark {
            while remaining > 0 {
                let Some(handle) = self.mark_queue.pop() else {
                    self.gc_state.phase = GcPhase::Sweep;
                    break;
                };

                remaining -= 1;
                self.apply_retained_delta(-(size_of::<ManagedReference>() as i64));
                self.trace_pointer(handle);
            }
        }

        // sweep phase
        if self.gc_state.phase == GcPhase::Sweep {
            self.finish_gc_cycle();
        }
    }

    /// Run one full GC cycle immediately.
    pub fn collect_handles(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> GcStats {
        self.begin_gc_cycle(roots.into_iter().map(Value::managed_reference));
        self.finish_gc_cycle();

        self.gc_state.last_stats.unwrap_or_default()
    }

    /// Complete one GC cycle immediately.
    pub fn finish_gc_cycle(&mut self) {
        // ignore duplicate finish requests once the collector is idle
        if self.gc_state.phase == GcPhase::Idle {
            return;
        }

        // drain remaining mark work before sweeping
        while let Some(pointer) = self.mark_queue.pop() {
            self.apply_retained_delta(-(size_of::<ManagedReference>() as i64));
            self.trace_pointer(pointer);
        }

        // sweep all currently allocated managed references
        let mut freed = 0_usize;
        let previous_next_unused_id = self.next_unused_id;

        // sweep every allocated handle in stable id order
        for reference_id in FIRST_ALLOCATED_REFERENCE_ID..previous_next_unused_id {
            let handle = ManagedReference::new(reference_id);
            let address = self
                .location(handle)
                .expect("allocated managed reference must keep one stable page slot");
            let (target_page, target_offset) = address.position();
            let page = self.page(target_page);
            let is_occupied = page.is_occupied(target_offset);
            let should_free = is_occupied && !page.is_marked(target_offset);

            // keep marked live allocations
            if !should_free {
                continue;
            }

            let allocation = self
                .page_mut(target_page)
                .take(target_offset)
                .expect("sweep expected occupied managed allocation");
            let mut retained_delta = size_of::<u64>() as i64;

            // recycle external span storage before reusing the handle id
            if let Some(span) = allocation.span() {
                retained_delta += self.free_span_delta(span);
                self.free_span(span);
            }
            self.free_ids.push(reference_id);
            self.allocated_count = self.allocated_count.saturating_sub(1);
            self.allocated_bytes = self
                .allocated_bytes
                .saturating_sub(Self::allocation_size_for(&allocation));
            self.apply_retained_delta(retained_delta);
            freed += 1;
        }

        // publish cycle stats and return to idle
        let stats = GcStats {
            freed_allocations: freed,
            live_allocations: self.allocated_count,
            freed_bytes: 0,
            live_bytes: self.allocated_bytes,
            heap_bytes: self.allocated_bytes,
        };

        self.gc_state.finish_cycle(stats);
        self.mark_queue.clear();
    }

    /// Clear pending mark work without completing a collection cycle.
    pub(crate) fn clear_mark_queue(&mut self) {
        let retained_delta = -((self.mark_queue.len() * size_of::<ManagedReference>()) as i64);
        self.mark_queue.clear();
        self.apply_retained_delta(retained_delta);
    }

    /// Mark one managed reference if it has not already been marked this cycle.
    pub fn mark_reference(&mut self, pointer: ManagedReference) {
        let reference_id = pointer.id();

        // ignore null and out of range handles
        if reference_id == 0 || reference_id >= self.next_unused_id {
            return;
        }

        let Some(address) = self.location(pointer) else {
            return;
        };
        let (target_page, target_offset) = address.position();
        let page = self.page_mut(target_page);

        // skip free or already marked slots
        if !page.is_occupied(target_offset) || page.is_marked(target_offset) {
            return;
        }

        page.mark(target_offset);
        self.mark_queue.push(pointer);
        self.apply_retained_delta(size_of::<ManagedReference>() as i64);
    }

    /// Mark one runtime value if it contains one managed reference.
    pub fn mark_value(&mut self, value: Value) {
        // only managed references contribute new mark work
        if let Some(pointer) = value.as_managed_reference() {
            self.mark_reference(pointer);
        }
    }

    /// Return whether one managed reference is marked in the current cycle.
    pub fn is_marked(&self, pointer: ManagedReference) -> bool {
        let reference_id = pointer.id();
        if reference_id == 0 || reference_id >= self.next_unused_id {
            return false;
        }

        let Some(address) = self.location(pointer) else {
            return false;
        };
        let (target_page, target_offset) = address.position();
        self.page(target_page).is_marked(target_offset)
    }

    // maintain tri color invariant for in place pointer writes
    pub(super) fn write_barrier(&mut self, source_marked: bool, value: Value) {
        if !source_marked || !self.gc_state.phase.is_marking() {
            return;
        }

        self.mark_value(value);
    }

    // trace one pointer's outgoing references
    fn trace_pointer(&mut self, pointer: ManagedReference) {
        let Some(allocation) = self.get(pointer) else {
            return;
        };

        // inline allocations: copy the tiny inline payload, then release the borrow
        if let Some(values) = allocation.inline_values() {
            let mut copied = [Value::VOID; INLINE_ALLOCATION_VALUES];
            copied[..values.len()].copy_from_slice(values);

            for value in copied.into_iter().take(values.len()) {
                self.mark_value(value);
            }

            return;
        }

        // span allocations: walk values directly from value pages
        let Some(span) = allocation.span() else {
            return;
        };

        for offset in 0..span.len() {
            let Some(value) = self.read_span_value(span, offset) else {
                return;
            };
            self.mark_value(value);
        }
    }

    /// Return a cloned list of the currently allocated managed references.
    pub fn allocated_references(&self) -> Vec<ManagedReference> {
        let mut pointers = Vec::with_capacity(self.allocated_count);
        let previous_next_unused_id = self.next_unused_id;

        for reference_id in FIRST_ALLOCATED_REFERENCE_ID..previous_next_unused_id {
            let pointer = ManagedReference::new(reference_id);
            let address = self
                .location(pointer)
                .expect("allocated managed reference must keep one stable page slot");
            let (target_page, target_offset) = address.position();
            if self.page(target_page).is_occupied(target_offset) {
                pointers.push(pointer);
            }
        }

        pointers
    }
}
