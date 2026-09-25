use destack_serde::Reflect;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    AtomicAccess, AtomicRmwOperator, BinaryOperator, Call, CallDispatch, CompareExchangeAccess,
    Constant, ConvertMode, CounterId, DispatchSlot, FenceAccess, FunctionId, GenericArgument,
    IndexSlice, Intrinsic, MemoryOperation, MemoryOrdering, Node, NodeType, Place, SamplerId,
    Space, Tree, TypeId, UnaryOperator, Value, ValueSlice, VectorReduceOperator, is_copy,
};

/// One MIR instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Instruction {
    /// Duplicate one SSA value.
    Copy {
        /// The SSA value to define.
        destination: Value,
        /// The value to duplicate.
        value: Value,
    },

    /// Recovered invalid instruction syntax.
    Error,

    // constants
    /// Load a constant value.
    Const {
        /// The SSA value to define.
        destination: Value,
        /// The constant value to load.
        value: Constant,
    },

    // arithmetic
    /// Binary operation (e.g., add, subtract, compare).
    Binary {
        /// The SSA value to define with the result.
        destination: Value,
        /// The binary operator to apply.
        operator: BinaryOperator,
        /// The left-hand operand.
        left: Value,
        /// The right-hand operand.
        right: Value,
    },
    /// Unary operation (e.g., negate, not).
    Unary {
        /// The SSA value to define with the result.
        destination: Value,
        /// The unary operator to apply.
        operator: UnaryOperator,
        /// The operand.
        argument: Value,
    },

    // type conversions
    /// Cast between types (bitcast, truncate, extend, etc.).
    Cast {
        /// The SSA value to define with the converted result.
        destination: Value,
        /// The cast operator to perform.
        operator: CastOperator,
        /// The value to cast.
        argument: Value,
        /// The target type to cast to.
        to_type: TypeId,
    },

    // conditional selection
    /// Select a value based on a boolean condition.
    ///
    /// The result is `then_value` when `condition` holds and `else_value` otherwise.
    /// Both values share one type and both are computed before the selection.
    Select {
        /// The SSA value to define with the selected result.
        destination: Value,
        /// The boolean condition (must be bool type).
        condition: Value,
        /// The value returned if condition is true.
        then_value: Value,
        /// The value returned if condition is false.
        else_value: Value,
    },

    /// Get a function pointer for a function (function.address).
    FunctionAddr {
        /// The SSA value to define with the function pointer.
        destination: Value,
        /// The function to take the address of.
        function: FunctionId,
        /// The generic arguments applied to a template.
        arguments: Vec<GenericArgument>,
    },
    /// Bind one environment to a function and produce a function value (function.bind).
    FunctionBind {
        /// The SSA value to define with the function value.
        destination: Value,
        /// The function to pair with the environment.
        function: FunctionId,
        /// The generic arguments applied to a template.
        arguments: Vec<GenericArgument>,
        /// The environment value to capture.
        environment: Value,
    },
    /// Project the environment from one function value (function.environment).
    FunctionEnvironment {
        /// The SSA value to define with the environment.
        destination: Value,
        /// The function value to project.
        function: Value,
    },
    /// Load the hidden environment for the current function (function.environment.current).
    FunctionEnvironmentCurrent {
        /// The SSA value to define with the hidden environment pointer.
        destination: Value,
    },

    // execution contexts
    /// Load the current execution context.
    ContextCurrent {
        /// The SSA value to define with the current context.
        destination: Value,
    },
    /// Replace the current execution context and return its previous value.
    ContextReplace {
        /// The SSA value to define with the previous context.
        destination: Value,
        /// The new current context.
        context: Value,
    },
    /// Extend one execution context with a variable value.
    ContextBind {
        /// The SSA value to define with the extended context.
        destination: Value,
        /// The context to extend.
        context: Value,
        /// The context variable identity.
        variable: Value,
        /// The value to bind.
        value: Value,
        /// The physical context node type.
        node_type: TypeId,
        /// The result context type.
        result_type: TypeId,
    },
    /// Load one variable value from an execution context.
    ContextGet {
        /// The SSA value to define with the selected value.
        destination: Value,
        /// The context to search.
        context: Value,
        /// The context variable identity.
        variable: Value,
        /// The value returned when the variable is unbound.
        default: Value,
        /// The physical context node type.
        node_type: TypeId,
        /// The result value type.
        result_type: TypeId,
    },

    // memory
    /// Read a place.
    Load {
        /// The SSA value to define with the loaded value.
        destination: Value,
        /// The storage to read.
        place: Place,
        /// The loaded value type.
        result_type: TypeId,
    },
    /// Take the address of a place.
    Address {
        /// The SSA value to define with the address.
        destination: Value,
        /// The addressed storage.
        place: Place,
        /// The result reference type.
        result_type: TypeId,
    },
    /// Write a place.
    Store {
        /// The storage to write.
        place: Place,
        /// The value to store.
        value: Value,
    },

    // aggregate construction
    /// Construct an aggregate from values in logical slot order.
    Aggregate {
        /// The SSA value to define with the constructed aggregate.
        destination: Value,
        /// The aggregate values stored in the tree's value buffer.
        values: ValueSlice,
    },

    // aggregate projection
    /// Extract one structural field from an aggregate value.
    FieldGet {
        /// The SSA value to define with the extracted field.
        destination: Value,
        /// The aggregate value to extract from.
        aggregate: Value,
        /// The zero-based logical field.
        field: u32,
    },
    /// Replace one structural field in an aggregate value.
    FieldSet {
        /// The SSA value to define with the new aggregate.
        destination: Value,
        /// The original aggregate value.
        aggregate: Value,
        /// The zero-based logical field to update.
        field: u32,
        /// The value to insert at the field.
        value: Value,
    },
    /// Extract one statically selected fixed-array element.
    ElementGet {
        /// The SSA value to define with the extracted element.
        destination: Value,
        /// The fixed-array aggregate to extract from.
        aggregate: Value,
        /// The zero-based element index.
        index: u32,
    },
    /// Insert one value into a statically selected fixed-array element.
    ElementSet {
        /// The SSA value to define with the updated fixed array.
        destination: Value,
        /// The original fixed-array aggregate.
        aggregate: Value,
        /// The zero-based element index.
        index: u32,
        /// The value to insert.
        value: Value,
    },
    // variant construction and projection
    /// Construct a variant value from one case payload.
    VariantNew {
        /// The SSA value to define with the constructed variant.
        destination: Value,
        /// The zero-based case index.
        case: u32,
        /// The case payload, or None for payloadless cases.
        payload: Option<Value>,
        /// The result variant type.
        result_type: TypeId,
    },
    /// Read the discriminant of a variant value.
    VariantTag {
        /// The SSA value to define with the discriminant.
        destination: Value,
        /// The variant value whose discriminant is read.
        variant: Value,
    },
    /// Read the discriminant of a stored variant.
    VariantTagLoad {
        /// The SSA value to define with the discriminant.
        destination: Value,
        /// The stored variant whose discriminant is read.
        place: Place,
    },
    /// Extract the payload of one statically selected variant case.
    VariantPayload {
        /// The SSA value to define with the extracted payload.
        destination: Value,
        /// The variant value to extract from.
        variant: Value,
        /// The zero-based case index.
        case: u32,
    },
    // slice descriptors
    /// Read the runtime length from a slice descriptor.
    SliceLength {
        /// The SSA value to define with the length.
        destination: Value,
        /// The slice value whose length is read.
        slice: Value,
    },

    // dynamic values
    /// Bind a typed reference payload to its concrete dispatch implementation.
    DynamicBind {
        /// The SSA value to define with the dynamic value.
        destination: Value,
        /// The typed payload reference.
        payload: Value,
        /// The concrete payload type paired with the destination constraint.
        concrete: TypeId,
    },
    /// Read the erased payload from a dynamic value.
    DynamicPayload {
        /// The SSA value to define with the payload.
        destination: Value,
        /// The dynamic value whose payload is read.
        dynamic: Value,
        /// The result type of the payload value.
        result_type: TypeId,
    },
    /// Read the concrete type id from a dynamic value's dispatch table.
    DynamicType {
        /// The SSA value to define with the type id.
        destination: Value,
        /// The dynamic value whose concrete type is read.
        dynamic: Value,
    },
    /// Read one slot entry through a dynamic value's concrete table.
    DynamicRead {
        /// The SSA value to define with the entry value.
        destination: Value,
        /// The dynamic value whose entry is read.
        dynamic: Value,
        /// The dispatch slot selecting the constraint entry.
        slot: DispatchSlot,
        /// The result type carrying the read value.
        result_type: TypeId,
    },
    /// Find one named entry through a dynamic value's concrete table.
    DynamicFind {
        /// The SSA value to define with the optional entry value.
        destination: Value,
        /// The dynamic value whose entries are searched.
        dynamic: Value,
        /// The string key value naming the entry.
        key: Value,
        /// The result type carrying the found value or undefined.
        result_type: TypeId,
    },

    // vector operations
    /// Broadcast a scalar to all vector lanes.
    VectorSplat {
        /// The SSA value to define with the vector result.
        destination: Value,
        /// The scalar value to broadcast.
        value: Value,
    },
    /// Extract a lane from a vector.
    VectorExtract {
        /// The SSA value to define with the extracted lane.
        destination: Value,
        /// The vector value to extract from.
        vector: Value,
        /// The lane index to extract.
        index: Value,
    },
    /// Insert a lane into a vector.
    VectorInsert {
        /// The SSA value to define with the updated vector.
        destination: Value,
        /// The original vector value.
        vector: Value,
        /// The lane index to update.
        index: Value,
        /// The lane value to insert.
        value: Value,
    },
    /// Shuffle vector lanes using a constant mask.
    VectorShuffle {
        /// The SSA value to define with the shuffled result.
        destination: Value,
        /// The left vector operand.
        left: Value,
        /// The right vector operand.
        right: Value,
        /// The shuffle mask indices.
        mask: IndexSlice,
    },
    /// Select vector lanes based on a boolean mask.
    VectorSelect {
        /// The SSA value to define with the selected result.
        destination: Value,
        /// The boolean mask vector.
        mask: Value,
        /// The value returned if the mask lane is true.
        then_value: Value,
        /// The value returned if the mask lane is false.
        else_value: Value,
    },
    /// Reduce a vector to a scalar.
    VectorReduce {
        /// The SSA value to define with the reduced result.
        destination: Value,
        /// The reduction operator to apply.
        operator: VectorReduceOperator,
        /// The vector value to reduce.
        vector: Value,
    },
    /// Compare two vectors elementwise.
    ///
    /// The result is a vector of boolean lanes.
    VectorCompare {
        /// The SSA value to define with the comparison result.
        destination: Value,
        /// The comparison operator to apply.
        operator: BinaryOperator,
        /// The left vector operand.
        left: Value,
        /// The right vector operand.
        right: Value,
    },
    /// Convert vector element types with an explicit mode.
    ///
    /// The result must have the same lane count as the input.
    VectorConvert {
        /// The SSA value to define with the converted vector.
        destination: Value,
        /// The conversion mode to apply.
        mode: ConvertMode,
        /// The vector value to convert.
        vector: Value,
    },

    // function calls
    /// Call one callable target without a local unwind continuation.
    Call {
        /// The SSA value to define with the return value, if any.
        destination: Option<Value>,
        /// The call operation.
        call: Call,
    },
    /// Drop one value.
    Drop {
        /// The value to drop.
        value: Value,
    },

    // heap allocation
    /// Allocate zeroed typed heap storage (`new.zeroed`).
    ///
    /// The result type decides whether the returned reference is managed or unique.
    NewZeroed {
        /// The SSA value to define with the allocated reference.
        destination: Value,
        /// The type stored in the allocation.
        storage_type: TypeId,
        /// The result type of the allocation.
        result_type: TypeId,
        /// The heap receiving the allocation.
        space: Space,
    },
    /// Allocate uninitialized typed heap storage (`new.uninit`).
    ///
    /// The result is an initialization token that must be completed before publication.
    NewUninit {
        /// The SSA value to define with the initialization token.
        destination: Value,
        /// The type stored in the allocation.
        storage_type: TypeId,
        /// The result type of the allocation.
        result_type: TypeId,
        /// The heap receiving the allocation.
        space: Space,
    },
    /// Complete initialization of one allocation (`new.complete`).
    NewComplete {
        /// The SSA value to define with the completed allocation.
        destination: Value,
        /// The initialization token to complete.
        value: Value,
        /// The result type of the completed value.
        result_type: TypeId,
    },
    /// Allocate zeroed typed repeated heap storage (`new.slice.zeroed`).
    ///
    /// The result type decides whether the returned slice is managed or unique.
    NewSliceZeroed {
        /// The SSA value to define with the allocated slice.
        destination: Value,
        /// The element type of the repeated storage.
        element: TypeId,
        /// The number of elements (runtime value).
        length: Value,
        /// The result type of the allocation.
        result_type: TypeId,
        /// The heap receiving the allocation.
        space: Space,
    },
    /// Allocate uninitialized typed repeated heap storage (`new.slice.uninit`).
    ///
    /// The result is an initialization token that must be completed before publication.
    NewSliceUninit {
        /// The SSA value to define with the initialization token.
        destination: Value,
        /// The element type of the repeated storage.
        element: TypeId,
        /// The number of elements (runtime value).
        length: Value,
        /// The result type of the allocation.
        result_type: TypeId,
        /// The heap receiving the allocation.
        space: Space,
    },
    /// Release ownership of an allocation while retaining its initialized contents for live aliases.
    ///
    /// Run finalization once the allocation is unreachable, before reclaiming its storage.
    Release {
        /// The unique heap representation whose backing allocation is returned.
        value: Value,
    },

    // collector protocol
    /// Record a managed reference write for the collector.
    BarrierWrite {
        /// The managed object whose reference range changed.
        object: Value,
        /// The byte offset of the changed reference range.
        offset: Value,
        /// The changed byte length.
        byte_len: Value,
    },

    // atomic memory operations
    /// Load from memory atomically.
    AtomicLoad {
        /// The SSA value to define with the loaded result.
        destination: Value,
        /// The storage to read.
        place: Place,
        /// The loaded value type.
        result_type: TypeId,
        /// The atomic access.
        access: AtomicAccess,
    },
    /// Store to memory atomically.
    AtomicStore {
        /// The storage to write.
        place: Place,
        /// The value to store.
        value: Value,
        /// The atomic access.
        access: AtomicAccess,
    },
    /// Compare exchange one memory location atomically.
    AtomicCompareExchange {
        /// The SSA value to define with the old value and success flag.
        destination: Value,
        /// The storage to update.
        place: Place,
        /// The expected current value.
        expected: Value,
        /// The replacement value.
        new_value: Value,
        /// Whether the compare exchange is weak.
        is_weak: bool,
        /// The compare exchange access.
        access: CompareExchangeAccess,
    },
    /// Apply one atomic read modify write operation.
    AtomicRmw {
        /// The SSA value to define with the old value.
        destination: Value,
        /// The read modify write operator.
        operator: AtomicRmwOperator,
        /// The storage to update.
        place: Place,
        /// The value argument for the operator.
        value: Value,
        /// The atomic access.
        access: AtomicAccess,
    },
    /// Publish one memory fence.
    AtomicFence {
        /// The fence access.
        access: FenceAccess,
    },
    // assumptions and hints
    /// Assume a condition is true (UB if false).
    Assume {
        /// The condition to assume.
        condition: Value,
    },

    // profile instrumentation
    /// Increment one profile counter.
    ProfileIncrement {
        /// The counter to increment.
        counter: CounterId,
    },
    /// Sample one runtime value.
    ProfileSample {
        /// The sampler receiving the value.
        sampler: SamplerId,
        /// The sampled MIR value.
        value: Value,
    },

    // runtime control
    /// Cooperate with runtime work at one explicit statepoint.
    Poll,

    // debug control
    /// Debugger breakpoint.
    Breakpoint,

    // intrinsics
    /// Call a machine intrinsic.
    Intrinsic {
        /// The SSA value to define with the result, if any.
        destination: Option<Value>,
        /// The intrinsic to call.
        intrinsic: Intrinsic,
        /// The arguments to pass.
        arguments: ValueSlice,
    },
}

