use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

/// Allocator that records gross and retained Rust allocation cost.
#[derive(Debug)]
pub(crate) struct CountingAllocator;

/// One allocator sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AllocationSample {
    /// The number of successful allocation calls.
    pub(crate) allocations: u64,
    /// The gross allocated bytes.
    pub(crate) allocated_bytes: u64,
    /// The bytes still live at the sample point.
    pub(crate) live_bytes: i64,
    /// The peak live bytes since the last reset.
    pub(crate) peak_live_bytes: i64,
}

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static IS_ENABLED: AtomicBool = AtomicBool::new(false);

impl CountingAllocator {
    /// Reset allocation counters.
    pub(crate) fn reset(&self) {
        ALLOCATIONS.store(0, Ordering::Relaxed);
        ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        LIVE_BYTES.store(0, Ordering::Relaxed);
        PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);
    }

    /// Return the current allocation sample.
    pub(crate) fn sample(&self) -> AllocationSample {
        AllocationSample {
            allocations: ALLOCATIONS.load(Ordering::Relaxed),
            allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
            live_bytes: LIVE_BYTES.load(Ordering::Relaxed),
            peak_live_bytes: PEAK_LIVE_BYTES.load(Ordering::Relaxed),
        }
    }

    /// Measure one operation and sample while the result is still live.
    pub(crate) fn measure<T>(&self, run: impl FnOnce() -> T) -> AllocationSample {
        let (value, sample) = self.capture(run);
        drop(value);

        sample
    }

    /// Measure one operation and return the live result with its sample.
    pub(crate) fn capture<T>(&self, run: impl FnOnce() -> T) -> (T, AllocationSample) {
        self.reset();
        IS_ENABLED.store(true, Ordering::Release);
        let value = run();
        std::hint::black_box(&value);
        let sample = self.sample();
        IS_ENABLED.store(false, Ordering::Release);

        (value, sample)
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: system allocator receives the original layout unchanged
        let pointer = unsafe { System.alloc(layout) };
        if IS_ENABLED.load(Ordering::Acquire) && !pointer.is_null() {
            record_alloc(layout.size());
        }

        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: system allocator receives the original layout unchanged
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if IS_ENABLED.load(Ordering::Acquire) && !pointer.is_null() {
            record_alloc(layout.size());
        }

        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if IS_ENABLED.load(Ordering::Acquire) {
            record_dealloc(layout.size());
        }

        // SAFETY: system allocator receives the pointer and layout from the caller
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: system allocator receives the pointer, layout, and target size unchanged
        let next = unsafe { System.realloc(pointer, layout, new_size) };
        if IS_ENABLED.load(Ordering::Acquire) && !next.is_null() {
            record_dealloc(layout.size());
            record_alloc(new_size);
        }

        next
    }
}

/// Record one successful allocation.
fn record_alloc(byte_len: usize) {
    let byte_len = byte_len as u64;
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    ALLOCATED_BYTES.fetch_add(byte_len, Ordering::Relaxed);
    let live_bytes = LIVE_BYTES.fetch_add(byte_len as i64, Ordering::Relaxed) + byte_len as i64;
    record_peak(live_bytes);
}

/// Record one deallocation.
fn record_dealloc(byte_len: usize) {
    LIVE_BYTES.fetch_sub(byte_len as i64, Ordering::Relaxed);
}

/// Record one live-byte peak.
fn record_peak(value: i64) {
    let mut peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
    while value > peak {
        match PEAK_LIVE_BYTES.compare_exchange_weak(
            peak,
            value,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(next_peak) => peak = next_peak,
        }
    }
}
