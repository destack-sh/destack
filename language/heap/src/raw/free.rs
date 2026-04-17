use super::{RawLocation, RawSpace};
use crate::value::RawPointer;
use crate::{HeapError, HeapResult};

impl RawSpace {
    /// Free one raw entry.
    pub fn free(&mut self, pointer: RawPointer) -> HeapResult<bool> {
        // resolve the live entry first
        let pointer_id = pointer.id();
        let Some(record) = self.pointer(pointer).copied() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let freed_bytes = record.byte_len as u64;
        self.totals
            .check_free(freed_bytes, crate::HeapDomain::Raw)?;

        match location {
            // release one small-span slot
            RawLocation::Small(slot) => {
                self.release_small_slot(slot)?;

                // update heap usage
                self.totals.free(freed_bytes, crate::HeapDomain::Raw)?;

                // clear the stable pointer slot
                self.retire_pointer(pointer_id)?;

                Ok(true)
            }
            // release one entry in large space and its arena pages
            RawLocation::Large(entry_id) => {
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
                self.totals.free(freed_bytes, crate::HeapDomain::Raw)?;

                // clear the stable pointer slot
                self.retire_pointer(pointer_id)?;

                // release the old physical pages after the live slot is gone
                arena.release_page_view(&pages)?;

                Ok(true)
            }
        }
    }
}
