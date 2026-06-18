use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    BinaryOperator, Block, BlockId, BlockParameter, Call, CallDispatchKind, Constant, DispatchSlot,
    FunctionId, LocalNodeId, Node, NodeType, Tree, TypeId, Value, ValueSlice,
};

/// One control-flow edge target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockTarget {
    /// The block to transfer control to.
    pub block: BlockId,
    /// Arguments for the target block's parameters.
    pub arguments: ValueSlice,
}

impl BlockTarget {
    /// Create a control-flow edge target.
    pub fn new(block: BlockId, arguments: ValueSlice) -> Self {
        Self { block, arguments }
    }

    /// Return target arguments.
    #[inline]
    pub fn arguments<'a>(&self, tree: &'a Tree) -> &'a [Value] {
        tree.get_values(self.arguments)
    }
}

/// Edge argument lookup result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeArguments<'a> {
    /// No edge to the successor was found.
    Missing,
    /// The edge arguments.
    Found(&'a [Value]),
    /// Multiple edges to the successor disagree on arguments.
    Conflict,
}

impl<'a> EdgeArguments<'a> {
    /// Merge arguments from one target when it reaches a successor.
    fn merge_target(
        &mut self,
        target: &BlockTarget,
        successor: LocalNodeId<Block>,
        tree: &'a Tree,
    ) {
        if Some(target.block) != Some(successor) {
            return;
        }

        let arguments = target.arguments(tree);
        *self = match *self {
            Self::Missing => Self::Found(arguments),
            Self::Found(existing) if existing == arguments => Self::Found(existing),
            Self::Found(_) | Self::Conflict => Self::Conflict,
        };
    }
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
        expected: TypeId,
    },
    /// Variant tag check for a physical tagged sum value.
    Variant {
        /// The tag value being checked.
        value: Value,
        /// The expected tag constant.
        expected: Constant,
    },
    /// Dynamic receiver type check for a class or concrete receiver.
    ReceiverType {
        /// The receiver being checked.
        receiver: Value,
        /// The expected concrete receiver type.
        expected: TypeId,
    },
    /// Interface conformance check for a receiver.
    Implements {
        /// The receiver being checked.
        receiver: Value,
        /// The expected interface type.
        expected: TypeId,
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
    pub value: i128,
    /// The target block for this case.
    pub target: BlockTarget,
}

/// Compact reference to a switch case list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SwitchCaseSlice {
    /// Start index in the switch case buffer.
    pub start: u32,
    /// Number of switch cases in the slice.
    pub count: u16,
}

impl SwitchCaseSlice {
    /// Create a new switch case slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Return whether this slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the number of switch cases in this slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}

/// Block terminator node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Terminator {
    /// Recovered invalid terminator syntax.
    Error,
    /// Return from the function.
    Return {
        /// The value to return, or None for void functions.
        value: Option<Value>,
    },

    /// Unconditional jump to another block.
    Jump {
        /// The block to jump to.
        target: BlockTarget,
    },
    /// Conditional branch.
    Branch {
        /// The boolean condition to test.
        condition: Value,
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
        value: Value,
        /// The block to jump to if no case matches.
        default: BlockTarget,
        /// The cases to match against.
        cases: SwitchCaseSlice,
    },

    /// Yield from a coroutine.
    Yield {
        /// The yielded value.
        value: Value,
        /// The block to resume at when the coroutine is continued.
        resume: BlockTarget,
        /// The cleanup block when the suspended frame is cancelled or dropped.
        unwind: Option<BlockTarget>,
    },

    /// Direct call with an explicit continuation.
    Call {
        /// The direct callee function.
        function: FunctionId,
        /// The shared call payload.
        call: Call<ValueSlice>,
        /// The continuation block.
        target: BlockTarget,
        /// The cleanup block when this call panics.
        unwind: Option<BlockTarget>,
    },
    /// Indirect call with an explicit continuation.
    CallIndirect {
        /// The function pointer or closure value to call.
        callee: Value,
        /// The shared call payload.
        call: Call<ValueSlice>,
        /// The continuation block.
        target: BlockTarget,
        /// The cleanup block when this call panics.
        unwind: Option<BlockTarget>,
    },
    /// Class call with an explicit continuation.
    CallVirtual {
        /// The receiver value for dispatch.
        receiver: Value,
        /// The class type declaring this dispatch slot.
        class: TypeId,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<ValueSlice>,
        /// The continuation block.
        target: BlockTarget,
        /// The cleanup block when this call panics.
        unwind: Option<BlockTarget>,
    },
    /// Dynamic call with an explicit continuation.
    CallDynamic {
        /// The receiver value for dispatch.
        receiver: Value,
        /// The dynamic constraint type declaring this dispatch slot.
        constraint: TypeId,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<ValueSlice>,
        /// The continuation block.
        target: BlockTarget,
        /// The cleanup block when this call panics.
        unwind: Option<BlockTarget>,
    },

