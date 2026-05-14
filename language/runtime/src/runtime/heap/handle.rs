use serde::{Deserialize, Serialize};

use destack_heap::{HeapReference, RootSlot};

use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Runtime-owned handle for one retained local heap reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapHandle {
    /// The handle table index.
    index: usize,
    /// The generation that validates the table entry.
    generation: usize,
}

/// One heap handle table entry.
#[derive(Debug, Clone)]
struct HeapHandleEntry {
    /// The retained local heap reference.
    reference: HeapReference,
    /// The generation that invalidates stale handles.
    generation: usize,
    /// The number of active handle owners.
    refcount: usize,
}

/// Worker-owned table of retained local heap handles.
#[derive(Debug, Default, Clone)]
pub struct HeapHandleTable {
    /// The table entries.
    entries: Vec<HeapHandleEntry>,
    /// The free entry indices.
    free: Vec<usize>,
}

impl HeapHandle {
    /// Return the table index.
    pub fn index(self) -> usize {
        self.index
    }

    /// Return the generation.
    pub fn generation(self) -> usize {
        self.generation
    }
}

impl HeapHandleTable {
    /// Return the number of retained handle entries.
    pub fn len(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.refcount != 0)
            .count()
    }

    /// Return whether this table has no retained handles.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Retain one local heap reference in the handle table.
    pub fn retain(&mut self, reference: HeapReference) -> HeapHandle {
        if let Some(index) = self.free.pop() {
            let entry = &mut self.entries[index];
            entry.reference = reference;
            entry.refcount = 1;

            return HeapHandle {
                index,
                generation: entry.generation,
            };
        }

        let index = self.entries.len();
        self.entries.push(HeapHandleEntry {
            reference,
            generation: 0,
            refcount: 1,
        });

        HeapHandle {
            index,
            generation: 0,
        }
    }

    /// Retain one existing handle owner.
    pub fn retain_handle(&mut self, handle: HeapHandle) -> RuntimeResult<HeapHandle> {
        let entry = self.entry_mut(handle)?;
        entry.refcount += 1;

        Ok(handle)
    }

    /// Release one retained handle owner.
    pub fn release(&mut self, handle: HeapHandle) -> RuntimeResult<()> {
        let entry = self.entry_mut(handle)?;
        entry.refcount -= 1;

        if entry.refcount == 0 {
            entry.reference = HeapReference::NULL;
            entry.generation = entry.generation.wrapping_add(1);
            self.free.push(handle.index);
        }

        Ok(())
    }

    /// Return the current local heap reference retained by one handle.
    pub fn reference(&self, handle: HeapHandle) -> RuntimeResult<HeapReference> {
        Ok(self.entry(handle)?.reference)
    }

    /// Visit mutable root locations retained by this handle table.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> destack_heap::HeapResult<()>,
    ) -> destack_heap::HeapResult<()> {
        for entry in &mut self.entries {
            if entry.refcount != 0 && !entry.reference.is_null() {
                visit(RootSlot::HeapReference(&mut entry.reference))?;
            }
        }

        Ok(())
    }

    /// Return one validated table entry.
    fn entry(&self, handle: HeapHandle) -> RuntimeResult<&HeapHandleEntry> {
        let Some(entry) = self.entries.get(handle.index) else {
            return Err(invalid_handle_error(handle));
        };

        if entry.refcount == 0 || entry.generation != handle.generation {
            return Err(invalid_handle_error(handle));
        }

        Ok(entry)
    }

    /// Return one validated mutable table entry.
    fn entry_mut(&mut self, handle: HeapHandle) -> RuntimeResult<&mut HeapHandleEntry> {
        let Some(entry) = self.entries.get_mut(handle.index) else {
            return Err(invalid_handle_error(handle));
        };

        if entry.refcount == 0 || entry.generation != handle.generation {
            return Err(invalid_handle_error(handle));
        }

        Ok(entry)
    }
}

/// Return one invalid handle error.
fn invalid_handle_error(handle: HeapHandle) -> Box<RuntimeError> {
    RuntimeError::Internal {
        message: format!(
            "invalid heap handle {}:{}",
            handle.index(),
            handle.generation()
        ),
    }
    .boxed()
}
