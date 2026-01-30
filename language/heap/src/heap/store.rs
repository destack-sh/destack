use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{ManagedHeap, RawHeap};

/// Shared heap storage for managed and raw allocations.
#[derive(Debug, Default)]
pub struct HeapStore {
    /// Managed heap used for GC-tracked allocations.
    pub managed: ManagedHeap,
    /// Raw heap used for manual allocations.
    pub raw: RawHeap,
}

impl HeapStore {
    /// Create a heap store from explicit heap instances.
    pub fn new(managed: ManagedHeap, raw: RawHeap) -> Self {
        Self { managed, raw }
    }

    /// Wrap this heap store in a shared handle.
    pub fn shared(self) -> SharedHeap {
        SharedHeap::new(self)
    }

    /// Create a shared heap store with default heaps.
    pub fn shared_default() -> SharedHeap {
        Self::default().shared()
    }
}

/// Shared heap handle used across runtime and VM.
#[derive(Clone, Debug)]
pub struct SharedHeap(Arc<HeapCell>);

impl SharedHeap {
    /// Create a new shared heap from a heap store.
    pub fn new(store: HeapStore) -> Self {
        Self(Arc::new(HeapCell::new(store)))
    }

    /// Borrow the heap store exclusively.
    pub fn borrow(&self) -> HeapBorrow<'_> {
        self.0.borrow()
    }

    /// Attempt to borrow the heap store exclusively.
    pub fn try_borrow(&self) -> Option<HeapBorrow<'_>> {
        self.0.try_borrow()
    }
}

impl Default for SharedHeap {
    fn default() -> Self {
        Self::new(HeapStore::default())
    }
}

/// Heap storage plus a lightweight borrow gate.
#[derive(Debug)]
struct HeapCell {
    store: UnsafeCell<HeapStore>,
    borrow_state: AtomicUsize,
}

impl HeapCell {
    /// Create a heap cell from a heap store.
    fn new(store: HeapStore) -> Self {
        Self {
            store: UnsafeCell::new(store),
            borrow_state: AtomicUsize::new(0),
        }
    }

    /// Borrow the heap store exclusively.
    fn borrow(&self) -> HeapBorrow<'_> {
        if self
            .borrow_state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("heap store is already borrowed");
        }

        HeapBorrow {
            cell: self,
            _not_send: PhantomData,
        }
    }

    /// Attempt to borrow the heap store exclusively.
    fn try_borrow(&self) -> Option<HeapBorrow<'_>> {
        if self
            .borrow_state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return None;
        }

        Some(HeapBorrow {
            cell: self,
            _not_send: PhantomData,
        })
    }

    /// Release the heap store borrow.
    fn release(&self) {
        self.borrow_state.store(0, Ordering::Release);
    }
}

/// Exclusive heap store borrow.
#[derive(Debug)]
pub struct HeapBorrow<'a> {
    cell: &'a HeapCell,
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for HeapBorrow<'_> {
    fn drop(&mut self) {
        self.cell.release();
    }
}

impl Deref for HeapBorrow<'_> {
    type Target = HeapStore;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.store.get() }
    }
}

impl DerefMut for HeapBorrow<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.cell.store.get() }
    }
}

// safety: heap access is serialized through borrow_state
unsafe impl Send for HeapCell {}
// safety: heap access is serialized through borrow_state
unsafe impl Sync for HeapCell {}
