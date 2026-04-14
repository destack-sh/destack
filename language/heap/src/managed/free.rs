use super::{ManagedLocation, ManagedReferenceRecord, ManagedSpace, ReferenceMapId};
use crate::value::ManagedReference;

impl ManagedSpace {
    /// Free one managed allocation.
    pub fn free(&mut self, reference: ManagedReference) -> bool {
        // resolve the live allocation first
        let reference_id = reference.id();
        let Some(record) = self.reference(reference).copied() else {
            return false;
        };
        let location = record.location();

        match location {
            // release one young allocation in place
            ManagedLocation::Young(young_id) => {
                let Some(allocation) = self.young_allocation_mut(young_id) else {
                    return false;
                };

                if !allocation.is_allocated {
                    return false;
                }

                allocation.is_allocated = false;
                allocation.marked = false;
                self.young.free_ids = self.push_boxed_u32(&self.young.free_ids, young_id.index());

                // update heap accounting
                self.allocated_count = self.allocated_count.saturating_sub(1);
                self.allocated_bytes = self
                    .allocated_bytes
                    .saturating_sub(record.byte_len() as u64);

                // clear the stable reference slot
                self.free_reference_ids.push(reference_id);

                if let Some(entry) = self
                    .references
                    .get_mut(reference_id.saturating_sub(1) as usize)
                {
                    *entry = ManagedReferenceRecord::vacant();
                }

                true
            }
            // release one small-span slot
            ManagedLocation::Small(slot) => {
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return false;
                };

                let slot_index = slot.slot_index();
                let was_full = span.occupied_count == span.slot_count;
                let size_class = span.size_class;

                if !span.occupied.contains(slot_index) {
                    return false;
                }

                span.occupied.clear(slot_index);
                span.occupied_count = span.occupied_count.saturating_sub(1);
                span.next_free_slot = span.next_free_slot.min(slot_index);
                Self::set_span_trace_id(span, slot_index, ReferenceMapId::new(0));
                Self::set_span_layout_id(span, slot_index, None);

                // requeue the span if it was full before the free
                let should_requeue = was_full && span.occupied_count < span.slot_count;

                if should_requeue {
                    let class_index = self.small.size_classes.class_index_for(size_class);

                    if let Some(class_index) = class_index {
                        self.small.available_spans[class_index].push(slot.span_index());
                    }
                }

                // update heap accounting
                self.allocated_count = self.allocated_count.saturating_sub(1);
                self.allocated_bytes = self
                    .allocated_bytes
                    .saturating_sub(record.byte_len() as u64);

                // clear the stable reference slot
                self.free_reference_ids.push(reference_id);

                if let Some(entry) = self
                    .references
                    .get_mut(reference_id.saturating_sub(1) as usize)
                {
                    *entry = ManagedReferenceRecord::vacant();
                }

                true
            }
            // release one allocation in large space and its arena pages
            ManagedLocation::Large(allocation_id) => {
                let arena = self.arena().clone();
                let Some(allocation) = self.allocation_mut(allocation_id) else {
                    return false;
                };

                if !allocation.is_allocated {
                    return false;
                }

                allocation.is_allocated = false;
                let pages = allocation.pages.clone();
                arena.release_pages(&pages);

                // update heap accounting
                self.allocated_count = self.allocated_count.saturating_sub(1);
                self.allocated_bytes = self
                    .allocated_bytes
                    .saturating_sub(record.byte_len() as u64);

                // clear the stable reference slot
                self.free_reference_ids.push(reference_id);

                if let Some(entry) = self
                    .references
                    .get_mut(reference_id.saturating_sub(1) as usize)
                {
                    *entry = ManagedReferenceRecord::vacant();
                }

                true
            }
            ManagedLocation::Vacant => false,
        }
    }
}
