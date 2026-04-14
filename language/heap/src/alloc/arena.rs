use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::collections::BTreeMap;
use std::process::abort;
use std::ptr::{copy_nonoverlapping, null_mut};
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use super::{PageId, PageMap, PageRun, PageSlot, Segment};
use crate::{DEFAULT_ARENA_SEGMENT_BYTES, DEFAULT_PAGE_BYTES, MAX_ARENA_SEGMENTS};

/// One serialized arena page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaPage {
    /// The page identifier inside the serialized arena.
    pub id: PageId,
    /// The exact bytes for this page.
    pub bytes: Box<[u8]>,
}

/// One serialized arena snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaSnapshot {
    /// The fixed page width for the snapshot.
    pub page_bytes: u32,
    /// The serialized page leaves reachable from one frozen root.
    pub pages: Box<[ArenaPage]>,
}

/// One branchable arena of fixed-width pages.
#[derive(Debug)]
pub struct Arena {
    /// The fixed page width for every page.
    page_bytes: u32,
    /// The number of pages stored in each arena segment.
    pages_per_segment: u32,
    /// The number of reserved arena segments.
    segment_count: AtomicUsize,
    /// The append-only arena segment directory.
    segments: Box<[AtomicPtr<Segment>]>,
}

impl Default for Arena {
    fn default() -> Self {
        Self::with_page_bytes(DEFAULT_PAGE_BYTES)
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        let segment_count = self.segment_count.load(Ordering::Acquire);
        for segment_index in 0..segment_count {
            let Some(slot) = self.segments.get(segment_index) else {
                continue;
            };
            let segment_ptr = slot.load(Ordering::Acquire);
            if segment_ptr.is_null() {
                continue;
            }

            unsafe {
                drop(Box::from_raw(segment_ptr));
            }
        }
    }
}