    /// Fallible zeroed typed heap allocation.
    NewZeroedTry {
        /// The type of the struct to allocate.
        layout: TypeId,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },
    /// Fallible uninitialized typed heap allocation.
    NewUninitTry {
        /// The type of the struct to allocate.
        layout: TypeId,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },
    /// Fallible zeroed slice backing allocation.
    NewSliceZeroedTry {
        /// The element type.
        element: TypeId,
        /// The number of elements.
        length: Value,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },
    /// Fallible uninitialized slice backing allocation.
    NewSliceUninitTry {
        /// The element type.
        element: TypeId,
        /// The number of elements.
        length: Value,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },

    /// Start language panic unwinding.
    Panic {
        /// Optional panic payload.
        payload: Option<Value>,
    },
    /// Continue the active unwind after a cleanup block.
    // TODO #Incomplete: a panic during cleanup must abort, nothing enforces that yet
    UnwindResume,
    /// Unrecoverable runtime termination.
    Trap {
        /// The trap kind.
        kind: TrapKind,
        /// Optional trap payload.
        payload: Option<Value>,
    },
    /// Unreachable code.
    Unreachable,

    /// Tail call to a function.
    // TODO #Incomplete: verify that no cleanup is live at tail calls, the frame the
    // cleanup lives in is thrown away (applies to all tail call variants)
    TailCall {
        /// The function to tail call.
        function: FunctionId,
        /// The shared call payload.
        call: Call<ValueSlice>,
    },
    /// Tail call through a function pointer.
    TailCallIndirect {
        /// The function pointer or closure value to tail call.
        callee: Value,
        /// The shared call payload.
        call: Call<ValueSlice>,
    },
    /// Tail call through a virtual dispatch slot.
    TailCallVirtual {
        /// The receiver value for dispatch.
        receiver: Value,
        /// The class type declaring this dispatch slot.
        class: TypeId,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<ValueSlice>,
    },
    /// Tail call through a dynamic dispatch slot.
    TailCallDynamic {
        /// The receiver value for dispatch.
        receiver: Value,
        /// The dynamic constraint type declaring this dispatch slot.
        constraint: TypeId,
        /// The dispatch slot for the method.
        slot: DispatchSlot,
        /// The shared call payload.
        call: Call<ValueSlice>,
    },
}

impl Node for Terminator {
    const TYPE: NodeType = NodeType::Terminator;
}

