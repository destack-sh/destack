use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    BinaryOperator, BlockReference, Call, CallDispatchKind, Constant, DispatchSlot,
    FunctionReference, IntegerReference, Node, NodeType, TypeReference, ValueReference,
};

/// One control-flow edge target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockTarget {
    /// The block to transfer control to.
    pub block: BlockReference,
    /// Arguments for the target block's parameters.
    pub arguments: Vec<ValueReference>,
}

/// Unrecoverable runtime trap kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrapKind {
    /// Abort execution immediately.
    Abort,
}

/// Semantic constraint for a runtime check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckConstraint {
    /// Bounds check on an index into a collection.
    Bounds {
        /// The index being checked.
        index: ValueReference,
        /// The length being checked against.
        length: ValueReference,
        /// The collection being indexed.
        collection: ValueReference,
        /// Whether the index is treated as signed.
        is_signed: bool,
    },
    /// Null check on a reference.
    Null {
        /// The value being checked for null.
        value: ValueReference,
    },
    /// Division by zero check.
    DivZero {
        /// The divisor being checked for zero.
        divisor: ValueReference,
    },
    /// Shift amount range check.
    ShiftRange {
        /// The shift amount being checked.
        value: ValueReference,
        /// The bit width of the shifted type.
        bit_width: u8,
        /// Whether the shift amount is signed.
        is_signed: bool,
    },
    /// Integer narrowing check.
    Narrow {
        /// The value being narrowed.
        value: ValueReference,
        /// The target bit width.
        to_width: u8,
        /// Whether the narrowed value is signed.
        is_signed: bool,
    },
    /// Overflow check for an arithmetic operation.
    Overflow {
        /// The operator being checked.
        operator: BinaryOperator,
        /// The left operand.
        left: ValueReference,
        /// The right operand.
        right: ValueReference,
        /// Whether the overflow check is signed.
        is_signed: bool,
    },
    /// Runtime type descriptor check for a value.
    Type {
        /// The descriptor value being checked.
        value: ValueReference,
        /// The expected dynamic type for this descriptor.
        expected: TypeReference,
    },
    /// Variant tag check for a physical tagged sum value.
    Variant {
        /// The tag value being checked.
        value: ValueReference,
        /// The expected tag constant.
        expected: Constant,
    },
    /// Dynamic receiver type check for a class or concrete receiver.
    ReceiverType {
        /// The receiver being checked.
        receiver: ValueReference,
        /// The expected concrete receiver type.
        expected: TypeReference,
    },
    /// Interface conformance check for a receiver.
    Implements {
        /// The receiver being checked.
        receiver: ValueReference,
        /// The expected interface type.
        expected: TypeReference,
    },
}

impl CheckConstraint {
    /// Get values used by this check kind.
    pub fn uses(&self) -> SmallVec<[ValueReference; 4]> {
        // collect values referenced by the check kind
        match self {
            CheckConstraint::Bounds {
                index,
                length,
                collection,
                ..
            } => smallvec![*index, *length, *collection],
            CheckConstraint::Null { value } => smallvec![*value],
            CheckConstraint::DivZero { divisor } => smallvec![*divisor],
            CheckConstraint::ShiftRange { value, .. } => smallvec![*value],
            CheckConstraint::Narrow { value, .. } => smallvec![*value],
            CheckConstraint::Overflow { left, right, .. } => smallvec![*left, *right],
            CheckConstraint::Type { value, .. } => smallvec![*value],
            CheckConstraint::Variant { value, .. } => smallvec![*value],
            CheckConstraint::ReceiverType { receiver, .. } => smallvec![*receiver],
            CheckConstraint::Implements { receiver, .. } => smallvec![*receiver],
        }
    }
}

/// One case arm for a switch terminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitchCase {
    /// The matched case value.
    pub value: IntegerReference,
    /// The target block for this case.
    pub target: BlockTarget,
}