impl Arena {
    /// Create one empty arena with the given page width.
    pub fn with_page_bytes(page_bytes: usize) -> Self {
        let page_bytes = page_bytes.max(1);
        let pages_per_segment = (DEFAULT_ARENA_SEGMENT_BYTES / page_bytes).max(1) as u32;

        Self {
            page_bytes: page_bytes as u32,
            pages_per_segment,
            segment_count: AtomicUsize::new(0),
            segments: std::iter::repeat_with(|| AtomicPtr::new(null_mut()))
                .take(MAX_ARENA_SEGMENTS)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    /// Restore one arena directly from one serialized snapshot.
    pub fn from_snapshot(snapshot: &ArenaSnapshot) -> Self {
        let arena = Self::with_page_bytes(snapshot.page_bytes as usize);
        let page_count = snapshot
            .pages
            .iter()
            .map(|page| page.id.index().saturating_add(1))
            .max()
            .unwrap_or(0);
        let mut segment_high_watermarks = BTreeMap::<usize, usize>::new();

        // publish enough segment capacity first
        arena.ensure_page_capacity(page_count);

        // materialize every serialized page into the arena
        for page in &snapshot.pages {
            let Some(target_page) = arena.page_bytes_mut(page.id) else {
                continue;
            };

            target_page.copy_from_slice(&page.bytes);

            let (segment_index, segment_page_index) = arena.page_position(page.id);
            let segment_high_watermark = segment_high_watermarks.entry(segment_index).or_default();
            *segment_high_watermark =
                (*segment_high_watermark).max(segment_page_index.saturating_add(1));
        }

        // restore the per-segment fresh-allocation cursors
        for (segment_index, high_watermark) in segment_high_watermarks {
            let Some(segment) = arena.segment(segment_index) else {
                continue;
            };

            segment
                .next_unused_page
                .fetch_max(high_watermark as u32, Ordering::AcqRel);
        }

        arena
    }

    /// Return the fixed page width.
    pub const fn page_bytes(&self) -> usize {
        self.page_bytes as usize
    }

    /// Allocate zeroed pages for one byte length.
    pub fn allocate_zeroed(&self, byte_len: usize) -> PageMap {
        let page_count = self.page_count(byte_len);

        PageMap::from_run(self.allocate_run(page_count))
    }

    /// Allocate pages and copy one byte slice into them.
    pub fn allocate_bytes(&self, bytes: &[u8]) -> PageMap {
        let mut page_map = self.allocate_zeroed(bytes.len());
        let is_written = self.set_bytes(&mut page_map, 0, bytes);

        debug_assert!(is_written);

        page_map
    }

    /// Retain one logical page map for another live root.
    pub fn retain_pages(&self, page_map: &PageMap) {
        if page_map.has_base_pages() {
            self.retain_run(page_map.base_run());
        }

        for patch in page_map.patches() {
            self.retain_run(patch.run);
        }
    }

    /// Return one cloned page map retained for another live root.
    pub fn clone_pages(&self, page_map: &PageMap) -> PageMap {
        self.retain_pages(page_map);
        *page_map
    }

    /// Release one logical page map after one root drops it.
    pub fn release_pages(&self, page_map: &PageMap) {
        if page_map.has_base_pages() {
            self.release_run(page_map.base_run());
        }

        for patch in page_map.patches() {
            self.release_run(patch.run);
        }
    }

    /// Snapshot one page map into serialized arena pages.
    pub fn snapshot_pages(&self, page_map: &PageMap) -> Box<[ArenaPage]> {
        page_map
            .page_ids()
            .map(|page| ArenaPage {
                id: page,
                bytes: self
                    .page_bytes_from_id(page)
                    .map(|bytes| bytes.to_vec().into_boxed_slice())
                    .unwrap_or_else(|| vec![0; self.page_bytes()].into_boxed_slice()),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Snapshot one arbitrary page-id set into serialized arena pages.
    pub fn snapshot_pages_from_ids(&self, pages: &[PageId]) -> ArenaSnapshot {
        ArenaSnapshot {
            page_bytes: self.page_bytes,
            pages: pages
                .iter()
                .copied()
                .map(|page| ArenaPage {
                    id: page,
                    bytes: self
                        .page_bytes_from_id(page)
                        .map(|bytes| bytes.to_vec().into_boxed_slice())
                        .unwrap_or_else(|| vec![0; self.page_bytes()].into_boxed_slice()),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    /// Return one byte vector for one logical byte range over one page map.
    pub fn bytes_to_vec(&self, page_map: &PageMap, byte_len: usize) -> Vec<u8> {
        self.bytes_to_vec_from(page_map, 0, byte_len)
    }

    /// Return one byte vector for one logical byte range starting at one offset.
    pub fn bytes_to_vec_from(&self, page_map: &PageMap, start: usize, byte_len: usize) -> Vec<u8> {
        let capacity = page_map.len().saturating_mul(self.page_bytes());

        // clamp the requested range to the logical byte capacity
        if start >= capacity || byte_len == 0 {
            return Vec::new();
        }

        let read_len = byte_len.min(capacity.saturating_sub(start));
        let mut bytes = Vec::with_capacity(read_len);
        let page_bytes = self.page_bytes();
        let start_page = start / page_bytes;
        let end = start.saturating_add(read_len);
        let end_page = end.div_ceil(page_bytes);

        // gather the visible bytes page by page
        for page_index in start_page..end_page {
            let Some(page_id) = page_map.page(page_index) else {
                break;
            };
            let Some(page) = self.page_bytes_from_id(page_id) else {
                break;
            };
            let page_start = page_index.saturating_mul(page_bytes);
            let slice_start = start.saturating_sub(page_start).min(page_bytes);
            let slice_end = end.saturating_sub(page_start).min(page_bytes);

            if slice_start >= slice_end {
                continue;
            }

            bytes.extend_from_slice(&page[slice_start..slice_end]);
        }

        bytes
    }

    /// Return one byte for one logical byte offset.
    pub fn byte_at(&self, page_map: &PageMap, byte_len: usize, index: usize) -> Option<u8> {
        if index >= byte_len {
            return None;
        }

        let page_bytes = self.page_bytes();
        let page_index = index / page_bytes;
        let byte_index = index % page_bytes;
        let page = page_map.page(page_index)?;
        let page = self.page_bytes_from_id(page)?;

        page.get(byte_index).copied()
    }

    /// Overwrite one logical byte range inside one page map.
    pub fn set_bytes(&self, page_map: &mut PageMap, start: usize, source: &[u8]) -> bool {
        let end = start.saturating_add(source.len());
        let capacity = page_map.len().saturating_mul(self.page_bytes());

        // reject writes that extend past the logical byte capacity
        if end > capacity {
            return false;
        }

        let page_bytes = self.page_bytes();
        let start_page = start / page_bytes;
        let end_page = end.saturating_add(page_bytes.saturating_sub(1)) / page_bytes;

        // detach any shared pages before mutating them
        if !self.detach_shared_write_pages(page_map, start_page, end_page) {
            return false;
        }

        // copy the source bytes into each touched page slice
        self.copy_bytes_into_pages(page_map, start_page, end_page, start, end, source)
    }

    /// Replace one logical page map with one new byte slice.
    pub fn replace_bytes(&self, page_map: &mut PageMap, bytes: &[u8]) {
        self.release_pages(page_map);
        *page_map = self.allocate_bytes(bytes);
    }

    /// Return the number of pages required for one byte length.
    pub fn page_count(&self, byte_len: usize) -> usize {
        if byte_len == 0 {
            return 0;
        }

        byte_len.div_ceil(self.page_bytes())
    }

    /// Return one serialized arena snapshot for one reachable page map.
    pub fn snapshot(&self, page_map: &PageMap) -> ArenaSnapshot {
        ArenaSnapshot {
            page_bytes: self.page_bytes,
            pages: self.snapshot_pages(page_map),
        }
    }

    /// Allocate one zeroed physical run.
    fn allocate_run(&self, page_count: usize) -> PageRun {
        if page_count == 0 {
            return PageRun::empty();
        }

        // reuse an existing run when possible
        if let Some(run) = self.take_reusable_run(page_count) {
            self.zero_run(run);
            self.run_refcount(run).store(1, Ordering::Release);

            return run;
        }

        // otherwise carve a new single-segment run
        if page_count <= self.pages_per_segment() {
            if let Some(run) = self.take_fresh_single_segment_run(page_count) {
                self.run_refcount(run).store(1, Ordering::Release);

                return run;
            }
        }

        // fall back to one fresh multi-segment run
        let run = self.allocate_fresh_multi_segment_run(page_count);
        self.run_refcount(run).store(1, Ordering::Release);

        run
    }

    /// Retain one physical run.
    fn retain_run(&self, run: PageRun) {
        if run.is_empty() {
            return;
        }

        self.run_refcount(run).fetch_add(1, Ordering::AcqRel);
    }

    /// Release one physical run.
    fn release_run(&self, run: PageRun) {
        if run.is_empty() {
            return;
        }

        // stop once another root still retains the run
        let count = self.run_refcount(run).fetch_sub(1, Ordering::AcqRel);
        if count != 1 {
            return;
        }

        // otherwise return each physical segment to its owning segment
        for run in self.segment_runs(run) {
            self.release_segment_run(run);
        }
    }

    /// Report whether one physical run is shared.
    fn run_is_shared(&self, run: PageRun) -> bool {
        if run.is_empty() {
            return false;
        }

        self.run_refcount(run).load(Ordering::Acquire) > 1
    }

    /// Ensure one logical page is unique before mutation.
    fn detach_page_for_write(&self, page_map: &mut PageMap, page_index: usize, slot: PageSlot) {
        if page_map.is_empty() {
            return;
        }

        // collapse back to one contiguous run when patches are exhausted
        if !page_map.can_patch(page_index) {
            self.rebase_page_map(page_map);
            return;
        }

        // clone just the touched page into one fresh single-page run
        let new_run = self.allocate_run(1);
        let Some(old_page_id) = slot.run.page(slot.run_page_index) else {
            return;
        };
        let Some(new_page_id) = new_run.page(0) else {
            return;
        };
        let Some(source_bytes) = self.page_bytes_from_id(old_page_id) else {
            return;
        };
        let Some(target_bytes) = self.page_bytes_mut(new_page_id) else {
            return;
        };

        unsafe {
            copy_nonoverlapping(
                source_bytes.as_ptr(),
                target_bytes.as_mut_ptr(),
                self.page_bytes(),
            );
        }

        // release any previous patched run once it is replaced
        if slot.is_patched {
            self.release_run(slot.run);
        }

        let is_installed = page_map.set_patch(page_index, new_run);

        debug_assert!(is_installed);
    }

    /// Ensure every touched page is unique before one write.
    fn detach_shared_write_pages(
        &self,
        page_map: &mut PageMap,
        start_page: usize,
        end_page: usize,
    ) -> bool {
        for page_index in start_page..end_page {
            let Some(slot) = page_map.slot(page_index) else {
                return false;
            };

            if self.run_is_shared(slot.run) {
                self.detach_page_for_write(page_map, page_index, slot);
            }
        }

        true
    }

    /// Copy one byte slice into the touched logical pages.
    fn copy_bytes_into_pages(
        &self,
        page_map: &PageMap,
        start_page: usize,
        end_page: usize,
        start: usize,
        end: usize,
        source: &[u8],
    ) -> bool {
        let page_bytes = self.page_bytes();
        let mut source_offset = 0usize;

        for page_index in start_page..end_page {
            // resolve the target page first
            let Some(page_id) = page_map.page(page_index) else {
                return false;
            };
            let Some(page) = self.page_bytes_mut(page_id) else {
                return false;
            };

            // compute the page-local write window
            let page_start = page_index.saturating_mul(page_bytes);
            let slice_start = start.saturating_sub(page_start).min(page_bytes);
            let slice_end = end.saturating_sub(page_start).min(page_bytes);

            if slice_start >= slice_end {
                continue;
            }

            // copy the matching source slice into this page
            let slice_len = slice_end.saturating_sub(slice_start);
            let source_end = source_offset.saturating_add(slice_len);

            page[slice_start..slice_end].copy_from_slice(&source[source_offset..source_end]);
            source_offset = source_end;
        }

        true
    }

    /// Rebase one logical page map into one fresh contiguous run.
    fn rebase_page_map(&self, page_map: &mut PageMap) {
        if page_map.is_empty() {
            return;
        }

        // allocate one fresh contiguous base run first
        let old_page_map = *page_map;
        let rebased_run = self.allocate_run(old_page_map.len());
        let rebased_page_map = PageMap::from_run(rebased_run);

        // materialize the current logical view into that new run
        for page_index in 0..old_page_map.len() {
            self.copy_page_between_maps(&old_page_map, &rebased_page_map, page_index);
        }

        // then drop the old sharing state
        self.release_pages(&old_page_map);
        *page_map = rebased_page_map;
    }

    /// Copy one logical page from one page map into another.
    fn copy_page_between_maps(&self, source: &PageMap, target: &PageMap, page_index: usize) {
        let Some(source_page_id) = source.page(page_index) else {
            return;
        };
        let Some(target_page_id) = target.page(page_index) else {
            return;
        };

        self.copy_page_between_ids(source_page_id, target_page_id);
    }

    /// Copy one physical page into another physical page.
    fn copy_page_between_ids(&self, source_page_id: PageId, target_page_id: PageId) {
        let Some(source_page) = self.page_bytes_from_id(source_page_id) else {
            return;
        };
        let Some(target_page) = self.page_bytes_mut(target_page_id) else {
            return;
        };

        unsafe {
            copy_nonoverlapping(
                source_page.as_ptr(),
                target_page.as_mut_ptr(),
                self.page_bytes(),
            );
        }
    }

    /// Ensure the arena can address the given number of pages.
    fn ensure_page_capacity(&self, page_count: usize) {
        let required_segments = page_count.div_ceil(self.pages_per_segment());
        let current_segments = self
            .segment_count
            .fetch_max(required_segments, Ordering::AcqRel);
        if required_segments <= current_segments {
            return;
        }

        for segment_index in current_segments..required_segments {
            self.publish_segment(segment_index);
        }
    }

    /// Zero one full run before reuse.
    fn zero_run(&self, run: PageRun) {
        for page_id in run.page_ids() {
            let Some(page) = self.page_bytes_mut(page_id) else {
                continue;
            };

            page.fill(0);
        }
    }

    /// Release one reusable run segment back to its owning segment.
    fn release_segment_run(&self, run: PageRun) {
        let (segment_index, _) = self.page_position(run.first_page);
        let segment = self.publish_segment(segment_index);
        let mut free_runs = match segment.free_runs.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        free_runs.entry(run.page_count).or_default().push(run);
    }

    /// Return the per-segment runs that physically cover one run.
    fn segment_runs(&self, run: PageRun) -> Vec<PageRun> {
        let mut segment_runs = Vec::new();
        let mut first_page = run.first_page.index();
        let mut remaining_pages = run.len();

        while remaining_pages > 0 {
            let segment_page_index = first_page % self.pages_per_segment();
            let segment_remaining_pages =
                self.pages_per_segment().saturating_sub(segment_page_index);
            let segment_pages = remaining_pages.min(segment_remaining_pages);

            segment_runs.push(PageRun::new(PageId::new(first_page), segment_pages));
            first_page = first_page.saturating_add(segment_pages);
            remaining_pages = remaining_pages.saturating_sub(segment_pages);
        }

        segment_runs
    }

    /// Return one reusable run large enough for the requested size.
    fn take_reusable_run(&self, page_count: usize) -> Option<PageRun> {
        let segment_count = self.segment_count.load(Ordering::Acquire);

        // search each published segment for one reusable run
        for segment_index in 0..segment_count {
            let Some(segment) = self.segment(segment_index) else {
                continue;
            };
            let mut free_runs = match segment.free_runs.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let mut reusable_entry = free_runs
                .range_mut(page_count as u32..)
                .find_map(|(&run_len, runs)| runs.pop().map(|run| (run_len as usize, run)));
            let Some((run_len, run)) = reusable_entry.take() else {
                continue;
            };

            // drop empty buckets after one successful pop
            free_runs.retain(|_, runs| !runs.is_empty());

            let allocation = PageRun::new(run.first_page, page_count);

            // return any remainder to the same segment bucket
            if run_len > page_count {
                let remainder = PageRun::new(
                    PageId::new(run.first_page.index().saturating_add(page_count)),
                    run_len.saturating_sub(page_count),
                );

                free_runs
                    .entry(remainder.page_count)
                    .or_default()
                    .push(remainder);
            }

            return Some(allocation);
        }

        None
    }

    /// Return one fresh run that fits inside one existing or new segment.
    fn take_fresh_single_segment_run(&self, page_count: usize) -> Option<PageRun> {
        let segment_count = self.segment_count.load(Ordering::Acquire);

        // first try already published segments
        for segment_index in 0..segment_count {
            let segment = self.publish_segment(segment_index);
            let Some(run) = self.try_allocate_segment_run(segment, segment_index, page_count)
            else {
                continue;
            };

            return Some(run);
        }

        // otherwise publish one new segment and allocate from it
        let segment_index = self.reserve_segment_range(1);
        let segment = self.publish_segment(segment_index);

        self.try_allocate_segment_run(segment, segment_index, page_count)
    }

    /// Allocate one fresh run that may span multiple new segments.
    fn allocate_fresh_multi_segment_run(&self, page_count: usize) -> PageRun {
        let required_segments = page_count.div_ceil(self.pages_per_segment());
        let first_segment_index = self.reserve_segment_range(required_segments);
        let last_segment_len = page_count % self.pages_per_segment();

        // mark each newly reserved segment run as consumed
        for segment_offset in 0..required_segments {
            let segment_index = first_segment_index.saturating_add(segment_offset);
            let segment = self.publish_segment(segment_index);
            let used_pages = if segment_offset + 1 == required_segments && last_segment_len != 0 {
                last_segment_len
            } else {
                self.pages_per_segment()
            };

            segment
                .next_unused_page
                .store(used_pages as u32, Ordering::Release);
        }

        PageRun::new(
            PageId::new(first_segment_index.saturating_mul(self.pages_per_segment())),
            page_count,
        )
    }

    /// Return one newly published segment range start.
    fn reserve_segment_range(&self, segment_count: usize) -> usize {
        let first_segment_index = self
            .segment_count
            .fetch_add(segment_count, Ordering::AcqRel);
        let end_segment_index = first_segment_index.saturating_add(segment_count);

        if end_segment_index > MAX_ARENA_SEGMENTS {
            fail_arena_segment_directory_exhausted();
        }

        first_segment_index
    }

    /// Publish one segment and return it.
    fn publish_segment(&self, segment_index: usize) -> &Segment {
        let Some(slot) = self.segments.get(segment_index) else {
            fail_arena_segment_directory_exhausted();
        };

        // fast path: reuse one already published segment
        let mut segment_ptr = slot.load(Ordering::Acquire);
        if segment_ptr.is_null() {
            let segment = Box::new(Segment::zeroed(
                self.segment_bytes(),
                self.pages_per_segment(),
            ));
            let candidate_ptr = Box::into_raw(segment);
            let install_result = slot.compare_exchange(
                null_mut(),
                candidate_ptr,
                Ordering::AcqRel,
                Ordering::Acquire,
            );

            // install the new segment or adopt the competing publication
            match install_result {
                Ok(..) => {
                    segment_ptr = candidate_ptr;
                }
                Err(existing_ptr) => {
                    unsafe {
                        drop(Box::from_raw(candidate_ptr));
                    }

                    segment_ptr = existing_ptr;
                }
            }
        }

        unsafe { &*segment_ptr }
    }

    /// Allocate one fresh run from one specific segment.
    fn try_allocate_segment_run(
        &self,
        segment: &Segment,
        segment_index: usize,
        page_count: usize,
    ) -> Option<PageRun> {
        loop {
            // read the current fresh-allocation cursor
            let start_page = segment.next_unused_page.load(Ordering::Acquire) as usize;
            let end_page = start_page.saturating_add(page_count);

            // stop once this segment is exhausted
            if end_page > self.pages_per_segment() {
                return None;
            }

            // advance the cursor only if nobody raced us
            let exchange_result = segment.next_unused_page.compare_exchange(
                start_page as u32,
                end_page as u32,
                Ordering::AcqRel,
                Ordering::Acquire,
            );

            if exchange_result.is_err() {
                continue;
            }

            let first_page = segment_index
                .saturating_mul(self.pages_per_segment())
                .saturating_add(start_page);

            return Some(PageRun::new(PageId::new(first_page), page_count));
        }
    }

    /// Return the atomic refcount for one run start.
    fn run_refcount(&self, run: PageRun) -> &AtomicU32 {
        let last_page = run.first_page.index().saturating_add(run.len());
        self.ensure_page_capacity(last_page);

        let (segment_index, segment_page_index) = self.page_position(run.first_page);
        let segment = self.publish_segment(segment_index);
        let Some(refcount) = segment.run_refcounts.get(segment_page_index) else {
            fail_missing_run_refcount_slot();
        };

        refcount
    }

    /// Return one immutable page slice for one page id.
    fn page_bytes_from_id(&self, page_id: PageId) -> Option<&[u8]> {
        self.ensure_page_capacity(page_id.index().saturating_add(1));

        let (segment_index, segment_page_index) = self.page_position(page_id);
        let segment = self.publish_segment(segment_index);

        Some(segment.page(segment_page_index, self.page_bytes()))
    }

    /// Return one mutable page slice for one page id.
    fn page_bytes_mut(&self, page_id: PageId) -> Option<&mut [u8]> {
        self.ensure_page_capacity(page_id.index().saturating_add(1));

        let (segment_index, segment_page_index) = self.page_position(page_id);
        let segment = self.publish_segment(segment_index);

        Some(segment.page_mut(segment_page_index, self.page_bytes()))
    }

    /// Return one published arena segment by segment index.
    fn segment(&self, segment_index: usize) -> Option<&Segment> {
        let slot = self.segments.get(segment_index)?;
        let segment_ptr = slot.load(Ordering::Acquire);

        if segment_ptr.is_null() {
            return None;
        }

        Some(unsafe { &*segment_ptr })
    }

    /// Return the segment and page index for one page id.
    fn page_position(&self, page_id: PageId) -> (usize, usize) {
        let pages_per_segment = self.pages_per_segment();
        let page_index = page_id.index();
        let segment_index = page_index / pages_per_segment;
        let segment_page_index = page_index % pages_per_segment;

        (segment_index, segment_page_index)
    }

    /// Return the number of pages stored in each segment.
    const fn pages_per_segment(&self) -> usize {
        self.pages_per_segment as usize
    }

    /// Return the byte width for one full segment.
    fn segment_bytes(&self) -> usize {
        self.pages_per_segment().saturating_mul(self.page_bytes())
    }
}

/// Allocate one zeroed page-aligned segment for arena page storage.
pub fn allocate_page_segment_bytes(byte_len: usize) -> *mut u8 {
    let layout = page_segment_layout(byte_len);

    unsafe { alloc_zeroed(layout) }
}

/// Free one page-aligned segment previously allocated by the arena.
pub fn free_page_segment_bytes(data: *mut u8, byte_len: usize) {
    if data.is_null() {
        return;
    }

    let layout = page_segment_layout(byte_len);

    unsafe {
        dealloc(data, layout);
    }
}

/// Return one page-aligned allocation layout for one segment.
fn page_segment_layout(byte_len: usize) -> Layout {
    let byte_len = byte_len.max(1);

    unsafe { Layout::from_size_align_unchecked(byte_len, DEFAULT_PAGE_BYTES) }
}

/// Abort when the arena segment directory cannot grow further.
#[cold]
fn fail_arena_segment_directory_exhausted() -> ! {
    abort()
}

/// Abort when one run refcount slot is unexpectedly absent.
#[cold]
fn fail_missing_run_refcount_slot() -> ! {
    abort()
}
