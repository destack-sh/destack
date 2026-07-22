use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    BinaryOperator, Block, BlockId, BlockParameter, Call, CallDispatch, Edge, FunctionId,
    LocalNodeId, Node, NodeType, Successor, Tree, TypeId, Value, ValueSlice,
};

/// One control-flow edge target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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

/// Condition tested by one runtime check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Exact runtime type check for a value.
    IsType {
        /// The value being checked.
        value: Value,
        /// The expected concrete runtime type.
        expected: TypeId,
    },
    /// Runtime subtype relation check for a value.
    IsSubtype {
        /// The value being checked.
        value: Value,
        /// The expected supertype.
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
            CheckConstraint::IsType { value, .. } => smallvec![*value],
            CheckConstraint::IsSubtype { value, .. } => smallvec![*value],
        }
    }
}

/// One case arm for a switch terminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SwitchCase {
    /// The matched case value.
    pub value: i128,
    /// The target block for this case.
    pub target: BlockTarget,
}

/// Compact reference to a switch case list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
        /// The condition tested by the check.
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
    /// Switch on the logical case of a variant value.
    VariantSwitch {
        /// The variant value to switch on.
        value: Value,
        /// The block to jump to if no case matches, absent when exhaustive.
        default: Option<BlockTarget>,
        /// The cases to match against, keyed by logical case index.
        cases: SwitchCaseSlice,
    },

    /// Yield from a coroutine to its current owner.
    Yield {
        /// The yielded value.
        value: Value,
        /// The block entered when the coroutine receives a resume command.
        resume: BlockTarget,
        /// The cleanup block when the yield is left by panic unwinding.
        unwind: Option<BlockTarget>,
    },

    /// Invoke one callable target with normal and unwind continuations.
    Invoke {
        /// The call operation.
        call: Call,
        /// The normal continuation block.
        target: BlockTarget,
        /// The local unwind continuation block.
        unwind: BlockTarget,
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
    UnwindResume,
    /// Abort execution immediately.
    Abort {
        /// Optional diagnostic payload.
        payload: Option<Value>,
    },
    /// Unreachable code.
    Unreachable,

    /// Tail call one callable target.
    TailCall {
        /// The call operation.
        call: Call,
    },
}

impl Node for Terminator {
    const TYPE: NodeType = NodeType::Terminator;
}

impl Terminator {
    /// Enumerate the control flow edges leaving this terminator.
    pub fn edges(&self, tree: &Tree, source: BlockId) -> Vec<(Edge, BlockId)> {
        self.targets(tree, source)
            .into_iter()
            .map(|(edge, _)| (edge, edge.target))
            .collect()
    }

