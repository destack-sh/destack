use std::array;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use parking_lot::{Mutex, RwLock};

use crate::{ArtifactBinding, ArtifactBindingId, ArtifactError};

const ARTIFACT_BINDING_STRIPES: usize = 32;

/// Immutable bindings addressed by compact process-local ids.
#[derive(Debug)]
pub(crate) struct ArtifactBindingIndex {
    /// The next compact binding id.
    next_id: AtomicU32,
    /// Whether pruned binding ids may be available.
    is_free_binding_available: AtomicBool,
    /// Pruned binding ids available for reuse.
    free_binding_ids: Mutex<Vec<ArtifactBindingId>>,
    /// Bindings striped across independent locks for parallel access.
    stripes: [RwLock<Vec<Option<ArtifactBinding>>>; ARTIFACT_BINDING_STRIPES],
}

impl ArtifactBindingIndex {
    /// Build an empty artifact binding index.
    pub(crate) fn new() -> Self {
        Self {
            next_id: AtomicU32::new(0),
            is_free_binding_available: AtomicBool::new(false),
            free_binding_ids: Mutex::new(Vec::new()),
            stripes: array::from_fn(|_index| RwLock::new(Vec::new())),
        }
    }

    /// Insert one immutable binding and return its compact id.
    pub(crate) fn insert(
        &self,
        binding: ArtifactBinding,
    ) -> Result<ArtifactBindingId, ArtifactError> {
        let reused = if self.is_free_binding_available.load(Ordering::Acquire) {
            let mut free_binding_ids = self.free_binding_ids.lock();
            let reused = free_binding_ids.pop();
            self.is_free_binding_available
                .store(!free_binding_ids.is_empty(), Ordering::Release);

            reused
        } else {
            None
        };
        let id = match reused {
            Some(id) => id,
            None => ArtifactBindingId(self.next_id.fetch_add(1, Ordering::Relaxed)),
        };
        let stripe = id.0 as usize % ARTIFACT_BINDING_STRIPES;
        let index = id.0 as usize / ARTIFACT_BINDING_STRIPES;
        let mut bindings = self.stripes[stripe].write();
        if bindings.len() <= index {
            bindings.resize_with(index + 1, || None);
        }
        if bindings[index].is_some() {
            return Err(ArtifactError::Invalid(
                "reused artifact binding id is still occupied",
            ));
        }
        bindings[index] = Some(binding);

        Ok(id)
    }

    /// Return one immutable binding.
    pub(crate) fn get(&self, id: ArtifactBindingId) -> Option<ArtifactBinding> {
        let stripe = id.0 as usize % ARTIFACT_BINDING_STRIPES;
        let index = id.0 as usize / ARTIFACT_BINDING_STRIPES;
        let bindings = self.stripes[stripe].read();

        bindings.get(index)?.clone()
    }

    /// Return whether one binding id is present.
    pub(crate) fn contains(&self, id: ArtifactBindingId) -> bool {
        let stripe = id.0 as usize % ARTIFACT_BINDING_STRIPES;
        let index = id.0 as usize / ARTIFACT_BINDING_STRIPES;
        let bindings = self.stripes[stripe].read();

        bindings.get(index).is_some_and(Option::is_some)
    }

    /// Retain selected binding ids.
    pub(crate) fn retain(&self, retained: &HashSet<ArtifactBindingId>) {
        let mut next_id = 0;

        // release unreachable bindings and find the last live slot
        for (stripe, bindings) in self.stripes.iter().enumerate() {
            let mut bindings = bindings.write();
            for (index, binding) in bindings.iter_mut().enumerate() {
                let id = ArtifactBindingId((index * ARTIFACT_BINDING_STRIPES + stripe) as u32);
                if binding.is_some() && !retained.contains(&id) {
                    *binding = None;
                }
                if binding.is_some() {
                    next_id = next_id.max(id.0 + 1);
                }
            }
        }

        // compact trailing slots and rebuild reusable ids below the live high water mark
        let mut free_binding_ids = Vec::new();
        for (stripe, bindings) in self.stripes.iter().enumerate() {
            let mut bindings = bindings.write();
            let length = (next_id as usize).saturating_add(ARTIFACT_BINDING_STRIPES - 1 - stripe)
                / ARTIFACT_BINDING_STRIPES;
            bindings.truncate(length);
            bindings.shrink_to_fit();
            free_binding_ids.extend(
                bindings
                    .iter()
                    .enumerate()
                    .filter(|(_index, binding)| binding.is_none())
                    .map(|(index, _binding)| {
                        ArtifactBindingId((index * ARTIFACT_BINDING_STRIPES + stripe) as u32)
                    }),
            );
        }
        self.next_id.store(next_id, Ordering::Relaxed);
        let is_free_binding_available = !free_binding_ids.is_empty();
        *self.free_binding_ids.lock() = free_binding_ids;
        self.is_free_binding_available
            .store(is_free_binding_available, Ordering::Release);
    }
}

impl Default for ArtifactBindingIndex {
    /// Build an empty artifact binding index.
    fn default() -> Self {
        Self::new()
    }
}
