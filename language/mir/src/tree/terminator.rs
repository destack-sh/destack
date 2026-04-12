use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    BinaryOperator, Block, BlockReference, Call, Function, FunctionReference, IntegerReference,
    InterfaceSlotId, Node, NodeType, TypeReference, ValueReference, VtableSlotId,
};

/// One control-flow edge target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockTarget {
    /// The block to transfer control to.
    pub block: BlockReference,
    /// Arguments for the target block's parameters.
    pub arguments: Vec<ValueReference>,
}

/// Unrecoverable runtime termination kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrapKind {
    /// Abort execution immediately without a payload.
    Abort,
    /// Panic with a runtime payload.
    Panic,
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
    /// Union tag check for a discriminated union value.
    Union {
        /// The tag value being checked.
        value: ValueReference,
        /// The expected tag index.
        expected: u64,
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
            CheckConstraint::Union { value, .. } => smallvec![*value],
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
    /// Direct call with explicit success and exception continuations.
    Invoke {
        /// The direct callee function.
        function: FunctionReference,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
        /// The success continuation block.
        normal_target: BlockTarget,
        /// The exception continuation block.
        unwind_target: BlockTarget,
    },
    /// Indirect call with explicit success and exception continuations.
    InvokeIndirect {
        /// The callable value to call.
        callee: ValueReference,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
        /// The success continuation block.
        normal_target: BlockTarget,
        /// The exception continuation block.
        unwind_target: BlockTarget,
    },
    /// Virtual call with explicit success and exception continuations.
    InvokeVirtual {
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The declaring type for this virtual call.
        declaring_type: TypeReference,
        /// The vtable slot id for the method.
        slot_id: VtableSlotId,
        /// The declared method target when known.
        declared_target: Option<FunctionReference>,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
        /// The success continuation block.
        normal_target: BlockTarget,
        /// The exception continuation block.
        unwind_target: BlockTarget,
    },
    /// Interface call with explicit success and exception continuations.
    InvokeInterface {
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The declaring interface type for this call.
        declaring_type: TypeReference,
        /// The interface slot id for the method.
        slot_id: InterfaceSlotId,
        /// The declared method target when known.
        declared_target: Option<FunctionReference>,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
        /// The success continuation block.
        normal_target: BlockTarget,
        /// The exception continuation block.
        unwind_target: BlockTarget,
    },
    /// Throw a managed exception object.
    Throw {
        /// The thrown exception payload.
        value: ValueReference,
    },
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
        /// The callable value to tail call.
        callee: ValueReference,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
    },
    /// Tail call through a virtual dispatch slot.
    TailCallVirtual {
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The declaring type for this virtual call.
        declaring_type: TypeReference,
        /// The vtable slot id for the method.
        slot_id: VtableSlotId,
        /// The declared method target when known.
        declared_target: Option<FunctionReference>,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
    },
    /// Tail call through an interface dispatch slot.
    TailCallInterface {
        /// The receiver value for dispatch.
        receiver: ValueReference,
        /// The declaring interface type for this call.
        declaring_type: TypeReference,
        /// The interface slot id for the method.
        slot_id: InterfaceSlotId,
        /// The declared method target when known.
        declared_target: Option<FunctionReference>,
        /// The shared call payload.
        call: Call<Vec<ValueReference>>,
    },
}

impl Node for Terminator {
    const TYPE: NodeType = NodeType::Terminator;
}

impl Terminator {
    /// Return the dispatch kind when this terminator performs a call.
    pub fn call_dispatch_kind(&self) -> Option<crate::CallDispatchKind> {
        match self {
            Terminator::Error => None,
            Terminator::Invoke { .. } | Terminator::TailCall { .. } => {
                Some(crate::CallDispatchKind::Direct)
            }
            Terminator::InvokeIndirect { .. } | Terminator::TailCallIndirect { .. } => {
                Some(crate::CallDispatchKind::Indirect)
            }
            Terminator::InvokeVirtual { slot_id, .. }
            | Terminator::TailCallVirtual { slot_id, .. } => {
                Some(crate::CallDispatchKind::Virtual { slot_id: *slot_id })
            }
            Terminator::InvokeInterface { slot_id, .. }
            | Terminator::TailCallInterface { slot_id, .. } => {
                Some(crate::CallDispatchKind::Interface { slot_id: *slot_id })
            }
            _ => None,
        }
    }