    /// Enumerate the control flow edges leaving this terminator with their targets.
    pub fn targets<'a>(&'a self, tree: &'a Tree, source: BlockId) -> Vec<(Edge, &'a BlockTarget)> {
        match self {
            Terminator::Error => Vec::new(),
            Terminator::Jump { target, .. } => block_edge(source, Successor::Jump, target)
                .into_iter()
                .collect(),
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => [
                block_edge(source, Successor::BranchThen, then_target),
                block_edge(source, Successor::BranchElse, else_target),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Terminator::Check {
                success, failure, ..
            } => [
                block_edge(source, Successor::CheckSuccess, success),
                block_edge(source, Successor::CheckFailure, failure),
            ]
            .into_iter()
            .flatten()
            .collect(),
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
            } => [
                block_edge(source, Successor::TrySuccess, success),
                block_edge(source, Successor::TryFailure, failure),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Terminator::Switch { default, cases, .. } => {
                let mut edges = Vec::with_capacity(cases.len() + 1);

                // add the default edge first
                edges.extend(block_edge(source, Successor::SwitchDefault, default));

                // add case edges in source order
                for case in tree.get_switch_cases(*cases) {
                    edges.extend(block_edge(
                        source,
                        Successor::SwitchCase { value: case.value },
                        &case.target,
                    ));
                }

                edges
            }
            Terminator::VariantSwitch { default, cases, .. } => {
                let mut edges = Vec::with_capacity(cases.len() + 1);

                // add the optional default edge first
                if let Some(default) = default {
                    edges.extend(block_edge(source, Successor::SwitchDefault, default));
                }

                // add case edges in source order
                for case in tree.get_switch_cases(*cases) {
                    edges.extend(block_edge(
                        source,
                        Successor::SwitchCase { value: case.value },
                        &case.target,
                    ));
                }

                edges
            }
            Terminator::Yield { resume, unwind, .. } => {
                let mut edges = Vec::with_capacity(2);

                // add the normal resume edge
                edges.extend(block_edge(source, Successor::YieldResume, resume));

                // add the optional unwind edge
                if let Some(unwind) = unwind {
                    edges.extend(block_edge(source, Successor::YieldUnwind, unwind));
                }

                edges
            }
            Terminator::Invoke { target, unwind, .. } => {
                let mut edges = Vec::with_capacity(2);

                // add the normal return edge
                edges.extend(block_edge(source, Successor::InvokeNormal, target));

                // add the local unwind edge
                edges.extend(block_edge(source, Successor::InvokeUnwind, unwind));

                edges
            }
            Terminator::Return { .. }
            | Terminator::Panic { .. }
            | Terminator::UnwindResume
            | Terminator::Abort { .. }
            | Terminator::Unreachable
            | Terminator::TailCall { .. } => Vec::new(),
        }
    }

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
            Terminator::VariantSwitch { default, cases, .. } => {
                let mut successors = SmallVec::new();
                if let Some(default) = default {
                    successors.push(default.block);
                }
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
            Terminator::Invoke { target, unwind, .. } => {
                smallvec![target.block, unwind.block]
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
            Terminator::Abort { .. } => smallvec![],
            Terminator::Unreachable => smallvec![],
            Terminator::TailCall { .. } => smallvec![],
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
            Terminator::VariantSwitch {
                value,
                default,
                cases,
                ..
            } => {
                let mut uses = smallvec![*value];
                if let Some(default) = default {
                    uses.extend(default.arguments(tree).iter().copied());
                }
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
            Terminator::Invoke {
                call,
                target,
                unwind,
            } => {
                let mut uses = call.uses(tree);
                uses.extend(target.arguments(tree).iter().copied());
                uses.extend(unwind.arguments(tree).iter().copied());

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
            Terminator::Abort { payload } => payload.iter().copied().collect(),
            Terminator::Unreachable => smallvec![],
            Terminator::TailCall { call } => call.uses(tree),
        }
    }

    /// Return values consumed by this terminator.
    pub fn consumes(&self, tree: &Tree) -> SmallVec<[Value; 8]> {
        match self {
            Terminator::Return { value: Some(value) } | Terminator::Yield { value, .. } => {
                smallvec![*value]
            }
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => call.uses(tree),
            _ => smallvec![],
        }
    }

    /// Return the arguments passed to one successor block.
    pub fn successor_arguments<'a>(
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
            Terminator::VariantSwitch { default, cases, .. } => {
                if let Some(default) = default
                    && Some(default.block) == Some(successor)
                {
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
            Terminator::Invoke { target, unwind, .. } => {
                if Some(target.block) == Some(successor) {
                    target.arguments(tree)
                } else if Some(unwind.block) == Some(successor) {
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
            Terminator::VariantSwitch { default, cases, .. } => {
                if let Some(default) = default {
                    arguments.merge_target(default, successor, tree);
                }
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
            Terminator::Invoke { target, unwind, .. } => {
                arguments.merge_target(target, successor, tree);
                arguments.merge_target(unwind, successor, tree);
            }
            _ => {}
        }

        arguments
    }

    /// Return successor parameters bound by explicit terminator arguments.
    pub fn successor_parameters<'a>(
        &self,
        tree: &'a Tree,
        successor: LocalNodeId<Block>,
    ) -> &'a [BlockParameter] {
        let arguments = self.successor_arguments(tree, successor);
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
            Terminator::Invoke { target, .. } => Some(target.block) == Some(successor),
            Terminator::NewZeroedTry { success, .. }
            | Terminator::NewUninitTry { success, .. }
            | Terminator::NewSliceZeroedTry { success, .. }
            | Terminator::NewSliceUninitTry { success, .. } => {
                Some(success.block) == Some(successor)
            }
            _ => false,
        }
    }

    /// Return the dispatch when this terminator performs a call.
    pub fn call_dispatch(&self) -> Option<CallDispatch> {
        match self {
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => {
                Some(call.callee.dispatch())
            }
            _ => None,
        }
    }

    /// Return the call signature when this terminator performs a call.
    pub fn call_signature(&self) -> Option<TypeId> {
        match self {
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => Some(call.signature),
            _ => None,
        }
    }

    /// Return the direct target when this terminator performs a call.
    pub fn call_direct_target(&self) -> Option<FunctionId> {
        match self {
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => {
                call.callee.function()
            }
            _ => None,
        }
    }
}

/// Pair one block target with its edge when it resolves to a concrete block.
fn block_edge(
    source: BlockId,
    successor: Successor,
    target: &BlockTarget,
) -> Option<(Edge, &BlockTarget)> {
    Some((Edge::new(source, successor, target.block), target))
}

#[cfg(test)]
mod tests {
    use crate::parse::{ParseOptions, Parser, test_file};
    use crate::{Block, Terminator, Tree, Type};
    use destack_core::StringPool;

    /// Parse one MIR tree for terminator owner-method tests.
    fn parse_tree(source: &str) -> (Tree, StringPool) {
        let file = test_file(source);

        Parser::parse(&file, ParseOptions::default())
            .expect("MIR parser requires text content")
            .finish()
            .expect("parse failed")
    }

    /// Return one named block from a tree.
    fn block_by_name(tree: &Tree, strings: &StringPool, name: &str) -> crate::BlockId {
        tree.iter_nodes::<Block>()
            .find(|(_, block)| {
                block
                    .name
                    .map(|name_id| strings.get(name_id) == name)
                    .unwrap_or(false)
            })
            .expect("missing block")
            .0
    }

    /// Return one named block's terminator.
    fn terminator_by_block_name<'a>(
        tree: &'a Tree,
        strings: &StringPool,
        name: &str,
    ) -> &'a Terminator {
        let block = tree.get(block_by_name(tree, strings, name));

        tree.get(block.terminator)
    }

    /// Return a block parameter's type.
    fn block_parameter_type(tree: &Tree, block: crate::BlockId, index: usize) -> &Type {
        let block = tree.get(block);
        let parameter = block.parameters.get(index).expect("missing parameter");

        tree.get(parameter.ty)
    }

    /// Parse fallible allocation edges with explicit success and failure payloads.
    fn parse_fallible_allocation_tree() -> (Tree, StringPool) {
        parse_tree(
            r#"
function test(v0: int64, v9: int32): int32 {
entry(v0: int64, v9: int32):
    new.slice.uninit.try int32, v0 => b1(v9), b2(v9)

b1(v1: uninit<slice<int32, managed, mutable>>, v2: int32):
    return v2

b2(v3: int32):
    return v3
}
"#,
        )
    }

    /// Fallible allocation terminators expose explicit edge arguments.
    #[test]
    fn test_successor_arguments_include_fallible_allocation_edges() {
        let (tree, strings) = parse_fallible_allocation_tree();
        let terminator = terminator_by_block_name(&tree, &strings, "entry");
        let success = block_by_name(&tree, &strings, "b1");
        let failure = block_by_name(&tree, &strings, "b2");
        let success_arguments = terminator.successor_arguments(&tree, success);
        let failure_arguments = terminator.successor_arguments(&tree, failure);

        // success and failure edges carry the explicit payload
        assert_eq!(success_arguments.len(), 1);
        assert_eq!(success_arguments, failure_arguments);
    }

    /// Fallible allocation success parameters skip the implicit result.
    #[test]
    fn test_successor_parameters_skip_fallible_allocation_result() {
        let (tree, strings) = parse_fallible_allocation_tree();
        let terminator = terminator_by_block_name(&tree, &strings, "entry");
        let success = block_by_name(&tree, &strings, "b1");
        let failure = block_by_name(&tree, &strings, "b2");

        // success receives an implicit allocation result before explicit payloads
        assert!(matches!(
            block_parameter_type(&tree, success, 0),
            Type::Uninit { .. }
        ));
        assert_eq!(terminator.successor_parameters(&tree, success).len(), 1);

        // failure has no implicit result and binds every explicit payload
        assert_eq!(terminator.successor_parameters(&tree, failure).len(), 1);
    }

    /// Fallible allocation result markers only apply to success edges.
    #[test]
    fn test_has_successor_result_marks_only_fallible_allocation_success() {
        let (tree, strings) = parse_fallible_allocation_tree();
        let terminator = terminator_by_block_name(&tree, &strings, "entry");
        let success = block_by_name(&tree, &strings, "b1");
        let failure = block_by_name(&tree, &strings, "b2");

        assert!(terminator.has_successor_result(success));
        assert!(!terminator.has_successor_result(failure));
    }
}
