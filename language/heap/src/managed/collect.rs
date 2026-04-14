use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use super::{GcKind, GcStats, ManagedLocation, ManagedSpace, ManagedYoungId, ReferenceMapError};
use crate::value::ManagedReference;

/// Managed collection failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedCollectError {
    /// One managed payload failed to trace its references.
    Trace {
        /// The managed allocation being traced.
        reference: ManagedReference,
        /// The underlying tracing failure.
        error: ReferenceMapError,
    },
}

impl Display for ManagedCollectError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trace { reference, error } => {
                write!(
                    formatter,
                    "managed heap tracing failed for reference {}: {error}",
                    reference.id()
                )
            }
        }
    }
}

impl Error for ManagedCollectError {}

impl ManagedSpace {
    /// Return the currently allocated managed references.
    pub fn allocated_references(&self) -> Vec<ManagedReference> {
        // compact the dense table back into stable live references
        self.references
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                (!record.is_vacant()).then_some(ManagedReference::new(index as u64 + 1))
            })
            .collect()
    }

    /// Perform one young-generation collection over explicit managed roots.
    pub fn collect_young_references<I>(&mut self, roots: I) -> Result<GcStats, ManagedCollectError>
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        // begin one minor cycle
        self.gc_state.begin_cycle(GcKind::Minor);

        // trace every reachable managed allocation from the explicit roots
        let reachable = self.reachable_reference_ids(roots)?;
        let live_references = self.allocated_references();
        let (freed_allocations, freed_bytes) =
            self.promote_or_free_young_references(&reachable, &live_references);

        // reset the nursery after promotion and sweeping
        self.reset_young_space();

        // finalize the completed minor-cycle statistics
        let stats = self.stats_after_collection(freed_allocations, freed_bytes);

        self.gc_state.finish_cycle(stats);

        Ok(stats)
    }

    /// Perform one full managed collection over explicit managed roots.
    pub fn collect_references<I>(&mut self, roots: I) -> Result<GcStats, ManagedCollectError>
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        // materialize roots once for the minor and full passes
        let roots = roots.into_iter().collect::<Vec<_>>();
        let _minor = self.collect_young_references(roots.iter().copied())?;

        // begin one full-cycle mark and sweep
        self.gc_state.begin_cycle(GcKind::Full);

        // trace every reachable mature allocation from the explicit roots
        let reachable = self.reachable_reference_ids(roots.iter().copied())?;
        let live_references = self.allocated_references();
        let (freed_allocations, freed_bytes) =
            self.free_unreachable_references(&reachable, &live_references);

        // finalize the completed full-cycle statistics
        let stats = self.stats_after_collection(freed_allocations, freed_bytes);

        self.gc_state.finish_cycle(stats);

        Ok(stats)
    }

    /// Promote one live young allocation into the mature spaces.
    fn promote_young_allocation(&mut self, reference: ManagedReference, young_id: ManagedYoungId) {
        // skip stale nursery ids that disappeared during the cycle
        let Some(allocation) = self.young_allocation(young_id).cloned() else {
            return;
        };

        // materialize the nursery bytes before relocating the allocation
        let bytes = self.arena().bytes_to_vec_from(
            &self.young.pages,
            self.young_allocation_offset(&allocation),
            allocation.byte_len,
        );
        let layout_id = allocation.layout_id.to_option();

        // prefer one mature small slot before falling back to one allocation in large space
        let location = if let Some(slot) =
            self.allocate_small_slot(&bytes, allocation.trace_id, layout_id)
        {
            ManagedLocation::Small(slot)
        } else {
            let pages = self.arena().allocate_bytes(&bytes);
            let allocation_id =
                self.allocate_allocation_slot(bytes.len(), pages, allocation.trace_id, layout_id);

            ManagedLocation::Large(allocation_id)
        };

        // rewrite the live reference to point at its mature location
        if let Some(record) = self.reference_mut(reference) {
            record.set_location(location);
        }
    }

    /// Promote or free every young reference seen during one minor collection.
    fn promote_or_free_young_references(
        &mut self,
        reachable: &[usize],
        live_references: &[ManagedReference],
    ) -> (usize, u64) {
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // walk every currently live managed reference
        for &reference in live_references {
            let Some(record) = self.reference(reference).copied() else {
                continue;
            };

            // skip mature references during the young pass
            let ManagedLocation::Young(young_id) = record.location() else {
                continue;
            };

            // promote reachable nursery allocations
            if reachable.contains(&(reference.id() as usize)) {
                self.promote_young_allocation(reference, young_id);
                continue;
            }

            // otherwise free unreachable nursery allocations
            if self.free(reference) {
                freed_allocations = freed_allocations.saturating_add(1);
                freed_bytes = freed_bytes.saturating_add(record.byte_len() as u64);
            }
        }

        (freed_allocations, freed_bytes)
    }

    /// Free every unreachable managed reference during one full collection.
    fn free_unreachable_references(
        &mut self,
        reachable: &[usize],
        live_references: &[ManagedReference],
    ) -> (usize, u64) {
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // walk every currently live managed reference
        for &reference in live_references {
            // keep reachable references intact
            if reachable.contains(&(reference.id() as usize)) {
                continue;
            }

            let Some(record) = self.reference(reference).copied() else {
                continue;
            };

            // free unreachable references and charge the reclaimed bytes
            if self.free(reference) {
                freed_allocations = freed_allocations.saturating_add(1);
                freed_bytes = freed_bytes.saturating_add(record.byte_len() as u64);
            }
        }

        (freed_allocations, freed_bytes)
    }

    /// Reset the nursery after one collection cycle.
    fn reset_young_space(&mut self) {
        // clone the shared arena handle once for the reset path
        let arena = self.arena().clone();

        // rebuild the nursery over the same arena
        self.young.reset(&arena);
    }

    /// Walk one managed allocation and return every reachable managed reference id.
    fn reachable_reference_ids<I>(&self, roots: I) -> Result<Vec<usize>, ManagedCollectError>
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        // track one mark bit for each stable managed reference id
        let mut reachable = vec![false; self.references.len().saturating_add(1)];

        // seed the work queue from the explicit roots
        let mut pending = roots
            .into_iter()
            .filter(|reference| !reference.is_null())
            .collect::<VecDeque<_>>();

        // drain the explicit root queue and trace each reachable payload once
        while let Some(reference) = pending.pop_front() {
            let reference_id = reference.id() as usize;

            // skip references that are already marked or outside the stable table
            if reference_id >= reachable.len() || reachable[reference_id] {
                continue;
            }

            // skip dead reference slots
            let Some(record) = self.reference(reference).copied() else {
                continue;
            };

            // mark this reference before walking its outgoing edges
            reachable[reference_id] = true;

            // skip allocations with no tracing metadata or no accessible payload
            let Some(trace_id) = self.location_trace_id(record.location()) else {
                continue;
            };
            let Some(reference_map) = self.reference_map_table.map(trace_id) else {
                continue;
            };
            let Some(bytes) = self.bytes(reference) else {
                continue;
            };

            // enqueue every non-null edge discovered in this payload
            reference_map
                .trace_references(&bytes, self.managed_reference_bytes, |reference| {
                    if !reference.is_null() {
                        pending.push_back(reference);
                    }
                })
                .map_err(|error| ManagedCollectError::Trace { reference, error })?;
        }

        // compact the mark bits back into stable reference ids
        let reachable = reachable
            .into_iter()
            .enumerate()
            .filter_map(|(reference_id, is_reachable)| is_reachable.then_some(reference_id))
            .collect();

        Ok(reachable)
    }

    /// Build one GC statistics snapshot after one collection pass.
    fn stats_after_collection(&self, freed_allocations: usize, freed_bytes: u64) -> GcStats {
        GcStats {
            freed_allocations,
            live_allocations: self.allocated_count,
            freed_bytes,
            allocated_bytes: self.allocated_bytes,
            active_bytes: self.active_bytes(),
        }
    }
}