impl Node for Instruction {
    const TYPE: NodeType = NodeType::Instruction;
}

impl Instruction {
    /// Return whether an instruction can have observable effects when its result is unused.
    pub fn has_side_effects(&self) -> bool {
        // preserve operations whose execution can trap
        if self.may_trap() {
            return true;
        }

        // classify instructions by side effects
        match self {
            Self::Error => {
                panic!("recovered MIR instruction reached optimizer");
            }

            // classify scalar and aggregate instructions as pure
            Self::Copy { .. }
            | Self::Const { .. }
            | Self::Binary { .. }
            | Self::Unary { .. }
            | Self::Cast { .. }
            | Self::Select { .. }
            | Self::Aggregate { .. }
            | Self::FieldGet { .. }
            | Self::FieldSet { .. }
            | Self::ElementGet { .. }
            | Self::ElementSet { .. }
            | Self::VariantNew { .. }
            | Self::VariantTag { .. }
            | Self::VariantTagLoad { .. }
            | Self::VariantPayload { .. }
            | Self::Address { .. }
            | Self::SliceLength { .. }
            | Self::DynamicBind { .. }
            | Self::DynamicPayload { .. }
            | Self::DynamicType { .. }
            | Self::DynamicRead { .. }
            | Self::DynamicFind { .. }
            | Self::VectorSplat { .. }
            | Self::VectorExtract { .. }
            | Self::VectorInsert { .. }
            | Self::VectorShuffle { .. }
            | Self::VectorSelect { .. }
            | Self::VectorReduce { .. }
            | Self::VectorCompare { .. }
            | Self::VectorConvert { .. }
            | Self::FunctionAddr { .. }
            | Self::FunctionBind { .. }
            | Self::FunctionEnvironment { .. }
            | Self::FunctionEnvironmentCurrent { .. }
            | Self::ContextCurrent { .. }
            | Self::ContextGet { .. }
            | Self::NewComplete { .. }
            | Self::Assume { .. } => false,

            // classify nonvolatile reads as pure
            Self::Load { .. } => false,

            // preserve memory writes
            Self::Store { .. }
            | Self::AtomicLoad { .. }
            | Self::AtomicStore { .. }
            | Self::AtomicCompareExchange { .. }
            | Self::AtomicRmw { .. }
            | Self::AtomicFence { .. }
            | Self::BarrierWrite { .. } => true,

            // preserve calls
            Self::Call { .. }
            | Self::ContextReplace { .. }
            | Self::ContextBind { .. }
            | Self::Drop { .. } => true,

            // preserve allocations
            Self::NewZeroed { .. }
            | Self::NewUninit { .. }
            | Self::NewSliceZeroed { .. }
            | Self::NewSliceUninit { .. } => true,

            // preserve storage release
            Self::Release { .. } => true,

            // preserve profile instrumentation
            Self::ProfileIncrement { .. } | Self::ProfileSample { .. } => true,

            // preserve runtime and debugger control
            Self::Poll | Self::Breakpoint => true,

            // check whether the intrinsic has side effects
            Self::Intrinsic { intrinsic, .. } => {
                !intrinsic.is_pure() || matches!(intrinsic, Intrinsic::BlackBox)
            }
        }
    }

