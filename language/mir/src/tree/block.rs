use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    BinaryOperator, Call, Function, Instruction, InterfaceSlotId, LocalNodeId, Node, NodeType,
    Type, TypedValue, Value, VtableSlotId,
};

/// A basic block is a sequence of instructions with:
/// - A single entry point (can have parameters for SSA)
/// - A single exit point (the terminator)
/// - No control flow within the block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// SSA parameters passed from predecessor blocks.
    /// Replaces traditional phi nodes with a cleaner model.
    pub parameters: Vec<TypedValue>,
    /// Instructions in execution order.
    pub instructions: Vec<LocalNodeId<Instruction>>,
    /// How control flow leaves this block.
    pub terminator: Terminator,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

impl Block {
    /// Create a new empty block.
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
        }
    }

    /// Create a block with parameters.
    pub fn with_parameters(parameters: Vec<TypedValue>) -> Self {
        Self {
            parameters,
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
        }
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

/// Target and arguments for a check edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckTarget {
    /// The block to transfer control to.
    pub target: LocalNodeId<Block>,
    /// Arguments for the target block's parameters.
    pub arguments: Vec<Value>,
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
        index: Value,
        /// The length being checked against.
        length: Value,
        /// The collection being indexed.
        collection: Value,
        /// Whether the index is treated as signed.
        is_signed: bool,
    },
    /// Null check on a reference.
    Null {
        /// The value being checked for null.
        value: Value,
    },
    /// Division by zero check.
    DivZero {
        /// The divisor being checked for zero.
        divisor: Value,
    },
    /// Shift amount range check.
    ShiftRange {
        /// The shift amount being checked.
        value: Value,
        /// The bit width of the shifted type.
        bit_width: u8,
        /// Whether the shift amount is signed.
        is_signed: bool,
    },
    /// Integer narrowing check.
    Narrow {
        /// The value being narrowed.
        value: Value,
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
        left: Value,
        /// The right operand.
        right: Value,
        /// Whether the overflow check is signed.
        is_signed: bool,
    },
    /// Runtime type descriptor check for a value.
    Type {
        /// The descriptor value being checked.
        value: Value,
        /// The expected dynamic type for this descriptor.
        expected: LocalNodeId<Type>,
    },
    /// Union tag check for a discriminated union value.
    Union {
        /// The tag value being checked.
        value: Value,
        /// The expected tag index.
        expected: u64,
    },
    /// Dynamic receiver type check for a class or concrete receiver.
    ReceiverType {
        /// The receiver being checked.
        receiver: Value,
        /// The expected concrete receiver type.
        expected: LocalNodeId<Type>,
    },
    /// Interface conformance check for a receiver.
    Implements {
        /// The receiver being checked.
        receiver: Value,
        /// The expected interface type.
        expected: LocalNodeId<Type>,
    },
}

