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

    /// Borrow the heap store for read-only access.
    pub fn borrow_read(&self) -> HeapReadBorrow<'_> {
        self.0.borrow_read()
    }

    /// Attempt to borrow the heap store for read-only access.
    pub fn try_borrow_read(&self) -> Option<HeapReadBorrow<'_>> {
        self.0.try_borrow_read()
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
    const WRITE_BIT: usize = 1 << (usize::BITS as usize - 1);
    const READ_MASK: usize = Self::WRITE_BIT - 1;

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
            .compare_exchange(0, Self::WRITE_BIT, Ordering::Acquire, Ordering::Relaxed)
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
            .compare_exchange(0, Self::WRITE_BIT, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return None;
        }

        Some(HeapBorrow {
            cell: self,
            _not_send: PhantomData,
        })
    }

    /// Borrow the heap store for read-only access.
    fn borrow_read(&self) -> HeapReadBorrow<'_> {
        self.try_borrow_read()
            .unwrap_or_else(|| panic!("heap store is already borrowed for write"))
    }

    /// Attempt to borrow the heap store for read-only access.
    fn try_borrow_read(&self) -> Option<HeapReadBorrow<'_>> {
        loop {
            // load the current borrow state
            let state = self.borrow_state.load(Ordering::Acquire);
            if state & Self::WRITE_BIT != 0 {
                return None;
            }

            // reserve a reader slot
            let readers = state & Self::READ_MASK;
            if readers == Self::READ_MASK {
                panic!("heap store reader count overflow");
            }

            // install the updated reader count
            let next = state + 1;
            if self
                .borrow_state
                .compare_exchange(state, next, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Some(HeapReadBorrow {
                    cell: self,
                    _not_send: PhantomData,
                });
            }
        }
    }

    /// Release the heap store write borrow.
    fn release_write(&self) {
        self.borrow_state.store(0, Ordering::Release);
    }

    /// Release a heap store read borrow.
    fn release_read(&self) {
        // drop the shared borrow count
        let previous = self.borrow_state.fetch_sub(1, Ordering::Release);
        debug_assert!(
            previous & Self::WRITE_BIT == 0,
            "read borrow released while write lock is held"
        );
        debug_assert!(previous & Self::READ_MASK != 0, "read borrow underflow");
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
        self.cell.release_write();
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

/// Shared heap store borrow for read-only access.
#[derive(Debug)]
pub struct HeapReadBorrow<'a> {
    cell: &'a HeapCell,
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for HeapReadBorrow<'_> {
    fn drop(&mut self) {
        self.cell.release_read();
    }
}

impl Deref for HeapReadBorrow<'_> {
    type Target = HeapStore;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.store.get() }
    }
}

// safety: heap access is serialized through borrow_state
unsafe impl Send for HeapCell {}
// safety: heap access is serialized through borrow_state
unsafe impl Sync for HeapCell {}