    /// Return whether executing this operation can trap without further operand guarantees.
    pub fn may_trap(&self) -> bool {
        match self {
            Self::Binary {
                operator: BinaryOperator::Divide | BinaryOperator::Remainder,
                ..
            }
            | Self::Cast {
                operator: CastOperator::FloatToInt,
                ..
            }
            | Self::Load { .. }
            | Self::Store { .. }
            | Self::VariantTagLoad { .. }
            | Self::DynamicRead { .. }
            | Self::AtomicLoad { .. }
            | Self::AtomicStore { .. }
            | Self::AtomicCompareExchange { .. }
            | Self::AtomicRmw { .. }
            | Self::Call { .. }
            | Self::Drop { .. }
            | Self::NewZeroed { .. }
            | Self::NewUninit { .. }
            | Self::NewSliceZeroed { .. }
            | Self::NewSliceUninit { .. } => true,
            Self::Intrinsic { intrinsic, .. } => matches!(
                intrinsic,
                Intrinsic::Memcpy
                    | Intrinsic::Memmove
                    | Intrinsic::Memset
                    | Intrinsic::Memcmp
                    | Intrinsic::VolatileLoad
                    | Intrinsic::VolatileStore
                    | Intrinsic::DivideCeil
                    | Intrinsic::RemainderEuclidean
                    | Intrinsic::Clamp
            ),
            _ => false,
        }
    }

