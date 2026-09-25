use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use smallvec::{SmallVec, smallvec};

use crate::{
    Block, DispatchSlot, FunctionId, GenericArgument, Instruction, LocalNodeId, Tree, TypeId,
    Value, ValueSlice,
};

/// One program point inside a function body: an instruction or a block terminator.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum Point {
    /// Callsite stored as an instruction.
    Instruction(LocalNodeId<Instruction>),
    /// Callsite stored as a block terminator.
    Terminator(LocalNodeId<Block>),
}

/// Dispatch performed by one callee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CallDispatch {
    /// Direct function call.
    Direct,
    /// Indirect call through a function value or pointer.
    Indirect,
    /// Virtual call through an object dispatch slot.
    Virtual {
        /// The dispatch slot for the method.
        slot: DispatchSlot,
    },
    /// Dynamic call through an erased dispatch table slot.
    Dynamic {
        /// The dispatch slot for the method.
        slot: DispatchSlot,
    },
    /// Call of the function one receiver type's witness names for an interface member.
    Witness,
}

/// Callable target for one call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Callee {
    /// Direct function target.
    Direct {
        /// The function to call.
        function: FunctionId,
        /// The generic arguments applied to a template.
        arguments: Vec<GenericArgument>,
    },
    /// Indirect function value or pointer target.
    Indirect {
        /// The function pointer or function value to call.
        value: Value,
    },
    /// Virtual method target.
    Virtual {
        /// The receiver value used for dispatch.
        receiver: Value,
        /// The class declaring the dispatch slot.
        class: TypeId,
        /// The virtual dispatch slot.
        slot: DispatchSlot,
    },
    /// Dynamic constraint target.
    Dynamic {
        /// The receiver value used for dispatch.
        receiver: Value,
        /// The constraint declaring the dispatch slot.
        constraint: TypeId,
        /// The dynamic dispatch slot.
        slot: DispatchSlot,
    },
    /// The function one receiver type's witness names for one interface member.
    Witness {
        /// The receiver type the witness is looked up for.
        receiver: TypeId,
        /// The applied interface declaring the member.
        interface: TypeId,
        /// The requirement the witness answers.
        requirement: FunctionId,
        /// The generic arguments applied to the requirement's own parameters.
        arguments: Vec<GenericArgument>,
    },
}

impl Callee {
    /// Return the dispatch performed by this callee.
    pub const fn dispatch(&self) -> CallDispatch {
        match self {
            Self::Direct { .. } => CallDispatch::Direct,
            Self::Indirect { .. } => CallDispatch::Indirect,
            Self::Virtual { slot, .. } => CallDispatch::Virtual { slot: *slot },
            Self::Dynamic { slot, .. } => CallDispatch::Dynamic { slot: *slot },
            Self::Witness { .. } => CallDispatch::Witness,
        }
    }

    /// Return the direct function target when present.
    pub const fn function(&self) -> Option<FunctionId> {
        match self {
            Self::Direct { function, .. } => Some(*function),
            Self::Indirect { .. }
            | Self::Virtual { .. }
            | Self::Dynamic { .. }
            | Self::Witness { .. } => None,
        }
    }

    /// Return values used to resolve this callee.
    pub fn uses(&self) -> SmallVec<[Value; 1]> {
        match self {
            Self::Direct { .. } | Self::Witness { .. } => smallvec![],
            Self::Indirect { value } => smallvec![*value],
            Self::Virtual { receiver, .. } | Self::Dynamic { receiver, .. } => {
                smallvec![*receiver]
            }
        }
    }

    /// Replace values used to resolve this callee.
    pub fn map_values(&self, mut map: impl FnMut(Value) -> Value) -> Self {
        match self {
            Self::Direct {
                function,
                arguments,
            } => Self::Direct {
                function: *function,
                arguments: arguments.clone(),
            },
            Self::Indirect { value } => Self::Indirect { value: map(*value) },
            Self::Virtual {
                receiver,
                class,
                slot,
            } => Self::Virtual {
                receiver: map(*receiver),
                class: *class,
                slot: *slot,
            },
            Self::Dynamic {
                receiver,
                constraint,
                slot,
            } => Self::Dynamic {
                receiver: map(*receiver),
                constraint: *constraint,
                slot: *slot,
            },
            Self::Witness { .. } => self.clone(),
        }
    }
}

/// One call operation shared by instructions and terminators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Call {
    /// The callable target.
    pub callee: Callee,
    /// The call arguments.
    pub arguments: ValueSlice,
    /// The signature type for the callee.
    pub signature: TypeId,
}

impl Call {
    /// Create one call.
    pub fn new(callee: Callee, arguments: ValueSlice, signature: TypeId) -> Self {
        Self {
            callee,
            arguments,
            signature,
        }
    }

    /// Return every value read by this call.
    pub fn uses(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        let mut values = self
            .callee
            .uses()
            .into_iter()
            .collect::<SmallVec<[Value; 8]>>();
        values.extend(tree.get_values(self.arguments).iter().copied());

        values
    }

    /// Return this call with remapped callee values and arguments.
    pub fn remap(&self, callee: Callee, arguments: ValueSlice) -> Self {
        Self {
            callee,
            arguments,
            signature: self.signature,
        }
    }
}
