use serde::{Deserialize, Serialize};

use tspp_serde::Reflect;

use crate::{Binding, BindingEffect, Function, LocalNodeId, Point, StorageSet};

/// Function and call effect tables for one MIR module.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EffectTable {
    /// Function effects sorted by function id.
    functions: Vec<(LocalNodeId<Function>, FunctionEffect)>,
    /// Call effects sorted by callsite.
    calls: Vec<(Point, CallEffect)>,
}

impl EffectTable {
    /// Return function effects when present.
    pub fn function(&self, function: LocalNodeId<Function>) -> Option<&FunctionEffect> {
        let index = self
            .functions
            .binary_search_by_key(&function, |(function, _)| *function)
            .ok()?;

        Some(&self.functions[index].1)
    }

    /// Return mutable function effects, inserting unknown effects when absent.
    pub fn upsert_function(&mut self, function: LocalNodeId<Function>) -> &mut FunctionEffect {
        let index = self
            .functions
            .binary_search_by_key(&function, |(function, _)| *function);
        let index = match index {
            Ok(index) => index,
            Err(index) => {
                self.functions
                    .insert(index, (function, FunctionEffect::default()));
                index
            }
        };

        &mut self.functions[index].1
    }

    /// Return call effects when present.
    pub fn call(&self, callsite: Point) -> Option<&CallEffect> {
        let index = self
            .calls
            .binary_search_by_key(&callsite, |(callsite, _)| *callsite)
            .ok()?;

        Some(&self.calls[index].1)
    }

    /// Return mutable call effects when present.
    pub fn call_mut(&mut self, callsite: Point) -> Option<&mut CallEffect> {
        let index = self
            .calls
            .binary_search_by_key(&callsite, |(callsite, _)| *callsite)
            .ok()?;

        Some(&mut self.calls[index].1)
    }

    /// Return mutable call effects, inserting unknown effects when absent.
    pub fn upsert_call(&mut self, callsite: Point) -> &mut CallEffect {
        let index = self
            .calls
            .binary_search_by_key(&callsite, |(callsite, _)| *callsite);
        let index = match index {
            Ok(index) => index,
            Err(index) => {
                self.calls.insert(index, (callsite, CallEffect::default()));
                index
            }
        };

        &mut self.calls[index].1
    }

    /// Iterate function effects in function id order.
    pub fn functions(&self) -> impl Iterator<Item = (LocalNodeId<Function>, &FunctionEffect)> {
        self.functions
            .iter()
            .map(|(function, effect)| (*function, effect))
    }

    /// Replace every function effect.
    pub(crate) fn replace_functions(
        &mut self,
        functions: impl IntoIterator<Item = (LocalNodeId<Function>, FunctionEffect)>,
    ) {
        self.functions = functions.into_iter().collect();
        self.functions
            .sort_unstable_by_key(|(function, _)| *function);

        // reject duplicate function effects
        if self
            .functions
            .windows(2)
            .any(|functions| functions[0].0 == functions[1].0)
        {
            unreachable!("function has multiple effect entries");
        }
    }
}

/// Effects for one function body or declaration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionEffect {
    /// Memory touched by this function.
    pub memory: MemoryEffect,
    /// Behavioral effects of this function.
    pub behavior: FunctionBehavior,
}

impl FunctionEffect {
    /// Create an effect with no memory access or special behavior.
    pub fn none() -> Self {
        Self {
            memory: MemoryEffect::none(),
            behavior: FunctionBehavior::none(),
        }
    }

    /// Create an effect with only memory access.
    pub fn memory(memory: MemoryEffect) -> Self {
        Self {
            memory,
            behavior: FunctionBehavior::none(),
        }
    }

    /// Create the effect of linked code outside the program: unknown memory, external behavior.
    pub fn external() -> Self {
        Self {
            memory: MemoryEffect::unknown(),
            behavior: FunctionBehavior::external(),
        }
    }

    /// Create the effect one runtime binding declares: external memory with declared behavior.
    pub fn binding(binding: &Binding) -> Self {
        Self {
            memory: MemoryEffect::unknown(),
            behavior: FunctionBehavior::binding(binding),
        }
    }
}

/// Effects for one callsite.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallEffect {
    /// Explicit or inferred memory effects, when available.
    pub memory: Option<MemoryEffect>,
    /// Additional or inferred behavioral effects, when available.
    pub behavior: Option<FunctionBehavior>,
    /// Argument memory behavior when known.
    pub arguments: Vec<CallArgumentEffect>,
}

/// Behavior of one argument passed to a bodyless call.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct CallArgumentEffect {
    /// Access mode for this argument.
    pub access: ArgumentAccess,
    /// Escape behavior for this argument.
    pub escape: ArgumentEscape,
}