    /// Return the storage selected by a memory operation.
    pub fn place(&self) -> Option<&Place> {
        match self {
            Self::Load { place, .. }
            | Self::VariantTagLoad { place, .. }
            | Self::Store { place, .. }
            | Self::Address { place, .. }
            | Self::AtomicLoad { place, .. }
            | Self::AtomicStore { place, .. }
            | Self::AtomicCompareExchange { place, .. }
            | Self::AtomicRmw { place, .. } => Some(place),
            _ => None,
        }
    }

    /// Return the place one load, store, or atomic operation accesses and how it accesses it.
    ///
    /// A load of a value that is not Copy moves it out, which also writes its place.
    pub fn place_access(
        &self,
        function: FunctionId,
        tree: &Tree,
    ) -> Option<(&Place, MemoryOperation)> {
        match self {
            // copy a Copy value out, else move it out
            Self::Load {
                place, result_type, ..
            } => {
                let generics = &tree.get(function).generics;
                let operation = if is_copy(tree, *result_type, generics) {
                    MemoryOperation::Read
                } else {
                    MemoryOperation::ReadWrite
                };

                Some((place, operation))
            }
            Self::AtomicLoad { place, .. } => Some((place, MemoryOperation::Read)),
            Self::Store { place, .. } | Self::AtomicStore { place, .. } => {
                Some((place, MemoryOperation::Write))
            }
            Self::AtomicCompareExchange { place, .. } | Self::AtomicRmw { place, .. } => {
                Some((place, MemoryOperation::ReadWrite))
            }
            _ => None,
        }
    }