/// Block terminator node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Terminator {
    /// Recovered invalid terminator syntax.
    Error,
    /// Return from the function.
    Return {
        /// The value to return, or None for void functions.
        value: Option<ValueReference>,
    },

    /// Unconditional jump to another block.
    Jump {
        /// The block to jump to.
        target: BlockTarget,
    },
    /// Conditional branch.
    Branch {
        /// The boolean condition to test.
        condition: ValueReference,
        /// The block to jump to if condition is true.
        then_target: BlockTarget,
        /// The block to jump to if condition is false.
        else_target: BlockTarget,
    },
    /// Runtime check with explicit success and failure edges.
    Check {
        /// Semantic constraint for the check.
        constraint: CheckConstraint,
        /// The block to jump to when the check succeeds.
        success: BlockTarget,
        /// The block to jump to when the check fails.
        failure: BlockTarget,
    },
    /// Switch on an integer value.
    Switch {
        /// The integer value to switch on.
        value: ValueReference,
        /// The block to jump to if no case matches.
        default: BlockTarget,
        /// The cases to match against.
        cases: Vec<SwitchCase>,
    },

    /// Yield from a coroutine.
    Yield {
        /// The yielded value.
        value: ValueReference,
        /// The block to resume at when the coroutine is continued.
        resume: BlockTarget,
    },

    /// Direct call with an explicit continuation.
    Call {
        /// The direct callee function.
        function: FunctionReference,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
        /// The continuation block.
        target: BlockTarget,
        /// The cleanup block when this call panics.
        unwind: Option<BlockTarget>,
    },
    /// Indirect call with an explicit continuation.
    CallIndirect {
        /// The function pointer or closure value to call.
        callee: ValueReference,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
        /// The continuation block.
        target: BlockTarget,
        /// The cleanup block when this call panics.
        unwind: Option<BlockTarget>,
    },
    /// Class call with an explicit continuation.
    CallVirtual {
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The class type declaring this dispatch slot.
        class: TypeReference,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
        /// The continuation block.
        target: BlockTarget,
        /// The cleanup block when this call panics.
        unwind: Option<BlockTarget>,
    },
    /// Dynamic call with an explicit continuation.
    CallDynamic {
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The dynamic constraint type declaring this dispatch slot.
        constraint: TypeReference,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
        /// The continuation block.
        target: BlockTarget,
        /// The cleanup block when this call panics.
        unwind: Option<BlockTarget>,
    },

    /// Fallible zeroed typed heap allocation.
    NewZeroedTry {
        /// The type of the struct to allocate.
        layout: TypeReference,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },
    /// Fallible uninitialized typed heap allocation.
    NewUninitTry {
        /// The type of the struct to allocate.
        layout: TypeReference,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },
    /// Fallible zeroed slice backing allocation.
    NewSliceZeroedTry {
        /// The element type.
        element: TypeReference,
        /// The number of elements.
        length: ValueReference,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },
    /// Fallible uninitialized slice backing allocation.
    NewSliceUninitTry {
        /// The element type.
        element: TypeReference,
        /// The number of elements.
        length: ValueReference,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },

    /// Start language panic unwinding.
    Panic {
        /// Optional panic payload.
        payload: Option<ValueReference>,
    },
    /// Resume the active language panic after cleanup.
    ResumePanic,
    /// Unrecoverable runtime termination.
    Trap {
        /// The trap kind.
        kind: TrapKind,
        /// Optional trap payload.
        payload: Option<ValueReference>,
    },
    /// Unreachable code.
    Unreachable,

    /// Tail call to a function.
    TailCall {
        /// The function to tail call.
        function: FunctionReference,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
    },
    /// Tail call through a function pointer.
    TailCallIndirect {
        /// The function pointer or closure value to tail call.
        callee: ValueReference,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
    },
    /// Tail call through a virtual dispatch slot.
    TailCallVirtual {
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The class type declaring this dispatch slot.
        class: TypeReference,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
    },
    /// Tail call through a dynamic dispatch slot.
    TailCallDynamic {
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The dynamic constraint type declaring this dispatch slot.
        constraint: TypeReference,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
    },
}