/// Access mode for a bodyless call pointer argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum ArgumentAccess {
    /// The argument is not accessed.
    None,
    /// The argument is only read.
    Read,
    /// The argument is only written.
    Write,
    /// The argument is read and written.
    #[default]
    ReadWrite,
}

/// Escape behavior for a bodyless call argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum ArgumentEscape {
    /// The argument does not escape the callee.
    None,
    /// The argument only escapes through the return value.
    Return,
    /// The argument may escape in an unknown way.
    #[default]
    Escape,
}

/// Memory access effect for a call or operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemoryEffect {
    /// Storage regions this operation may read.
    pub read: StorageSet,
    /// Storage regions this operation may write.
    pub write: StorageSet,
    /// Storage regions whose accesses this operation orders.
    pub barrier: StorageSet,
}

impl MemoryEffect {
    /// Create an effect with no memory access.
    pub const fn none() -> Self {
        Self {
            read: StorageSet::NONE,
            write: StorageSet::NONE,
            barrier: StorageSet::NONE,
        }
    }

    /// Create a read only effect over the provided storage.
    pub const fn read_only(storage: StorageSet) -> Self {
        Self {
            read: storage,
            write: StorageSet::NONE,
            barrier: StorageSet::NONE,
        }
    }

    /// Create a write only effect over the provided storage.
    pub const fn write_only(storage: StorageSet) -> Self {
        Self {
            read: StorageSet::NONE,
            write: storage,
            barrier: StorageSet::NONE,
        }
    }

    /// Create a read write effect over the provided storage.
    pub const fn read_write(storage: StorageSet) -> Self {
        Self {
            read: storage,
            write: storage,
            barrier: StorageSet::NONE,
        }
    }

    /// Create an effect ordering the accesses of the provided storage without touching it.
    pub const fn barrier(storage: StorageSet) -> Self {
        Self {
            read: StorageSet::NONE,
            write: StorageSet::NONE,
            barrier: storage,
        }
    }

    /// Create an unknown effect.
    pub const fn unknown() -> Self {
        Self {
            read: StorageSet::ANY,
            write: StorageSet::ANY,
            barrier: StorageSet::ANY,
        }
    }

    /// Return true when this effect orders memory accesses.
    pub fn is_barrier(&self) -> bool {
        !self.barrier.is_empty()
    }

    /// Return true when this effect may read memory.
    pub fn reads(&self) -> bool {
        !self.read.is_empty()
    }

    /// Return true when this effect may write memory.
    pub fn writes(&self) -> bool {
        !self.write.is_empty()
    }

    /// Return all storage touched by this effect.
    pub fn storage(&self) -> StorageSet {
        self.read.union(self.write)
    }

    /// Return the combined storage reads and writes of two effects.
    pub fn union(&self, other: &Self) -> Self {
        Self {
            read: self.read.union(other.read),
            write: self.write.union(other.write),
            barrier: self.barrier.union(other.barrier),
        }
    }

    /// Return this effect constrained to the given storage.
    pub fn with_storage(self, storage: StorageSet) -> Self {
        Self {
            read: if self.reads() {
                storage
            } else {
                StorageSet::NONE
            },
            write: if self.writes() {
                storage
            } else {
                StorageSet::NONE
            },
            barrier: self.barrier,
        }
    }
}

impl Default for MemoryEffect {
    fn default() -> Self {
        Self::unknown()
    }
}

/// Determinism for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Determinism {
    /// The operation is deterministic for the same inputs and runtime state.
    Deterministic,
    /// The operation may observe entropy, time, scheduling, or host state.
    NonDeterministic,
}

impl Determinism {
    /// Return true when the operation is deterministic.
    pub fn is_deterministic(self) -> bool {
        matches!(self, Self::Deterministic)
    }
}

/// Return behavior for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ReturnBehavior {
    /// The operation may or may not return to the caller.
    MayReturn,
    /// The operation never returns to the caller.
    NoReturn,
    /// The operation is guaranteed to return to the caller.
    WillReturn,
}

impl ReturnBehavior {
    /// Return true when the operation never returns.
    pub fn is_no_return(self) -> bool {
        matches!(self, Self::NoReturn)
    }

    /// Return true when the operation is guaranteed to return.
    pub fn is_will_return(self) -> bool {
        matches!(self, Self::WillReturn)
    }
}

/// Panic behavior for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum PanicBehavior {
    /// The operation cannot panic.
    CannotPanic,
    /// The operation may panic and unwind cleanup.
    MayPanic,
}

impl PanicBehavior {
    /// Return true when the operation may panic.
    pub fn may_panic(self) -> bool {
        matches!(self, Self::MayPanic)
    }
}

/// Parking behavior for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ParkBehavior {
    /// The operation cannot park the current fiber.
    CannotPark,
    /// The operation may park the current fiber.
    MayPark,
}