    /// Return the heap one allocating instruction names.
    pub fn allocation_space(&self) -> Option<Space> {
        match self {
            Instruction::NewZeroed { space, .. }
            | Instruction::NewUninit { space, .. }
            | Instruction::NewSliceZeroed { space, .. }
            | Instruction::NewSliceUninit { space, .. } => Some(*space),
            _ => None,
        }
    }

    /// Return the place one storing instruction writes.
    pub fn store_place(&self) -> Option<&Place> {
        match self {
            Instruction::Store { place, .. }
            | Instruction::AtomicStore { place, .. }
            | Instruction::AtomicCompareExchange { place, .. }
            | Instruction::AtomicRmw { place, .. } => Some(place),
            _ => None,
        }
    }

    /// Return the SSA value defined by this instruction.
    pub fn destination(&self) -> Option<Value> {
        match self {
            Instruction::Error => None,
            Instruction::Copy { destination, .. } | Instruction::Const { destination, .. } => {
                Some(*destination)
            }
            Instruction::Binary { destination, .. } => Some(*destination),
            Instruction::Unary { destination, .. } => Some(*destination),
            Instruction::Cast { destination, .. } => Some(*destination),
            Instruction::Select { destination, .. } => Some(*destination),
            Instruction::FunctionAddr { destination, .. } => Some(*destination),
            Instruction::FunctionBind { destination, .. } => Some(*destination),
            Instruction::FunctionEnvironment { destination, .. } => Some(*destination),
            Instruction::FunctionEnvironmentCurrent { destination, .. } => Some(*destination),
            Instruction::ContextCurrent { destination, .. }
            | Instruction::ContextReplace { destination, .. }
            | Instruction::ContextBind { destination, .. }
            | Instruction::ContextGet { destination, .. } => Some(*destination),
            Instruction::Load { destination, .. } | Instruction::Address { destination, .. } => {
                Some(*destination)
            }
            Instruction::Store { .. } => None,
            Instruction::Aggregate { destination, .. } => Some(*destination),
            Instruction::FieldGet { destination, .. } => Some(*destination),
            Instruction::FieldSet { destination, .. } => Some(*destination),
            Instruction::ElementGet { destination, .. } => Some(*destination),
            Instruction::ElementSet { destination, .. } => Some(*destination),
            Instruction::VariantNew { destination, .. } => Some(*destination),
            Instruction::VariantTag { destination, .. } => Some(*destination),
            Instruction::VariantTagLoad { destination, .. } => Some(*destination),
            Instruction::VariantPayload { destination, .. } => Some(*destination),
            Instruction::SliceLength { destination, .. } => Some(*destination),
            Instruction::DynamicBind { destination, .. } => Some(*destination),
            Instruction::DynamicPayload { destination, .. } => Some(*destination),
            Instruction::DynamicType { destination, .. } => Some(*destination),
            Instruction::DynamicRead { destination, .. } => Some(*destination),
            Instruction::DynamicFind { destination, .. } => Some(*destination),
            Instruction::VectorSplat { destination, .. } => Some(*destination),
            Instruction::VectorExtract { destination, .. } => Some(*destination),
            Instruction::VectorInsert { destination, .. } => Some(*destination),
            Instruction::VectorShuffle { destination, .. } => Some(*destination),
            Instruction::VectorSelect { destination, .. } => Some(*destination),
            Instruction::VectorReduce { destination, .. } => Some(*destination),
            Instruction::VectorCompare { destination, .. } => Some(*destination),
            Instruction::VectorConvert { destination, .. } => Some(*destination),
            Instruction::Call { destination, .. } => *destination,
            Instruction::Drop { .. } => None,
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewComplete { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. } => Some(*destination),
            Instruction::Release { .. } => None,
            Instruction::BarrierWrite { .. } => None,
            Instruction::AtomicLoad { destination, .. } => Some(*destination),
            Instruction::AtomicStore { .. } => None,
            Instruction::AtomicCompareExchange { destination, .. } => Some(*destination),
            Instruction::AtomicRmw { destination, .. } => Some(*destination),
            Instruction::AtomicFence { .. } => None,
            Instruction::Assume { .. } => None,
            Instruction::ProfileIncrement { .. } => None,
            Instruction::ProfileSample { .. } => None,
            Instruction::Poll | Instruction::Breakpoint => None,
            Instruction::Intrinsic { destination, .. } => *destination,
        }
    }

