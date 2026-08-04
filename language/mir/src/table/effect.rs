use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{CallArgumentEffect, CallSite, Function, LocalNodeId, StorageSet};

/// Function and call effect tables for one MIR module.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EffectTable {
    /// Effects keyed by function id.
    pub functions: HashMap<LocalNodeId<Function>, FunctionEffect>,
    /// Effects keyed by callsite.
    pub calls: HashMap<CallSite, CallEffect>,
}

impl EffectTable {
    /// Return function effects when present.
    pub fn function(&self, function: LocalNodeId<Function>) -> Option<&FunctionEffect> {
        self.functions.get(&function)
    }

    /// Return mutable function effects, inserting unknown effects when absent.
    pub fn function_mut(&mut self, function: LocalNodeId<Function>) -> &mut FunctionEffect {
        self.functions.entry(function).or_default()
    }

    /// Return call effects when present.
    pub fn call(&self, callsite: CallSite) -> Option<&CallEffect> {
        self.calls.get(&callsite)
    }

    /// Return mutable call effects, inserting unknown effects when absent.
    pub fn call_mut(&mut self, callsite: CallSite) -> &mut CallEffect {
        self.calls.entry(callsite).or_default()
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

    /// Create an unknown effect.
    pub fn unknown() -> Self {
        Self {
            memory: MemoryEffect::unknown(),
            behavior: FunctionBehavior::unknown(),
        }
    }
}

/// Effects for one callsite.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallEffect {
    /// Memory touched by this call.
    pub memory: MemoryEffect,
    /// Behavioral effects of this call.
    pub behavior: FunctionBehavior,
    /// Resolved direct target when dispatch analysis proves one.
    pub target: Option<LocalNodeId<Function>>,
    /// Argument memory behavior when known.
    pub arguments: Vec<CallArgumentEffect>,
}

/// Memory access effect for a call or operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemoryEffect {
    /// Storage regions this operation may read.
    pub read: StorageSet,
    /// Storage regions this operation may write.
    pub write: StorageSet,
}

impl MemoryEffect {
    /// Create an effect with no memory access.
    pub const fn none() -> Self {
        Self {
            read: StorageSet::NONE,
            write: StorageSet::NONE,
        }
    }

    /// Create a read only effect over the provided storage.
    pub const fn read_only(storage: StorageSet) -> Self {
        Self {
            read: storage,
            write: StorageSet::NONE,
        }
    }

    /// Create a write only effect over the provided storage.
    pub const fn write_only(storage: StorageSet) -> Self {
        Self {
            read: StorageSet::NONE,
            write: storage,
        }
    }

    /// Create a read write effect over the provided storage.
    pub const fn read_write(storage: StorageSet) -> Self {
        Self {
            read: storage,
            write: storage,
        }
    }

    /// Create an unknown effect.
    pub const fn unknown() -> Self {
        Self {
            read: StorageSet::ANY,
            write: StorageSet::ANY,
        }
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

/// Behavioral effects for calls and functions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FunctionBehavior {
    /// Determinism for this operation.
    pub determinism: Determinism,
    /// Panic behavior for this operation.
    pub panic: PanicBehavior,
    /// Return behavior for this operation.
    pub return_behavior: ReturnBehavior,
    /// Whether optimization must preserve each execution of this operation.
    pub must_preserve_execution: bool,
    /// Whether this operation may allocate storage.
    pub allocates: bool,
    /// Whether this operation may free storage.
    pub frees: bool,
}

impl FunctionBehavior {
    /// Create a behavior with no special effects.
    pub const fn none() -> Self {
        Self {
            determinism: Determinism::Deterministic,
            panic: PanicBehavior::CannotPanic,
            return_behavior: ReturnBehavior::MayReturn,
            must_preserve_execution: false,
            allocates: false,
            frees: false,
        }
    }

    /// Create an unknown behavior.
    pub const fn unknown() -> Self {
        Self {
            determinism: Determinism::NonDeterministic,
            panic: PanicBehavior::MayPanic,
            return_behavior: ReturnBehavior::MayReturn,
            must_preserve_execution: false,
            allocates: true,
            frees: true,
        }
    }

    /// Create pure behavior.
    pub const fn pure() -> Self {
        Self {
            determinism: Determinism::Deterministic,
            panic: PanicBehavior::CannotPanic,
            return_behavior: ReturnBehavior::WillReturn,
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
        Self::unknown()
    }
}
