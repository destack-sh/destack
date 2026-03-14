use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};

use super::World;

/// Shared and exclusive access state for world-owned execution and capture operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum AccessState {
    /// The world allows shared activity with the given number of active operations.
    Shared { active_operations: usize },
    /// The world is under exclusive access for capture or restore.
    Exclusive,
}

/// One temporary guard that releases one active world operation on drop.
pub(crate) struct WorldActivityGuard<'a> {
    /// The world whose active operation count is currently borrowed.
    world: &'a World,
}

impl Drop for WorldActivityGuard<'_> {
    fn drop(&mut self) {
        self.world.finish_activity();
    }
}

/// One temporary guard that releases exclusive access when dropped.
pub(crate) struct WorldExclusiveGuard<'a> {
    /// The world under exclusive access.
    world: &'a World,
}

impl Drop for WorldExclusiveGuard<'_> {
    fn drop(&mut self) {
        self.world.release_exclusive_access();
    }
}

/// One temporary guard that clears structural-mutation reentrancy on drop.
pub(crate) struct WorldMutationGuard<'a> {
    /// The world whose mutation state is currently active.
    world: &'a World,
}

impl Drop for WorldMutationGuard<'_> {
    fn drop(&mut self) {
        self.world.finish_mutation();
    }
}

impl World {
    /// Enter one world activity that must not overlap with exclusive world access.
    pub(crate) fn enter_activity(&self) -> RuntimeResult<WorldActivityGuard<'_>> {
        match self.access_state.get() {
            AccessState::Shared { active_operations } => {
                self.access_state.set(AccessState::Shared {
                    active_operations: active_operations.saturating_add(1),
                });

                Ok(WorldActivityGuard { world: self })
            }
            AccessState::Exclusive => Err(RuntimeError::ExclusiveAccessConflict.boxed()),
        }
    }

    /// Finish one previously started world activity.
    fn finish_activity(&self) {
        match self.access_state.get() {
            AccessState::Shared { active_operations } => {
                let active_operations = active_operations.saturating_sub(1);
                self.access_state
                    .set(AccessState::Shared { active_operations });
            }
            AccessState::Exclusive => {
                panic!("world activity finished under exclusive access");
            }
        }
    }

    /// Acquire one exclusive-access lease for capture or restore operations.
    pub(crate) fn acquire_exclusive_access(&self) -> RuntimeResult<WorldExclusiveGuard<'_>> {
        match self.access_state.get() {
            AccessState::Shared {
                active_operations: 0,
            } => {
                self.access_state.set(AccessState::Exclusive);
                Ok(WorldExclusiveGuard { world: self })
            }
            AccessState::Shared { active_operations } => {
                Err(RuntimeError::ExclusiveAccessActive { active_operations }.boxed())
            }
            AccessState::Exclusive => Err(RuntimeError::ExclusiveAccessHeld.boxed()),
        }
    }

    /// Release one previously acquired exclusive-access lease.
    fn release_exclusive_access(&self) {
        self.access_state.set(AccessState::Shared {
            active_operations: 0,
        });
    }

    /// Enter one structural mutation section.
    pub(crate) fn enter_mutation(&self) -> RuntimeResult<WorldMutationGuard<'_>> {
        if self.mutation_active.replace(true) {
            return Err(RuntimeError::Internal {
                message: "world mutation reentered".to_string(),
            }
            .boxed());
        }

        Ok(WorldMutationGuard { world: self })
    }

    /// Finish one previously entered structural mutation section.
    fn finish_mutation(&self) {
        self.mutation_active.set(false);
    }
}