impl Node for Terminator {
    const TYPE: NodeType = NodeType::Terminator;
}

impl Terminator {
    /// Return the dispatch kind when this terminator performs a call.
    pub fn call_dispatch_kind(&self) -> Option<CallDispatchKind> {
        match self {
            Terminator::Error => None,
            Terminator::Call { .. } | Terminator::TailCall { .. } => Some(CallDispatchKind::Direct),
            Terminator::CallIndirect { .. } | Terminator::TailCallIndirect { .. } => {
                Some(CallDispatchKind::Indirect)
            }
            Terminator::CallVirtual { slot, .. } | Terminator::TailCallVirtual { slot, .. } => {
                Some(CallDispatchKind::Virtual { slot: *slot })
            }
            Terminator::CallDynamic { slot, .. } | Terminator::TailCallDynamic { slot, .. } => {
                Some(CallDispatchKind::Dynamic { slot: *slot })
            }
            _ => None,
        }
    }

    /// Return the call signature when this terminator performs a call.
    pub fn call_signature(&self) -> Option<TypeReference> {
        match self {
            Terminator::Error => None,
            Terminator::Call { call, .. }
            | Terminator::CallIndirect { call, .. }
            | Terminator::CallVirtual { call, .. }
            | Terminator::CallDynamic { call, .. }
            | Terminator::TailCall { call, .. }
            | Terminator::TailCallIndirect { call, .. }
            | Terminator::TailCallVirtual { call, .. }
            | Terminator::TailCallDynamic { call, .. } => Some(call.signature.clone()),
            _ => None,
        }
    }

    /// Return the direct target when this terminator performs a call.
    pub fn call_direct_target(&self) -> Option<FunctionReference> {
        match self {
            Terminator::Error => None,
            Terminator::Call { function, .. } | Terminator::TailCall { function, .. } => {
                Some(*function)
            }
            _ => None,
        }
    }

    /// Get all successor block ids.
    pub fn successors(&self) -> SmallVec<[BlockReference; 2]> {
        match self {
            Terminator::Error => smallvec![],
            Terminator::Return { .. } => smallvec![],
            Terminator::Jump { target, .. } => smallvec![target.block],
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => smallvec![then_target.block, else_target.block],
            Terminator::Check {
                success, failure, ..
            } => smallvec![success.block, failure.block],
            Terminator::Switch { default, cases, .. } => {
                let mut successors = smallvec![default.block];
                successors.extend(cases.iter().map(|case| case.target.block));
                successors
            }
            Terminator::Yield { resume, .. } => smallvec![resume.block],
            Terminator::Call { target, unwind, .. }
            | Terminator::CallIndirect { target, unwind, .. }
            | Terminator::CallVirtual { target, unwind, .. }
            | Terminator::CallDynamic { target, unwind, .. } => {
                let mut successors = smallvec![target.block];
                if let Some(unwind) = unwind {
                    successors.push(unwind.block);
                }
                successors
            }
            Terminator::NewZeroedTry {
                success, failure, ..
            }
            | Terminator::NewUninitTry {
                success, failure, ..
            }
            | Terminator::NewSliceZeroedTry {
                success, failure, ..
            }
            | Terminator::NewSliceUninitTry {
                success, failure, ..
            } => smallvec![success.block, failure.block],
            Terminator::Panic { .. } => smallvec![],
            Terminator::ResumePanic => smallvec![],
            Terminator::Trap { .. } => smallvec![],
            Terminator::Unreachable => smallvec![],
            Terminator::TailCall { .. } => smallvec![],
            Terminator::TailCallIndirect { .. } => smallvec![],
            Terminator::TailCallVirtual { .. } => smallvec![],
            Terminator::TailCallDynamic { .. } => smallvec![],
        }
    }