impl ParkBehavior {
    /// Return true when the operation may park the current fiber.
    pub fn may_park(self) -> bool {
        matches!(self, Self::MayPark)
    }
}

/// Behavioral effects for calls and functions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FunctionBehavior {
    /// Determinism for this operation.
    pub determinism: Determinism,
    /// Panic behavior for this operation.
    pub panic: PanicBehavior,
    /// Return behavior for this operation.
    pub return_behavior: ReturnBehavior,
    /// Parking behavior for this operation.
    pub park: ParkBehavior,
    /// Whether optimization must preserve each execution of this operation.
    pub must_preserve_execution: bool,
    /// Whether this operation may allocate storage.
    pub allocates: bool,
    /// Whether this operation may free storage.
    pub frees: bool,
}

impl FunctionBehavior {
    /// Combine alternative execution effects and return guarantees.
    pub fn union(&self, other: &Self) -> Self {
        Self {
            determinism: if self.determinism == other.determinism {
                self.determinism
            } else {
                Determinism::NonDeterministic
            },
            panic: if self.panic.may_panic() || other.panic.may_panic() {
                PanicBehavior::MayPanic
            } else {
                PanicBehavior::CannotPanic
            },
            park: if self.park.may_park() || other.park.may_park() {
                ParkBehavior::MayPark
            } else {
                ParkBehavior::CannotPark
            },
            return_behavior: if self.return_behavior == other.return_behavior {
                self.return_behavior
            } else {
                ReturnBehavior::MayReturn
            },
            must_preserve_execution: self.must_preserve_execution || other.must_preserve_execution,
            allocates: self.allocates || other.allocates,
            frees: self.frees || other.frees,
        }
    }

    /// Create a behavior with no special effects.
    pub const fn none() -> Self {
        Self {
            determinism: Determinism::Deterministic,
            panic: PanicBehavior::CannotPanic,
            return_behavior: ReturnBehavior::MayReturn,
            park: ParkBehavior::CannotPark,
            must_preserve_execution: false,
            allocates: false,
            frees: false,
        }
    }

    /// Create the behavior of external linked code, which cannot park.
    pub const fn external() -> Self {
        Self {
            determinism: Determinism::NonDeterministic,
            panic: PanicBehavior::MayPanic,
            return_behavior: ReturnBehavior::MayReturn,
            park: ParkBehavior::CannotPark,
            must_preserve_execution: true,
            allocates: true,
            frees: true,
        }
    }

    /// Create the behavior one runtime binding declares through its effect class and park option.
    pub fn binding(binding: &Binding) -> Self {
        let is_external = binding.effect == BindingEffect::External;

        Self {
            determinism: match binding.effect {
                BindingEffect::Pure | BindingEffect::Deterministic => Determinism::Deterministic,
                BindingEffect::External => Determinism::NonDeterministic,
            },
            panic: PanicBehavior::MayPanic,
            return_behavior: ReturnBehavior::MayReturn,
            park: match binding.is_park {
                true => ParkBehavior::MayPark,
                false => ParkBehavior::CannotPark,
            },
            must_preserve_execution: is_external,
            allocates: true,
            frees: is_external,
        }
    }

    /// Create pure behavior.
    pub const fn pure() -> Self {
        Self {
            determinism: Determinism::Deterministic,
            panic: PanicBehavior::CannotPanic,
            return_behavior: ReturnBehavior::WillReturn,
            park: ParkBehavior::CannotPark,
            must_preserve_execution: false,
            allocates: false,
            frees: false,
        }
    }

    /// Return this behavior with the may-panic flag enabled.
    pub const fn with_panic(mut self) -> Self {
        self.panic = PanicBehavior::MayPanic;
        self
    }

    /// Return this behavior with noreturn enabled.
    pub const fn with_noreturn(mut self) -> Self {
        self.return_behavior = ReturnBehavior::NoReturn;
        self
    }

    /// Return this behavior with will-return enabled.
    pub const fn with_will_return(mut self) -> Self {
        self.return_behavior = ReturnBehavior::WillReturn;
        self
    }

    /// Return this behavior with parking enabled.
    pub const fn with_park(mut self) -> Self {
        self.park = ParkBehavior::MayPark;
        self
    }

    /// Return this behavior with execution preservation enabled.
    pub const fn with_preserved_execution(mut self) -> Self {
        self.must_preserve_execution = true;
        self
    }

    /// Return this behavior with allocation enabled.
    pub const fn with_allocates(mut self) -> Self {
        self.allocates = true;
        self
    }

    /// Return this behavior with free enabled.
    pub const fn with_frees(mut self) -> Self {
        self.frees = true;
        self
    }
}

impl Default for FunctionBehavior {
    fn default() -> Self {
        Self::external()
    }
}