impl CheckConstraint {
    /// Get values used by this check kind.
    pub fn uses(&self) -> SmallVec<[Value; 4]> {
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

/// Block terminator - how control flow leaves a block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Terminator {
    /// Return from the function.
    Return {
        /// The value to return, or None for void functions.
        value: Option<Value>,
    },

    /// Unconditional jump to another block.
    Jump {
        /// The block to jump to.
        target: LocalNodeId<Block>,
        /// Arguments to pass to the target block's parameters.
        arguments: Vec<Value>,
    },

    /// Conditional branch (if-then-else).
    Branch {
        /// The boolean condition to test.
        condition: Value,
        /// The block to jump to if condition is true.
        then_target: LocalNodeId<Block>,
        /// Arguments for the then block's parameters.
        then_arguments: Vec<Value>,
        /// The block to jump to if condition is false.
        else_target: LocalNodeId<Block>,
        /// Arguments for the else block's parameters.
        else_arguments: Vec<Value>,
    },

    /// Runtime check with explicit success and failure edges.
    Check {
        /// Semantic constraint for the check.
        constraint: CheckConstraint,
        /// The block to jump to when the check succeeds.
        success: CheckTarget,
        /// The block to jump to when the check fails.
        failure: CheckTarget,
    },

    /// Switch on an integer value (multi-way branch).
    Switch {
        /// The integer value to switch on.
        value: Value,
        /// The block to jump to if no case matches.
        default: LocalNodeId<Block>,
        /// Arguments for the default block's parameters.
        default_arguments: Vec<Value>,
        /// The cases to match against.
        cases: Vec<SwitchCase>,
    },

    /// Yield from a coroutine (generator or async function).
    ///
    /// Suspends execution, yielding a value to the caller.
    /// When resumed, execution continues at the resume block with the resumed value.
    ///
    /// For generators: `yield value` suspends and returns value to caller.
    /// For async: `await promise` suspends until promise resolves.
    /// The function suspension kind determines the exact semantics.
    Yield {
        /// The value to yield (for generators) or the promise to await (for async).
        value: Value,
        /// The block to resume at when the coroutine is continued.
        resume: LocalNodeId<Block>,
        /// Arguments to pass to the resume block's leading parameters.
        /// The resumed value (from `.next(arg)` or resolved promise) follows these arguments.
        resume_arguments: Vec<Value>,
    },

    /// Direct call with explicit success and exception continuations.
    ///
    /// The success target receives the call result as its leading block parameter
    /// when the callee returns a non-void value.
    /// The exception target receives the thrown managed exception object as its
    /// leading block parameter.
    Invoke {
        /// The direct callee function.
        function: LocalNodeId<Function>,
        /// The shared call payload.
        call: Call<Vec<Value>>,
        /// The success continuation block.
        normal_target: LocalNodeId<Block>,
        /// Arguments for the success continuation after the implicit result.
        normal_arguments: Vec<Value>,
        /// The exception continuation block.
        unwind_target: LocalNodeId<Block>,
        /// Arguments for the exception continuation after the implicit exception.
        unwind_arguments: Vec<Value>,
    },

    /// Indirect call with explicit success and exception continuations.
    ///
    /// The success target receives the call result as its leading block parameter
    /// when the callee returns a non-void value.
    /// The exception target receives the thrown managed exception object as its
    /// leading block parameter.
    InvokeIndirect {
        /// The callable value to call.
        callee: Value,
        /// The shared call payload.
        call: Call<Vec<Value>>,
        /// The success continuation block.
        normal_target: LocalNodeId<Block>,
        /// Arguments for the success continuation after the implicit result.
        normal_arguments: Vec<Value>,
        /// The exception continuation block.
        unwind_target: LocalNodeId<Block>,
        /// Arguments for the exception continuation after the implicit exception.
        unwind_arguments: Vec<Value>,
    },

    /// Virtual call with explicit success and exception continuations.
    ///
    /// The success target receives the call result as its leading block parameter
    /// when the callee returns a non-void value.
    /// The exception target receives the thrown managed exception object as its
    /// leading block parameter.
    InvokeVirtual {
        /// The receiver value for dispatch.
        receiver: Value,
        /// The declaring type for this virtual call.
        declaring_type: LocalNodeId<Type>,
        /// The vtable slot id for the method.
        slot_id: VtableSlotId,
        /// The declared method target when known.
        declared_target: Option<LocalNodeId<Function>>,
        /// The shared call payload.
        call: Call<Vec<Value>>,
        /// The success continuation block.
        normal_target: LocalNodeId<Block>,
        /// Arguments for the success continuation after the implicit result.
        normal_arguments: Vec<Value>,
        /// The exception continuation block.
        unwind_target: LocalNodeId<Block>,
        /// Arguments for the exception continuation after the implicit exception.
        unwind_arguments: Vec<Value>,
    },

    /// Interface call with explicit success and exception continuations.
    ///
    /// The success target receives the call result as its leading block parameter
    /// when the callee returns a non-void value.
    /// The exception target receives the thrown managed exception object as its
    /// leading block parameter.
    InvokeInterface {
        /// The receiver value for dispatch.
        receiver: Value,
        /// The declaring interface type for this call.
        declaring_type: LocalNodeId<Type>,
        /// The interface slot id for the method.
        slot_id: InterfaceSlotId,
        /// The declared method target when known.
        declared_target: Option<LocalNodeId<Function>>,
        /// The shared call payload.
        call: Call<Vec<Value>>,
        /// The success continuation block.
        normal_target: LocalNodeId<Block>,
        /// Arguments for the success continuation after the implicit result.
        normal_arguments: Vec<Value>,
        /// The exception continuation block.
        unwind_target: LocalNodeId<Block>,
        /// Arguments for the exception continuation after the implicit exception.
        unwind_arguments: Vec<Value>,
    },

    /// Throw a managed exception object.
    Throw {
        /// The thrown exception payload.
        value: Value,
    },

    /// Unrecoverable runtime termination.
    Trap {
        /// The trap kind.
        kind: TrapKind,
        /// Optional trap payload.
        payload: Option<Value>,
    },

    /// Unreachable code (triggers undefined behavior if executed).
    Unreachable,

    /// Tail call to a function (does not return to this function).
    ///
    /// The callee's return value becomes this function's return value.
    /// Codegen can reuse the current stack frame. Semantically equivalent
    /// to `call` followed by `return`, but enables stack frame reuse.
    TailCall {
        /// The function to tail call.
        function: LocalNodeId<Function>,
        /// The shared call payload.
        call: Call<Vec<Value>>,
    },

    /// Tail call through a function pointer (does not return to this function).
    ///
    /// The callee's return value becomes this function's return value.
    /// Codegen can reuse the current stack frame.
    TailCallIndirect {
        /// The callable value to tail call.
        callee: Value,
        /// The shared call payload.
        call: Call<Vec<Value>>,
    },
    /// Tail call through a virtual dispatch slot.
    ///
    /// The callee's return value becomes this function's return value.
    TailCallVirtual {
        /// The receiver value for dispatch.
        receiver: Value,
        /// The declaring type for this virtual call.
        declaring_type: LocalNodeId<Type>,
        /// The vtable slot id for the method.
        slot_id: VtableSlotId,
        /// The declared method target when known.
        declared_target: Option<LocalNodeId<Function>>,
        /// The shared call payload.
        call: Call<Vec<Value>>,
    },
    /// Tail call through an interface dispatch slot.
    ///
    /// The callee's return value becomes this function's return value.
    TailCallInterface {
        /// The receiver value for dispatch.
        receiver: Value,
        /// The declaring interface type for this call.
        declaring_type: LocalNodeId<Type>,
        /// The interface slot id for the method.
        slot_id: InterfaceSlotId,
        /// The declared method target when known.
        declared_target: Option<LocalNodeId<Function>>,
        /// The shared call payload.
        call: Call<Vec<Value>>,
    },
}

impl Terminator {
    /// Return the dispatch kind when this terminator performs a call.
    pub fn call_dispatch_kind(&self) -> Option<crate::CallDispatchKind> {
        match self {
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
    pub fn call_signature(&self) -> Option<LocalNodeId<Type>> {
        match self {
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
    pub fn call_declared_target(&self) -> Option<LocalNodeId<Function>> {
        match self {
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
    pub fn successors(&self) -> SmallVec<[LocalNodeId<Block>; 2]> {
        match self {
            Terminator::Return { .. } => smallvec![],
            Terminator::Jump { target, .. } => smallvec![*target],
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => smallvec![*then_target, *else_target],
            Terminator::Check {
                success, failure, ..
            } => {
                smallvec![success.target, failure.target]
            }
            Terminator::Switch { default, cases, .. } => {
                let mut successors = smallvec![*default];
                successors.extend(cases.iter().map(|c| c.target));
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
            } => smallvec![*normal_target, *unwind_target],
            Terminator::Throw { .. } => smallvec![],
            Terminator::Trap { .. } => smallvec![],
            Terminator::Unreachable => smallvec![],
            // tail calls don't return to this function, so no successors
            Terminator::TailCall { .. } => smallvec![],
            Terminator::TailCallIndirect { .. } => smallvec![],
            Terminator::TailCallVirtual { .. } => smallvec![],
            Terminator::TailCallInterface { .. } => smallvec![],
        }
    }

    /// Get all values used by this terminator.
    pub fn uses(&self) -> SmallVec<[Value; 4]> {
        match self {
            Terminator::Return { value } => value.iter().copied().collect(),
            Terminator::Jump { arguments, .. } => arguments.iter().copied().collect(),
            Terminator::Branch {
                condition,
                then_arguments,
                else_arguments,
                ..
            } => {
                let mut uses = smallvec![*condition];
                uses.extend(then_arguments.iter().copied());
                uses.extend(else_arguments.iter().copied());
                uses
            }
            Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                let mut uses = constraint.uses();
                uses.extend(success.arguments.iter().copied());
                uses.extend(failure.arguments.iter().copied());
                uses
            }
            Terminator::Switch {
                value,
                default_arguments,
                cases,
                ..
            } => {
                let mut uses = smallvec![*value];
                uses.extend(default_arguments.iter().copied());
                for case in cases {
                    uses.extend(case.arguments.iter().copied());
                }
                uses
            }
            Terminator::Yield {
                value,
                resume_arguments,
                ..
            } => {
                let mut uses = smallvec![*value];
                uses.extend(resume_arguments.iter().copied());
                uses
            }
            Terminator::Invoke {
                call,
                normal_arguments,
                unwind_arguments,
                ..
            } => {
                let mut uses: SmallVec<[Value; 4]> = call.arguments.iter().copied().collect();
                uses.extend(normal_arguments.iter().copied());
                uses.extend(unwind_arguments.iter().copied());
                uses
            }
            Terminator::InvokeIndirect {
                callee,
                call,
                normal_arguments,
                unwind_arguments,
                ..
            } => {
                let mut uses = smallvec![*callee];
                uses.extend(call.arguments.iter().copied());
                uses.extend(normal_arguments.iter().copied());
                uses.extend(unwind_arguments.iter().copied());
                uses
            }
            Terminator::InvokeVirtual {
                receiver,
                call,
                normal_arguments,
                unwind_arguments,
                ..
            }
            | Terminator::InvokeInterface {
                receiver,
                call,
                normal_arguments,
                unwind_arguments,
                ..
            } => {
                let mut uses = smallvec![*receiver];
                uses.extend(call.arguments.iter().copied());
                uses.extend(normal_arguments.iter().copied());
                uses.extend(unwind_arguments.iter().copied());
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
            Terminator::TailCallVirtual { receiver, call, .. } => {
                let mut uses = smallvec![*receiver];
                uses.extend(call.arguments.iter().copied());
                uses
            }
            Terminator::TailCallInterface { receiver, call, .. } => {
                let mut uses = smallvec![*receiver];
                uses.extend(call.arguments.iter().copied());
                uses
            }
        }
    }
}

/// A case in a switch terminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitchCase {
    /// The integer constant to match against.
    pub value: i64,
    /// The block to jump to if this case matches.
    pub target: LocalNodeId<Block>,
    /// Arguments for the target block's parameters.
    pub arguments: Vec<Value>,
}
