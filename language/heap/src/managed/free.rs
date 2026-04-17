use super::{ManagedLocation, ManagedSpace, MapId};
use crate::value::ManagedReference;
use crate::{HeapDomain, HeapError, HeapResult};

impl ManagedSpace {
    /// Free one managed entry.
    pub fn free(&mut self, reference: ManagedReference) -> HeapResult<bool> {
        // resolve the live entry first
        let reference_id = reference.id();
        let Some(record) = self.reference(reference).copied() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let freed_bytes = record.byte_len() as u64;
        self.totals.check_free(freed_bytes, HeapDomain::Managed)?;

        match location {
            // release one young entry in place
            ManagedLocation::Young(young_id) => {
                let Some(entry) = self.young_entry_mut(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };

                if !entry.is_live {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                }

                entry.is_live = false;

                // update heap usage
                self.totals.free(freed_bytes, HeapDomain::Managed)?;

                // clear the stable reference slot
                self.retire_reference(reference_id)?;

                Ok(true)
            }
            // release one small-span slot
            ManagedLocation::Small(slot) => {
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                let slot_index = slot.slot_index();
                let was_full = span.occupied_count == span.slot_count;
                let size_class = span.size_class;

                if !span.occupied.contains(slot_index) {
                    return Err(HeapError::MissingSmallSlot {
                        span_index: slot.span_index(),
                        slot_index,
                    });
                }
                if span.occupied_count == 0 {
                    return Err(HeapError::MissingSmallSlot {
                        span_index: slot.span_index(),
                        slot_index,
                    });
                }

                span.occupied.clear(slot_index);
                span.occupied_count -= 1;
                span.next_free_slot = span.next_free_slot.min(slot_index);
                span.set_map_id(slot_index, MapId::empty());
                span.set_layout_id(slot_index, None);

                // requeue the span if it was full before the free
                let should_requeue = was_full && span.occupied_count < span.slot_count;

                if should_requeue {
                    let class_index = self.small.size_classes.class_index_for(size_class);

                    if let Some(class_index) = class_index {
                        self.small.available_spans[class_index].push(slot.span_index());
                    }
                }

                // update heap usage
                self.totals.free(freed_bytes, HeapDomain::Managed)?;

                // clear the stable reference slot
                self.retire_reference(reference_id)?;

                Ok(true)
            }
            // release one entry in large space and its arena pages
            ManagedLocation::Large(entry_id) => {
                let arena = self.arena().clone();
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                if !entry.is_live {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                }

                let pages = entry.pages;

                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                // retire the live large-entry slot before releasing its pages
                entry.retire();
                self.large.free_large_entry_ids.push(entry_id.id());

                // update heap usage
                self.totals.free(freed_bytes, HeapDomain::Managed)?;

                // clear the stable reference slot
                self.retire_reference(reference_id)?;

                // release the old physical pages after the live slot is gone
                arena.release_page_view(&pages)?;

                Ok(true)
            }
        }
    }
}
