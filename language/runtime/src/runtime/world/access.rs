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
pub(crate) struct ActivityGuard<'a> {
    /// The world whose active operation count is currently borrowed.
    world: &'a World,
}

impl Drop for ActivityGuard<'_> {
    fn drop(&mut self) {
        self.world.finish_activity();
    }
}

/// One temporary guard that releases exclusive access when dropped.
pub(crate) struct ExclusiveAccessGuard<'a> {
    /// The world under exclusive access.
    world: &'a World,
}

impl Drop for ExclusiveAccessGuard<'_> {
    fn drop(&mut self) {
        self.world.release_exclusive_access();
    }
}

impl World {
    /// Enter one world activity that must not overlap with exclusive world access.
    pub(crate) fn enter_activity(&self) -> RuntimeResult<ActivityGuard<'_>> {
        let mut access_state = self.access_state.write();
        match *access_state {
            AccessState::Shared { active_operations } => {
                *access_state = AccessState::Shared {
                    active_operations: active_operations.saturating_add(1),
                };

                Ok(ActivityGuard { world: self })
            }
            AccessState::Exclusive => Err(RuntimeError::ExclusiveAccessConflict.boxed()),
        }
    }

    /// Finish one previously started world activity.
    fn finish_activity(&self) {
        let mut access_state = self.access_state.write();
        match *access_state {
            AccessState::Shared { active_operations } => {
                let active_operations = active_operations.saturating_sub(1);
                *access_state = AccessState::Shared { active_operations };
            }
            AccessState::Exclusive => {
                panic!("world activity finished under exclusive access");
            }
        }
    }

    /// Acquire one exclusive-access lease for capture or restore operations.
    pub(crate) fn acquire_exclusive_access(&self) -> RuntimeResult<ExclusiveAccessGuard<'_>> {
        let mut access_state = self.access_state.write();
        match *access_state {
            AccessState::Shared {
                active_operations: 0,
            } => {
                *access_state = AccessState::Exclusive;
                Ok(ExclusiveAccessGuard { world: self })
            }
            AccessState::Shared { active_operations } => {
                Err(RuntimeError::ExclusiveAccessActive { active_operations }.boxed())
            }
            AccessState::Exclusive => Err(RuntimeError::ExclusiveAccessHeld.boxed()),
        }
    }

    /// Release one previously acquired exclusive-access lease.
    fn release_exclusive_access(&self) {
        *self.access_state.write() = AccessState::Shared {
            active_operations: 0,
        };
    }
}