impl Terminator {
    /// Return all successor blocks.
    pub fn successors(&self, tree: &Tree) -> SmallVec<[BlockId; 2]> {
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
                successors.extend(
                    tree.get_switch_cases(*cases)
                        .iter()
                        .map(|case| case.target.block),
                );

                successors
            }
            Terminator::Yield { resume, unwind, .. } => {
                let mut successors = smallvec![resume.block];
                if let Some(unwind) = unwind {
                    successors.push(unwind.block);
                }

                successors
            }
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
            Terminator::UnwindResume => smallvec![],
            Terminator::Trap { .. } => smallvec![],
            Terminator::Unreachable => smallvec![],
            Terminator::TailCall { .. } => smallvec![],
            Terminator::TailCallIndirect { .. } => smallvec![],
            Terminator::TailCallVirtual { .. } => smallvec![],
            Terminator::TailCallDynamic { .. } => smallvec![],
        }
    }

    /// Return all SSA values used by this terminator.
    pub fn uses(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        match self {
            Terminator::Error => smallvec![],
            Terminator::Return { value } => value.iter().copied().collect(),
            Terminator::Jump { target, .. } => target.arguments(tree).iter().copied().collect(),
            Terminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => {
                let mut uses = smallvec![*condition];
                uses.extend(then_target.arguments(tree).iter().copied());
                uses.extend(else_target.arguments(tree).iter().copied());

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
                    .collect::<SmallVec<[Value; 8]>>();
                uses.extend(success.arguments(tree).iter().copied());
                uses.extend(failure.arguments(tree).iter().copied());

                uses
            }
            Terminator::Switch {
                value,
                default,
                cases,
                ..
            } => {
                let mut uses = smallvec![*value];
                uses.extend(default.arguments(tree).iter().copied());
                for case in tree.get_switch_cases(*cases) {
                    uses.extend(case.target.arguments(tree).iter().copied());
                }

                uses
            }
            Terminator::Yield {
                value,
                resume,
                unwind,
            } => {
                let mut uses = smallvec![*value];
                uses.extend(resume.arguments(tree).iter().copied());
                if let Some(unwind) = unwind {
                    uses.extend(unwind.arguments(tree).iter().copied());
                }

                uses
            }
            Terminator::Call {
                call,
                target,
                unwind,
                ..
            } => {
                let mut uses = tree
                    .get_values(call.arguments)
                    .iter()
                    .copied()
                    .collect::<SmallVec<[Value; 8]>>();
                uses.extend(target.arguments(tree).iter().copied());
                if let Some(unwind) = unwind {
                    uses.extend(unwind.arguments(tree).iter().copied());
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
                uses.extend(tree.get_values(call.arguments).iter().copied());
                uses.extend(target.arguments(tree).iter().copied());
                if let Some(unwind) = unwind {
                    uses.extend(unwind.arguments(tree).iter().copied());
                }

                uses
            }
            Terminator::CallVirtual {
                receiver,
                call,
                target,
                unwind,
                ..
            }
            | Terminator::CallDynamic {
                receiver,
                call,
                target,
                unwind,
                ..
            } => {
                let mut uses = smallvec![*receiver];
                uses.extend(tree.get_values(call.arguments).iter().copied());
                uses.extend(target.arguments(tree).iter().copied());
                if let Some(unwind) = unwind {
                    uses.extend(unwind.arguments(tree).iter().copied());
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
                uses.extend(success.arguments(tree).iter().copied());
                uses.extend(failure.arguments(tree).iter().copied());

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
                uses.extend(success.arguments(tree).iter().copied());
                uses.extend(failure.arguments(tree).iter().copied());

                uses
            }
            Terminator::Panic { payload } => payload.iter().copied().collect(),
            Terminator::UnwindResume => smallvec![],
            Terminator::Trap { payload, .. } => payload.iter().copied().collect(),
            Terminator::Unreachable => smallvec![],
            Terminator::TailCall { call, .. } => {
                tree.get_values(call.arguments).iter().copied().collect()
            }
            Terminator::TailCallIndirect { callee, call, .. } => {
                let mut uses = smallvec![*callee];
                uses.extend(tree.get_values(call.arguments).iter().copied());

                uses
            }
            Terminator::TailCallVirtual { receiver, call, .. }
            | Terminator::TailCallDynamic { receiver, call, .. } => {
                let mut uses = smallvec![*receiver];
                uses.extend(tree.get_values(call.arguments).iter().copied());

                uses
            }
        }
    }

    /// Return the arguments passed to one successor block.
    pub fn arguments_for_successor<'a>(
        &self,
        tree: &'a Tree,
        successor: LocalNodeId<Block>,
    ) -> &'a [Value] {
        match self {
            Terminator::Jump { target } if Some(target.block) == Some(successor) => {
                target.arguments(tree)
            }

            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                if Some(then_target.block) == Some(successor) {
                    then_target.arguments(tree)
                } else if Some(else_target.block) == Some(successor) {
                    else_target.arguments(tree)
                } else {
                    &[]
                }
            }
            Terminator::Check {
                success, failure, ..
            } => {
                if Some(success.block) == Some(successor) {
                    success.arguments(tree)
                } else if Some(failure.block) == Some(successor) {
                    failure.arguments(tree)
                } else {
                    &[]
                }
            }

            Terminator::Switch { default, cases, .. } => {
                if Some(default.block) == Some(successor) {
                    return default.arguments(tree);
                }

                for case in tree.get_switch_cases(*cases) {
                    if Some(case.target.block) == Some(successor) {
                        return case.target.arguments(tree);
                    }
                }

                &[]
            }

            Terminator::Yield { resume, unwind, .. } => {
                if Some(resume.block) == Some(successor) {
                    resume.arguments(tree)
                } else if let Some(unwind) = unwind
                    && Some(unwind.block) == Some(successor)
                {
                    unwind.arguments(tree)
                } else {
                    &[]
                }
            }
            Terminator::Call { target, unwind, .. }
            | Terminator::CallIndirect { target, unwind, .. }
            | Terminator::CallVirtual { target, unwind, .. }
            | Terminator::CallDynamic { target, unwind, .. } => {
                if Some(target.block) == Some(successor) {
                    target.arguments(tree)
                } else if let Some(unwind) = unwind
                    && Some(unwind.block) == Some(successor)
                {
                    unwind.arguments(tree)
                } else {
                    &[]
                }
            }

            _ => &[],
        }
    }

    /// Return the arguments passed to one successor, reporting conflicts.
    pub fn edge_arguments<'a>(
        &self,
        tree: &'a Tree,
        successor: LocalNodeId<Block>,
    ) -> EdgeArguments<'a> {
        let mut arguments = EdgeArguments::Missing;

        // compare every matching edge to the requested successor
        match self {
            Terminator::Jump { target } => {
                arguments.merge_target(target, successor, tree);
            }
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                arguments.merge_target(then_target, successor, tree);
                arguments.merge_target(else_target, successor, tree);
            }
            Terminator::Check {
                success, failure, ..
            } => {
                arguments.merge_target(success, successor, tree);
                arguments.merge_target(failure, successor, tree);
            }
            Terminator::Switch { default, cases, .. } => {
                arguments.merge_target(default, successor, tree);
                for case in tree.get_switch_cases(*cases) {
                    arguments.merge_target(&case.target, successor, tree);
                }
            }
            Terminator::Yield { resume, unwind, .. } => {
                arguments.merge_target(resume, successor, tree);
                if let Some(unwind) = unwind {
                    arguments.merge_target(unwind, successor, tree);
                }
            }
            Terminator::Call { target, unwind, .. }
            | Terminator::CallIndirect { target, unwind, .. }
            | Terminator::CallVirtual { target, unwind, .. }
            | Terminator::CallDynamic { target, unwind, .. } => {
                arguments.merge_target(target, successor, tree);
                if let Some(unwind) = unwind {
                    arguments.merge_target(unwind, successor, tree);
                }
            }
            _ => {}
        }

        arguments
    }

    /// Return successor parameters bound by explicit terminator arguments.
    pub fn argument_parameters_for_successor<'a>(
        &self,
        tree: &'a Tree,
        successor: LocalNodeId<Block>,
    ) -> &'a [BlockParameter] {
        let arguments = self.arguments_for_successor(tree, successor);
        let block = tree.get(successor);
        let parameters = block.parameters.as_slice();

        // skip the leading result parameter on call and yield resume edges
        if parameters.len() == arguments.len() + 1 && self.has_successor_result(successor) {
            &parameters[1..]
        } else {
            parameters
        }
    }

    /// Return whether one successor receives an implicit terminator result.
    pub fn has_successor_result(&self, successor: LocalNodeId<Block>) -> bool {
        match self {
            Terminator::Yield { resume, .. } => Some(resume.block) == Some(successor),
            Terminator::Call { target, .. }
            | Terminator::CallIndirect { target, .. }
            | Terminator::CallVirtual { target, .. }
            | Terminator::CallDynamic { target, .. } => Some(target.block) == Some(successor),
            _ => false,
        }
    }

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
    pub fn call_signature(&self) -> Option<TypeId> {
        match self {
            Terminator::Error => None,
            Terminator::Call { call, .. }
            | Terminator::CallIndirect { call, .. }
            | Terminator::CallVirtual { call, .. }
            | Terminator::CallDynamic { call, .. }
            | Terminator::TailCall { call, .. }
            | Terminator::TailCallIndirect { call, .. }
            | Terminator::TailCallVirtual { call, .. }
            | Terminator::TailCallDynamic { call, .. } => Some(call.signature),
            _ => None,
        }
    }

    /// Return the direct target when this terminator performs a call.
    pub fn call_direct_target(&self) -> Option<FunctionId> {
        match self {
            Terminator::Error => None,
            Terminator::Call { function, .. } | Terminator::TailCall { function, .. } => {
                Some(*function)
            }
            _ => None,
        }
    }
}
