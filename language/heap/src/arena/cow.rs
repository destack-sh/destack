use super::arena::Arena;
use super::{PageId, PageSlot, PageView};
use crate::{HeapError, HeapResult};

impl Arena {
    /// Overwrite one logical byte range inside one page view.
    ///
    /// Payload mutation is crate-internal so the heap surfaces can enforce
    /// the required ownership and synchronization rules around shared access.
    pub(crate) fn set_bytes(
        &self,
        page_view: &mut PageView,
        start: usize,
        source: &[u8],
    ) -> HeapResult<()> {
        let end = self.byte_range_end(page_view, start, source.len())?;

        let page_bytes = self.page_bytes();
        let start_page = start / page_bytes;
        let end_page = end.div_ceil(page_bytes);

        // detach any shared pages before mutating them
        self.detach_shared_write_range(page_view, start_page, end_page)?;

        // copy the source bytes into each touched page slice
        self.copy_bytes_into_pages(page_view, start_page, end_page, start, end, source)
    }

    /// Ensure one logical page is unique before mutation.
    fn detach_page_for_write(
        &self,
        page_view: &mut PageView,
        page_index: usize,
        slot: PageSlot,
    ) -> HeapResult<()> {
        if page_view.is_empty() {
            return Ok(());
        }

        // collapse back to one contiguous run when patches are exhausted
        if !page_view.can_patch(page_index) {
            self.rebase_page_view(page_view)?;
            return Ok(());
        }

        // clone just the touched page into one fresh single-page run
        let new_run = self.allocate_run(1)?;
        let Some(old_page_id) = slot.run.page(slot.run_page_index) else {
            return Err(HeapError::InvalidPatchedRun {
                page_index,
                page_count: slot.run.len(),
            });
        };
        let Some(new_page_id) = new_run.page(0) else {
            return Err(HeapError::InvalidPatchedRun {
                page_index,
                page_count: new_run.len(),
            });
        };
        let source_bytes = self.read_page_bytes(old_page_id)?;
        let page_ptr = self.page_slice_mut_ptr(new_page_id)?;
        let page = unsafe { &mut *page_ptr };

        page.copy_from_slice(&source_bytes);

        // release any previous patched run once it is replaced
        if slot.is_patched {
            self.release_run(slot.run)?;
        }

        page_view.set_patch(page_index, new_page_id)
    }

    /// Ensure every touched run is unique before one write.
    fn detach_shared_write_range(
        &self,
        page_view: &mut PageView,
        start_page: usize,
        end_page: usize,
    ) -> HeapResult<()> {
        let mut touches_base_run = false;
        let mut touched_base_pages = 0usize;

        // preflight the write window first
        for page_index in start_page..end_page {
            let Some(slot) = page_view.slot(page_index) else {
                return Err(HeapError::MissingLogicalPage { page_index });
            };

            if !slot.is_patched {
                touches_base_run = true;
                touched_base_pages += 1;
            }
        }

        let base_run_is_unique = if touches_base_run {
            self.run_is_unique(page_view.base_run())?
        } else {
            true
        };

        // rebase once when this write would outgrow inline patches anyway
        if !base_run_is_unique && touched_base_pages > page_view.patch_room() {
            self.rebase_page_view(page_view)?;

            return Ok(());
        }

        // detach only the shared runs in the write window
        for page_index in start_page..end_page {
            let Some(slot) = page_view.slot(page_index) else {
                return Err(HeapError::MissingLogicalPage { page_index });
            };

            let run_is_unique = if slot.is_patched {
                self.run_is_unique(slot.run)?
            } else {
                base_run_is_unique
            };

            if !run_is_unique {
                self.detach_page_for_write(page_view, page_index, slot)?;
            }
        }

        Ok(())
    }

    /// Copy one byte slice into the touched logical pages.
    fn copy_bytes_into_pages(
        &self,
        page_view: &PageView,
        start_page: usize,
        end_page: usize,
        start: usize,
        end: usize,
        source: &[u8],
    ) -> HeapResult<()> {
        let page_bytes = self.page_bytes();
        let mut source_offset = 0usize;

        // write each touched page-local slice in order
        for page_index in start_page..end_page {
            // resolve the destination page first
            let Some(page_id) = page_view.page(page_index) else {
                return Err(HeapError::MissingLogicalPage { page_index });
            };
            // resolve the page-local visible slice
            let page_start = page_index * page_bytes;
            let slice_start = start.saturating_sub(page_start).min(page_bytes);
            let slice_end = end.saturating_sub(page_start).min(page_bytes);
            if slice_start >= slice_end {
                continue;
            }

            // copy the next logical source slice into this page
            let slice_len = slice_end - slice_start;
            let source_end = source_offset + slice_len;
            let page_ptr = self.page_slice_mut_ptr(page_id)?;
            let page = unsafe { &mut *page_ptr };

            page[slice_start..slice_end].copy_from_slice(&source[source_offset..source_end]);
            source_offset = source_end;
        }

        Ok(())
    }

    /// Rebase one logical page view into one fresh contiguous run.
    fn rebase_page_view(&self, page_view: &mut PageView) -> HeapResult<()> {
        if page_view.is_empty() {
            return Ok(());
        }

        // allocate one fresh contiguous base run first
        let old_page_view = *page_view;
        let rebased_run = self.allocate_run(old_page_view.len())?;
        let rebased_page_view = PageView::from_run(rebased_run);

        // materialize the current logical view into that new run
        for page_index in 0..old_page_view.len() {
            self.copy_page_between_views(&old_page_view, &rebased_page_view, page_index)?;
        }

        // then drop the old sharing state
        self.release_page_view(&old_page_view)?;
        *page_view = rebased_page_view;

        Ok(())
    }

    /// Copy one logical page from one page view into another.
    fn copy_page_between_views(
        &self,
        source: &PageView,
        target: &PageView,
        page_index: usize,
    ) -> HeapResult<()> {
        let Some(source_page_id) = source.page(page_index) else {
            return Err(HeapError::MissingLogicalPage { page_index });
        };
        let Some(target_page_id) = target.page(page_index) else {
            return Err(HeapError::MissingLogicalPage { page_index });
        };

        self.copy_page_between_ids(source_page_id, target_page_id)
    }

    /// Copy one physical page into another physical page.
    fn copy_page_between_ids(
        &self,
        source_page_id: PageId,
        target_page_id: PageId,
    ) -> HeapResult<()> {
        let source_page = self.read_page_bytes(source_page_id)?;
        let page_ptr = self.page_slice_mut_ptr(target_page_id)?;
        let page = unsafe { &mut *page_ptr };

        page.copy_from_slice(&source_page);

        Ok(())
    }
}