    /// Return the values this instruction stores inline.
    ///
    /// Call and intrinsic arguments live in the tree's value buffer, which `argument_slice` reaches.
    pub fn uses(&self) -> SmallVec<[Value; 4]> {
        match self {
            Instruction::Error => smallvec![],
            Instruction::Copy { value, .. } => smallvec![*value],
            Instruction::Const { .. } => smallvec![],
            Instruction::Binary { left, right, .. } => smallvec![*left, *right],
            Instruction::Unary { argument, .. } => smallvec![*argument],
            Instruction::Cast { argument, .. } => smallvec![*argument],
            Instruction::Select {
                condition,
                then_value,
                else_value,
                ..
            } => smallvec![*condition, *then_value, *else_value],
            Instruction::FunctionAddr { .. } => smallvec![],
            Instruction::FunctionBind { environment, .. } => smallvec![*environment],
            Instruction::FunctionEnvironment { function, .. } => smallvec![*function],
            Instruction::FunctionEnvironmentCurrent { .. } => smallvec![],
            Instruction::ContextCurrent { .. } => smallvec![],
            Instruction::ContextReplace { context, .. } => smallvec![*context],
            Instruction::ContextBind {
                context,
                variable,
                value,
                ..
            } => smallvec![*context, *variable, *value],
            Instruction::ContextGet {
                context,
                variable,
                default,
                ..
            } => smallvec![*context, *variable, *default],
            Instruction::Load { place, .. }
            | Instruction::Address { place, .. }
            | Instruction::AtomicLoad { place, .. } => place.uses(),
            Instruction::Store { place, value }
            | Instruction::AtomicStore { place, value, .. }
            | Instruction::AtomicRmw { place, value, .. } => {
                let mut values = place.uses();
                values.push(*value);

                values
            }
            // arguments stored externally
            Instruction::Aggregate { .. } => smallvec![],
            Instruction::FieldGet { aggregate, .. } => smallvec![*aggregate],
            Instruction::FieldSet {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::ElementGet { aggregate, .. } => smallvec![*aggregate],
            Instruction::ElementSet {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::VariantNew { payload, .. } => {
                payload.iter().copied().collect::<SmallVec<[Value; 4]>>()
            }
            Instruction::VariantTag { variant, .. } => smallvec![*variant],
            Instruction::VariantTagLoad { place, .. } => place.uses(),
            Instruction::VariantPayload { variant, .. } => smallvec![*variant],
            Instruction::SliceLength { slice, .. } => smallvec![*slice],
            Instruction::DynamicBind { payload, .. } => smallvec![*payload],
            Instruction::DynamicPayload { dynamic, .. } => smallvec![*dynamic],
            Instruction::DynamicType { dynamic, .. } => smallvec![*dynamic],
            Instruction::DynamicRead { dynamic, .. } => smallvec![*dynamic],
            Instruction::DynamicFind { dynamic, key, .. } => smallvec![*dynamic, *key],
            Instruction::VectorSplat { value, .. } => smallvec![*value],
            Instruction::VectorExtract { vector, index, .. } => smallvec![*vector, *index],
            Instruction::VectorInsert {
                vector,
                index,
                value,
                ..
            } => smallvec![*vector, *index, *value],
            Instruction::VectorShuffle { left, right, .. } => smallvec![*left, *right],
            Instruction::VectorSelect {
                mask,
                then_value,
                else_value,
                ..
            } => smallvec![*mask, *then_value, *else_value],
            Instruction::VectorReduce { vector, .. } => smallvec![*vector],
            Instruction::VectorCompare { left, right, .. } => smallvec![*left, *right],
            Instruction::VectorConvert { vector, .. } => smallvec![*vector],
            Instruction::Call { call, .. } => call.callee.uses().into_iter().collect(),
            Instruction::Drop { value } => smallvec![*value],
            Instruction::NewZeroed { .. } | Instruction::NewUninit { .. } => smallvec![],
            Instruction::NewComplete { value, .. } => smallvec![*value],
            Instruction::NewSliceZeroed { length, .. }
            | Instruction::NewSliceUninit { length, .. } => smallvec![*length],
            Instruction::Release { value } => smallvec![*value],
            Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => smallvec![*object, *offset, *byte_len],
            Instruction::AtomicCompareExchange {
                place,
                expected,
                new_value,
                ..
            } => {
                let mut values = place.uses();
                values.extend([*expected, *new_value]);

                values
            }
            Instruction::AtomicFence { .. } => smallvec![],
            Instruction::Assume { condition } => smallvec![*condition],
            Instruction::ProfileIncrement { .. } => smallvec![],
            Instruction::ProfileSample { value, .. } => smallvec![*value],
            Instruction::Poll | Instruction::Breakpoint => smallvec![],
            Instruction::Intrinsic { .. } => smallvec![],
        }
    }

    /// Return every value read by this instruction.
    pub fn reads(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        let mut values = self.uses().into_iter().collect::<SmallVec<[Value; 8]>>();

        if let Some(arguments) = self.argument_slice() {
            values.extend(tree.get_values(arguments).iter().copied());
        }

        values
    }

    /// Return values consumed by this instruction.
    pub fn consumes(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        match self {
            Instruction::Store { value, .. }
            | Instruction::NewComplete { value, .. }
            | Instruction::Release { value }
            | Instruction::Cast {
                argument: value, ..
            } => smallvec![*value],
            Instruction::Intrinsic {
                intrinsic: Intrinsic::Transmute | Intrinsic::SpaceCast,
                arguments,
                ..
            } => tree.get_values(*arguments).iter().copied().collect(),
            Instruction::AtomicStore { value, .. } | Instruction::AtomicRmw { value, .. } => {
                smallvec![*value]
            }
            Instruction::AtomicCompareExchange {
                expected,
                new_value,
                ..
            } => smallvec![*expected, *new_value],
            Instruction::FieldSet {
                aggregate, value, ..
            }
            | Instruction::ElementSet {
                aggregate, value, ..
            } => smallvec![*aggregate, *value],
            Instruction::VariantNew { payload, .. } => {
                payload.iter().copied().collect::<SmallVec<[Value; 8]>>()
            }
            Instruction::FunctionBind { environment, .. }
            | Instruction::VectorSplat {
                value: environment, ..
            } => smallvec![*environment],
            Instruction::FunctionEnvironment { .. }
            | Instruction::FunctionEnvironmentCurrent { .. } => smallvec![],
            Instruction::VectorInsert { vector, value, .. } => smallvec![*vector, *value],
            Instruction::Aggregate { .. } => self.argument_slice_values(tree),
            Instruction::Call { call, .. } => {
                let mut values = call
                    .callee
                    .uses()
                    .into_iter()
                    .collect::<SmallVec<[Value; 8]>>();
                values.extend(tree.get_values(call.arguments).iter().copied());

                values
            }
            Instruction::Drop { value } => smallvec![*value],
            Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => {
                let arguments = tree.get_values(*arguments);
                let mut values = SmallVec::<[Value; 8]>::new();

                for &index in intrinsic.consumed_arguments() {
                    let Some(&value) = arguments.get(index as usize) else {
                        continue;
                    };

                    values.push(value);
                }

                values
            }
            _ => smallvec![],
        }
    }

    /// Return externally stored values for this instruction.
    fn argument_slice_values(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        let Some(arguments) = self.argument_slice() else {
            return smallvec![];
        };

        tree.get_values(arguments).iter().copied().collect()
    }

    /// Return the argument slice of the instructions that externalize their value lists.
    pub fn argument_slice(&self) -> Option<ValueSlice> {
        match self {
            Instruction::Aggregate { values, .. } => Some(*values),
            Instruction::Call { call, .. } => Some(call.arguments),
            Instruction::Intrinsic { arguments, .. } => Some(*arguments),
            _ => None,
        }
    }

    /// Return the dispatch for a call instruction.
    pub fn call_dispatch(&self) -> Option<CallDispatch> {
        match self {
            Instruction::Call { call, .. } => Some(call.callee.dispatch()),
            _ => None,
        }
    }

    /// Return the signature type for call instructions.
    pub fn call_signature(&self) -> Option<TypeId> {
        match self {
            Instruction::Call { call, .. } => Some(call.signature),
            _ => None,
        }
    }

    /// Return the memory orderings this instruction carries, mutable.
    pub fn orderings_mut(&mut self) -> SmallVec<[&mut MemoryOrdering; 2]> {
        match self {
            Instruction::AtomicLoad { access, .. }
            | Instruction::AtomicStore { access, .. }
            | Instruction::AtomicRmw { access, .. } => smallvec![&mut access.ordering],
            Instruction::AtomicCompareExchange { access, .. } => {
                let mut orderings = smallvec![&mut access.success.ordering];
                orderings.extend(access.failure_ordering.as_mut());

                orderings
            }
            Instruction::AtomicFence { access } => smallvec![&mut access.ordering],
            _ => SmallVec::new(),
        }
    }

    /// Return the direct target for call instructions.
    pub fn call_direct_target(&self) -> Option<FunctionId> {
        match self {
            Instruction::Call { call, .. } => call.callee.function(),
            _ => None,
        }
    }
}

/// Scalar conversion operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CastOperator {
    /// Reinterpret bits at the same size.
    Bitcast,
    /// Convert integers, extending by source signedness or retaining low bits.
    IntToInt,
    /// Convert integers, clamping into the destination range.
    IntToIntSaturating,
    /// Convert an integer to a float, rounding to nearest with ties to even.
    IntToFloat,
    /// Convert a float to an integer, trapping on NaN or an out of range result.
    FloatToInt,
    /// Convert a float to an integer, clamping overflow and mapping NaN to zero.
    FloatToIntSaturating,
    /// Convert float formats, rounding to nearest with ties to even.
    FloatToFloat,
    /// Add the world memory base to a reference offset to obtain a native pointer.
    ReferenceToPointer,
    /// Subtract the world memory base from a native pointer to obtain a reference offset.
    PointerToReference,
    /// Convert a pointer to an integer.
    PointerToInt,
    /// Convert an integer to a pointer.
    IntToPointer,
}

impl CastOperator {
    /// Return the MIR instruction name.
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Bitcast => "cast.bit",
            Self::IntToInt => "cast.intToInt",
            Self::IntToIntSaturating => "cast.intToIntSaturating",
            Self::IntToFloat => "cast.intToFloat",
            Self::FloatToInt => "cast.floatToInt",
            Self::FloatToIntSaturating => "cast.floatToIntSaturating",
            Self::FloatToFloat => "cast.floatToFloat",
            Self::ReferenceToPointer => "cast.referenceToPointer",
            Self::PointerToReference => "cast.pointerToReference",
            Self::PointerToInt => "cast.pointerToInt",
            Self::IntToPointer => "cast.intToPointer",
        }
    }
}