    /// Get all SSA values used by this terminator.
    pub fn uses(&self) -> SmallVec<[ValueReference; 8]> {
        match self {
            Terminator::Error => smallvec![],
            Terminator::Return { value } => value.iter().copied().collect(),
            Terminator::Jump { target, .. } => target.arguments.iter().copied().collect(),
            Terminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => {
                let mut uses = smallvec![*condition];
                uses.extend(then_target.arguments.iter().copied());
                uses.extend(else_target.arguments.iter().copied());
                uses
            }
            Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                let mut uses = constraint
                    .uses()
                    .into_iter()
                    .collect::<SmallVec<[ValueReference; 8]>>();
                uses.extend(success.arguments.iter().copied());
                uses.extend(failure.arguments.iter().copied());
                uses
            }
            Terminator::Switch {
                value,
                default,
                cases,
                ..
            } => {
                let mut uses = smallvec![*value];
                uses.extend(default.arguments.iter().copied());
                for case in cases {
                    uses.extend(case.target.arguments.iter().copied());
                }
                uses
            }
            Terminator::Yield { value, resume, .. } => {
                let mut uses = smallvec![*value];
                uses.extend(resume.arguments.iter().copied());
                uses
            }
            Terminator::Call {
                call,
                target,
                unwind,
                ..
            } => {
                let mut uses = call
                    .arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[ValueReference; 8]>>();
                uses.extend(target.arguments.iter().copied());
                if let Some(unwind) = unwind {
                    uses.extend(unwind.arguments.iter().copied());
                }
                uses
            }
            Terminator::CallIndirect {
                callee,
                call,
                target,
                unwind,
                ..
            } => {
                let mut uses = smallvec![*callee];
                uses.extend(call.arguments.iter().copied());
                uses.extend(target.arguments.iter().copied());
                if let Some(unwind) = unwind {
                    uses.extend(unwind.arguments.iter().copied());
                }
                uses
            }
            Terminator::CallVirtual {
                receiver,
                call,
                target,
                unwind,
                ..
            } => {
                let mut uses = smallvec![*receiver];
                uses.extend(call.arguments.iter().copied());
                uses.extend(target.arguments.iter().copied());
                if let Some(unwind) = unwind {
                    uses.extend(unwind.arguments.iter().copied());
                }
                uses
            }
            Terminator::CallDynamic {
                receiver,
                call,
                target,
                unwind,
                ..
            } => {
                let mut uses = smallvec![*receiver];
                uses.extend(call.arguments.iter().copied());
                uses.extend(target.arguments.iter().copied());
                if let Some(unwind) = unwind {
                    uses.extend(unwind.arguments.iter().copied());
                }
                uses
            }
            Terminator::NewZeroedTry {
                success, failure, ..
            }
            | Terminator::NewUninitTry {
                success, failure, ..
            } => {
                let mut uses = smallvec![];
                uses.extend(success.arguments.iter().copied());
                uses.extend(failure.arguments.iter().copied());
                uses
            }
            Terminator::NewSliceZeroedTry {
                length,
                success,
                failure,
                ..
            }
            | Terminator::NewSliceUninitTry {
                length,
                success,
                failure,
                ..
            } => {
                let mut uses = smallvec![*length];
                uses.extend(success.arguments.iter().copied());
                uses.extend(failure.arguments.iter().copied());
                uses
            }
            Terminator::Panic { payload } => payload.iter().copied().collect(),
            Terminator::ResumePanic => smallvec![],
            Terminator::Trap { payload, .. } => payload.iter().copied().collect(),
            Terminator::Unreachable => smallvec![],
            Terminator::TailCall { call, .. } => call.arguments.iter().copied().collect(),
            Terminator::TailCallIndirect { callee, call, .. } => {
                let mut uses = smallvec![*callee];
                uses.extend(call.arguments.iter().copied());
                uses
            }
            Terminator::TailCallVirtual { receiver, call, .. }
            | Terminator::TailCallDynamic { receiver, call, .. } => {
                let mut uses = smallvec![*receiver];
                uses.extend(call.arguments.iter().copied());
                uses
            }
        }
    }
}
