use super::{RawLocation, RawPointerRecord, RawSpace};
use crate::value::RawPointer;

impl RawSpace {
    /// Free one raw allocation.
    pub fn free(&mut self, pointer: RawPointer) -> bool {
        // resolve the live allocation first
        let pointer_id = pointer.id();
        let Some(record) = self.pointer(pointer).copied() else {
            return false;
        };

        match record.location {
            // release one small-span slot
            RawLocation::Small(slot) => {
                self.release_small_slot(slot);

                // update heap accounting
                self.allocated_count = self.allocated_count.saturating_sub(1);
                self.allocated_bytes = self.allocated_bytes.saturating_sub(record.byte_len as u64);

                // clear the stable pointer slot
                if let Some(entry) = self.pointers.get_mut(pointer_id.saturating_sub(1) as usize) {
                    *entry = RawPointerRecord::vacant();
                }

                self.free_pointer_ids.push(pointer_id);

                true
            }
            // release one allocation in large space and its arena pages
            RawLocation::Large(allocation_id) => {
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
                self.allocated_bytes = self.allocated_bytes.saturating_sub(record.byte_len as u64);

                // clear the stable pointer slot
                if let Some(entry) = self.pointers.get_mut(pointer_id.saturating_sub(1) as usize) {
                    *entry = RawPointerRecord::vacant();
                }

                self.free_pointer_ids.push(pointer_id);

                true
            }
            RawLocation::Vacant => false,
        }
    }
}