    /// Return the call signature when this terminator performs a call.
    pub fn call_signature(&self) -> Option<TypeReference> {
        match self {
            Terminator::Error => None,
            Terminator::Invoke { call, .. }
            | Terminator::InvokeIndirect { call, .. }
            | Terminator::InvokeVirtual { call, .. }
            | Terminator::InvokeInterface { call, .. }
            | Terminator::TailCall { call, .. }
            | Terminator::TailCallIndirect { call, .. }
            | Terminator::TailCallVirtual { call, .. }
            | Terminator::TailCallInterface { call, .. } => Some(call.signature),
            _ => None,
        }
    }

    /// Return the declared target when this terminator performs a call.
    pub fn call_declared_target(&self) -> Option<FunctionReference> {
        match self {
            Terminator::Error => None,
            Terminator::Invoke { function, .. } | Terminator::TailCall { function, .. } => {
                Some(*function)
            }
            Terminator::InvokeVirtual {
                declared_target, ..
            }
            | Terminator::InvokeInterface {
                declared_target, ..
            }
            | Terminator::TailCallVirtual {
                declared_target, ..
            }
            | Terminator::TailCallInterface {
                declared_target, ..
            } => *declared_target,
            _ => None,
        }
    }

    /// Get all successor block ids.
    pub fn successors(&self) -> SmallVec<[BlockReference; 2]> {
        match self {
            Terminator::Error => smallvec![],
            Terminator::Return { .. } => smallvec![],
            Terminator::Jump { target, .. } => smallvec![*target],
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => smallvec![*then_target, *else_target],
            Terminator::Check {
                success, failure, ..
            } => smallvec![success.block, failure.block],
            Terminator::Switch { default, cases, .. } => {
                let mut successors = smallvec![*default];
                successors.extend(cases.iter().map(|case| case.target.block));
                successors
            }
            Terminator::Yield { resume, .. } => smallvec![*resume],
            Terminator::Invoke {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeInterface {
                normal_target,
                unwind_target,
                ..
            } => smallvec![normal_target.block, unwind_target.block],
            Terminator::Throw { .. } => smallvec![],
            Terminator::Trap { .. } => smallvec![],
            Terminator::Unreachable => smallvec![],
            Terminator::TailCall { .. } => smallvec![],
            Terminator::TailCallIndirect { .. } => smallvec![],
            Terminator::TailCallVirtual { .. } => smallvec![],
            Terminator::TailCallInterface { .. } => smallvec![],
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
            Terminator::Invoke {
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                let mut uses = call
                    .arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[ValueReference; 8]>>();
                uses.extend(normal_target.arguments.iter().copied());
                uses.extend(unwind_target.arguments.iter().copied());
                uses
            }
            Terminator::InvokeIndirect {
                callee,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                let mut uses = smallvec![*callee];
                uses.extend(call.arguments.iter().copied());
                uses.extend(normal_target.arguments.iter().copied());
                uses.extend(unwind_target.arguments.iter().copied());
                uses
            }
            Terminator::InvokeVirtual {
                receiver,
                call,
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeInterface {
                receiver,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                let mut uses = smallvec![*receiver];
                uses.extend(call.arguments.iter().copied());
                uses.extend(normal_target.arguments.iter().copied());
                uses.extend(unwind_target.arguments.iter().copied());
                uses
            }
            Terminator::Throw { value } => smallvec![*value],
            Terminator::Trap { payload, .. } => payload.iter().copied().collect(),
            Terminator::Unreachable => smallvec![],
            Terminator::TailCall { call, .. } => call.arguments.iter().copied().collect(),
            Terminator::TailCallIndirect { callee, call, .. } => {
                let mut uses = smallvec![*callee];
                uses.extend(call.arguments.iter().copied());
                uses
            }
            Terminator::TailCallVirtual { receiver, call, .. }
            | Terminator::TailCallInterface { receiver, call, .. } => {
                let mut uses = smallvec![*receiver];
                uses.extend(call.arguments.iter().copied());
                uses
            }
        }
    }
}
