use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_heap as heap;
use tspp_memory::MemoryMap;
use tspp_program as program;
use tspp_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Fibers indexed by stable generation-checked identities.
#[derive(Debug, Default)]
pub(crate) struct FiberTable {
    /// Dense reusable fiber slots.
    slots: Vec<FiberSlot>,
    /// Vacant slot indices.
    vacant: Vec<u32>,
    /// Number of live fibers.
    len: usize,
}

/// One generation-checked fiber slot.
#[derive(Debug)]
struct FiberSlot {
    /// Generation issued by this slot.
    generation: u32,
    /// Live scheduling state when this slot is occupied.
    state: Option<FiberState>,
}

/// One fiber's scheduling state.
#[derive(Debug)]
pub(crate) enum FiberState {
    /// Executing on the worker or split off awaiting retention; the table
    /// holds no execution and wakes buffer until the fiber parks.
    Running {
        /// One wake delivered before the fiber parked.
        pending: Option<program::Value>,
    },
    /// Parked with its retained execution awaiting one wake.
    Parked(vm::Fiber),
    /// Woken with its wake runnable queued and holding the execution.
    Ready(vm::Fiber),
}

impl FiberTable {
    /// Insert one running fiber and return its identity.
    pub(crate) fn insert(&mut self) -> program::FiberId {
        let index = if let Some(index) = self.vacant.pop() {
            index
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(FiberSlot {
                generation: 1,
                state: None,
            });

            index
        };
        let slot = &mut self.slots[index as usize];
        slot.state = Some(FiberState::Running { pending: None });
        self.len += 1;

        program::FiberId::new(index, slot.generation)
    }

    /// Park one running fiber with its retained execution.
    ///
    /// A wake delivered between the park decision and this retention settles
    /// the fiber immediately; the returned value must queue its wake runnable.
    pub(crate) fn park(
        &mut self,
        fiber_id: program::FiberId,
        execution: vm::Fiber,
    ) -> RuntimeResult<Option<program::Value>> {
        let slot = self.slot_mut(fiber_id)?;
        match slot.state.take() {
            Some(FiberState::Running { pending: None }) => {
                slot.state = Some(FiberState::Parked(execution));

                Ok(None)
            }
            Some(FiberState::Running {
                pending: Some(value),
            }) => {
                slot.state = Some(FiberState::Ready(execution));

                Ok(Some(value))
            }
            state => {
                slot.state = state;

                Err(Box::<RuntimeError>::from(
                    program::Error::InvalidFiberState { fiber_id },
                ))
            }
        }
    }

    /// Take one wake buffered before the running fiber parked.
    pub(crate) fn take_pending(
        &mut self,
        fiber_id: program::FiberId,
    ) -> RuntimeResult<Option<program::Value>> {
        let slot = self.slot_mut(fiber_id)?;
        match &mut slot.state {
            Some(FiberState::Running { pending }) => Ok(pending.take()),
            _ => Err(Box::<RuntimeError>::from(
                program::Error::InvalidFiberState { fiber_id },
            )),
        }
    }

    /// Deliver one wake, returning whether a wake runnable must be queued.
    pub(crate) fn wake(
        &mut self,
        fiber_id: program::FiberId,
        value: program::Value,
    ) -> RuntimeResult<Option<program::Value>> {
        let slot = self.slot_mut(fiber_id)?;
        match slot.state.take() {
            // buffer wakes that arrive before the fiber parks
            Some(FiberState::Running { pending: None }) => {
                slot.state = Some(FiberState::Running {
                    pending: Some(value),
                });

                Ok(None)
            }
            Some(FiberState::Parked(execution)) => {
                slot.state = Some(FiberState::Ready(execution));

                Ok(Some(value))
            }
            state => {
                slot.state = state;

                Err(Box::<RuntimeError>::from(
                    program::Error::InvalidFiberState { fiber_id },
                ))
            }
        }
    }