/// One integer width conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerConversion {
    /// Keep the bits of an equal width.
    Identity,
    /// Keep the low bits of a narrower width.
    Truncate,
    /// Widen by repeating the sign bit.
    SignExtend,
    /// Widen with zero bits.
    ZeroExtend,
}

impl IntegerConversion {
    /// Select the conversion between two integer widths by source signedness.
    pub const fn new(source_bits: u32, target_bits: u32, is_signed: bool) -> Self {
        if source_bits > target_bits {
            Self::Truncate
        } else if source_bits < target_bits && is_signed {
            Self::SignExtend
        } else if source_bits < target_bits {
            Self::ZeroExtend
        } else {
            Self::Identity
        }
    }
}

impl fmt::Display for CastOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for CastOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cast.bit" => Ok(Self::Bitcast),
            "cast.intToInt" => Ok(Self::IntToInt),
            "cast.intToIntSaturating" => Ok(Self::IntToIntSaturating),
            "cast.intToFloat" => Ok(Self::IntToFloat),
            "cast.floatToInt" => Ok(Self::FloatToInt),
            "cast.floatToIntSaturating" => Ok(Self::FloatToIntSaturating),
            "cast.floatToFloat" => Ok(Self::FloatToFloat),
            "cast.referenceToPointer" => Ok(Self::ReferenceToPointer),
            "cast.pointerToReference" => Ok(Self::PointerToReference),
            "cast.pointerToInt" => Ok(Self::PointerToInt),
            "cast.intToPointer" => Ok(Self::IntToPointer),
            _ => Err(()),
        }
    }
}