    /// Take one ready fiber's execution for resumption.
    pub(crate) fn resume(&mut self, fiber_id: program::FiberId) -> RuntimeResult<vm::Fiber> {
        let slot = self.slot_mut(fiber_id)?;
        match slot.state.take() {
            Some(FiberState::Ready(execution)) => {
                slot.state = Some(FiberState::Running { pending: None });

                Ok(execution)
            }
            state => {
                slot.state = state;

                Err(Box::<RuntimeError>::from(
                    program::Error::InvalidFiberState { fiber_id },
                ))
            }
        }
    }

    /// Remove one completed fiber, returning an undelivered buffered wake.
    pub(crate) fn remove(
        &mut self,
        fiber_id: program::FiberId,
    ) -> RuntimeResult<Option<program::Value>> {
        let slot = self.slot_mut(fiber_id)?;
        let Some(state) = slot.state.take() else {
            return Err(Box::<RuntimeError>::from(program::Error::UndefinedFiber {
                fiber_id,
            }));
        };

        // reject retiring a fiber that still holds an execution
        let pending = match state {
            FiberState::Running { pending } => pending,
            state @ (FiberState::Parked(_) | FiberState::Ready(_)) => {
                slot.state = Some(state);

                return Err(Box::<RuntimeError>::from(
                    program::Error::InvalidFiberState { fiber_id },
                ));
            }
        };
        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.vacant.push(fiber_id.index());
        self.len -= 1;

        Ok(pending)
    }

    /// Return whether no fibers are live.
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate every retained execution for root visiting and teardown.
    pub(crate) fn executions_mut(&mut self) -> impl Iterator<Item = &mut vm::Fiber> {
        self.slots
            .iter_mut()
            .filter_map(|slot| match &mut slot.state {
                Some(FiberState::Parked(execution) | FiberState::Ready(execution)) => {
                    Some(execution)
                }
                Some(FiberState::Running { .. }) | None => None,
            })
    }

    /// Visit mutable heap root slots in wakes buffered for running fibers.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        for slot in &mut self.slots {
            let Some(FiberState::Running {
                pending: Some(value),
            }) = &mut slot.state
            else {
                continue;
            };
            program
                .visit_value_root_slots(value, visit)
                .map_err(Box::<RuntimeError>::from)?;
        }

        Ok(())
    }

    /// Capture one durable fiber table image.
    pub(crate) fn image(&self) -> FiberTableImage {
        FiberTableImage {
            slots: self
                .slots
                .iter()
                .map(|slot| FiberSlotImage {
                    generation: slot.generation,
                    state: slot.state.as_ref().map(FiberState::image),
                })
                .collect(),
            vacant: self.vacant.clone(),
        }
    }

    /// Restore one fiber table from its image over restored world memory.
    pub(crate) fn restore(image: &FiberTableImage, memory: &Arc<MemoryMap>) -> RuntimeResult<Self> {
        let mut slots = Vec::with_capacity(image.slots.len());
        let mut len = 0;
        for slot in &image.slots {
            let state = slot
                .state
                .as_ref()
                .map(|state| state.restore(memory))
                .transpose()?;
            len += usize::from(state.is_some());
            slots.push(FiberSlot {
                generation: slot.generation,
                state,
            });
        }

        Ok(Self {
            slots,
            vacant: image.vacant.clone(),
            len,
        })
    }

    /// Fork this fiber table over one already-forked world memory map.
    pub(crate) fn fork(&self, memory: &Arc<MemoryMap>) -> Self {
        Self {
            slots: self
                .slots
                .iter()
                .map(|slot| FiberSlot {
                    generation: slot.generation,
                    state: slot.state.as_ref().map(|state| state.fork(memory)),
                })
                .collect(),
            vacant: self.vacant.clone(),
            len: self.len,
        }
    }

    /// Return one live slot mutably.
    fn slot_mut(&mut self, fiber_id: program::FiberId) -> RuntimeResult<&mut FiberSlot> {
        let slot = self.slots.get_mut(fiber_id.index() as usize);
        let slot =
            slot.filter(|slot| slot.generation == fiber_id.generation() && slot.state.is_some());

        slot.ok_or_else(|| Box::<RuntimeError>::from(program::Error::UndefinedFiber { fiber_id }))
    }
}

impl FiberState {
    /// Capture one durable image of this scheduling state.
    fn image(&self) -> FiberStateImage {
        match self {
            Self::Running { pending } => FiberStateImage::Running {
                pending: pending.as_ref().map(program::Value::fork),
            },
            Self::Parked(execution) => FiberStateImage::Parked(execution.image()),
            Self::Ready(execution) => FiberStateImage::Ready(execution.image()),
        }
    }

    /// Fork this scheduling state over one already-forked world memory map.
    fn fork(&self, memory: &Arc<MemoryMap>) -> Self {
        match self {
            Self::Running { pending } => Self::Running {
                pending: pending.as_ref().map(program::Value::fork),
            },
            Self::Parked(execution) => Self::Parked(execution.fork(memory.clone())),
            Self::Ready(execution) => Self::Ready(execution.fork(memory.clone())),
        }
    }
}

/// Durable image of one fiber table.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct FiberTableImage {
    /// Captured fiber slots in identity order.
    slots: Vec<FiberSlotImage>,
    /// Captured vacant slot indices in reuse order.
    vacant: Vec<u32>,
}

/// Durable image of one generation-checked fiber slot.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FiberSlotImage {
    /// Generation issued by the slot.
    generation: u32,
    /// Captured scheduling state when the slot is occupied.
    state: Option<FiberStateImage>,
}

/// Durable image of one fiber's scheduling state.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum FiberStateImage {
    /// Mounted on the worker with the execution retained by the machine.
    Running {
        /// One wake delivered before the fiber parked.
        pending: Option<program::Value>,
    },
    /// Parked with its retained execution awaiting one wake.
    Parked(vm::FiberImage),
    /// Woken with its wake runnable queued and holding the execution.
    Ready(vm::FiberImage),
}

impl FiberTableImage {
    /// Return whether no fibers are captured live.
    pub(crate) fn is_empty(&self) -> bool {
        self.slots.iter().all(|slot| slot.state.is_none())
    }

    /// Iterate every captured parked or ready fiber execution.
    pub(crate) fn executions(&self) -> impl Iterator<Item = (program::FiberId, &vm::FiberImage)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| match &slot.state {
                Some(FiberStateImage::Parked(image) | FiberStateImage::Ready(image)) => {
                    Some((program::FiberId::new(index as u32, slot.generation), image))
                }
                _ => None,
            })
    }

    /// Fork this image through explicit COW value sharing.
    pub(crate) fn inherit(&self) -> Self {
        Self {
            slots: self
                .slots
                .iter()
                .map(|slot| FiberSlotImage {
                    generation: slot.generation,
                    state: slot.state.as_ref().map(FiberStateImage::inherit),
                })
                .collect(),
            vacant: self.vacant.clone(),
        }
    }
}

impl FiberStateImage {
    /// Fork this state image through explicit COW value sharing.
    fn inherit(&self) -> Self {
        match self {
            Self::Running { pending } => Self::Running {
                pending: pending.as_ref().map(program::Value::fork),
            },
            Self::Parked(execution) => Self::Parked(execution.clone()),
            Self::Ready(execution) => Self::Ready(execution.clone()),
        }
    }

    /// Restore one scheduling state over restored world memory.
    fn restore(&self, memory: &Arc<MemoryMap>) -> RuntimeResult<FiberState> {
        match self {
            Self::Running { pending } => Ok(FiberState::Running {
                pending: pending.as_ref().map(program::Value::fork),
            }),
            Self::Parked(image) => Ok(FiberState::Parked(
                vm::Fiber::from_image(memory.clone(), image).map_err(Box::<RuntimeError>::from)?,
            )),
            Self::Ready(image) => Ok(FiberState::Ready(
                vm::Fiber::from_image(memory.clone(), image).map_err(Box::<RuntimeError>::from)?,
            )),
        }
    }
}
