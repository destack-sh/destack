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

    /// Pair this target with its control-flow edge.
    fn edge(&self, source: BlockId, successor: Successor) -> (Edge, &Self) {
        (Edge::new(source, successor, self.block), self)
    }

    /// Rewrite this target when it reaches `successor`.
    fn rewrite<F>(&mut self, successor: BlockId, rewrite: &mut F, tree: &mut Tree) -> bool
    where
        F: FnMut(&BlockTarget, &mut Tree) -> BlockTarget,
    {
        if self.block != successor {
            return false;
        }

        *self = rewrite(self, tree);

        true
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
        if target.block != successor {
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

    /// Rewrite every case that reaches one successor.
    fn rewrite_successor<F>(&mut self, successor: BlockId, rewrite: &mut F, tree: &mut Tree) -> bool
    where
        F: FnMut(&BlockTarget, &mut Tree) -> BlockTarget,
    {
        let mut cases = tree.get_switch_cases(*self).to_vec();
        let mut is_changed = false;

        // rewrite matching case targets
        for case in &mut cases {
            is_changed |= case.target.rewrite(successor, rewrite, tree);
        }

        // store the replacement case range
        if is_changed {
            *self = tree.add_switch_cases(&cases);
        }

        is_changed
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
        /// The type stored in the allocation.
        storage_type: TypeId,
        /// The block to jump to when allocation succeeds.
        success: BlockTarget,
        /// The block to jump to when allocation fails.
        failure: BlockTarget,
    },
    /// Fallible uninitialized typed heap allocation.
    NewUninitTry {
        /// The type stored in the allocation.
        storage_type: TypeId,
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
            Terminator::Jump { target, .. } => vec![target.edge(source, Successor::Jump)],
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => vec![
                then_target.edge(source, Successor::BranchThen),
                else_target.edge(source, Successor::BranchElse),
            ],
            Terminator::Check {
                success, failure, ..
            } => vec![
                success.edge(source, Successor::CheckSuccess),
                failure.edge(source, Successor::CheckFailure),
            ],
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
            } => vec![
                success.edge(source, Successor::NewSuccess),
                failure.edge(source, Successor::NewFailure),
            ],
            Terminator::Switch { default, cases, .. } => {
                let mut edges = Vec::with_capacity(cases.len() + 1);

                // add the default edge first
                edges.push(default.edge(source, Successor::SwitchDefault));

                // add case edges in source order
                for case in tree.get_switch_cases(*cases) {
                    edges.push(
                        case.target
                            .edge(source, Successor::SwitchCase { value: case.value }),
                    );
                }

                edges
            }
            Terminator::VariantSwitch { default, cases, .. } => {
                let mut edges = Vec::with_capacity(cases.len() + 1);

                // add the optional default edge first
                if let Some(default) = default {
                    edges.push(default.edge(source, Successor::SwitchDefault));
                }

                // add case edges in source order
                for case in tree.get_switch_cases(*cases) {
                    edges.push(
                        case.target
                            .edge(source, Successor::SwitchCase { value: case.value }),
                    );
                }

                edges
            }
            Terminator::Invoke { target, unwind, .. } => {
                vec![
                    target.edge(source, Successor::InvokeNormal),
                    unwind.edge(source, Successor::InvokeUnwind),
                ]
            }
            Terminator::Return { .. }
            | Terminator::Panic { .. }
            | Terminator::UnwindResume
            | Terminator::Abort { .. }
            | Terminator::Unreachable
            | Terminator::TailCall { .. } => Vec::new(),
        }
    }

    /// Rewrite every edge to one successor.
    pub fn rewrite_successor(
        &mut self,
        successor: BlockId,
        mut rewrite: impl FnMut(&BlockTarget, &mut Tree) -> BlockTarget,
        tree: &mut Tree,
    ) -> bool {
        match self {
            Self::Jump { target } => target.rewrite(successor, &mut rewrite, tree),
            Self::Branch {
                then_target,
                else_target,
                ..
            } => {
                let then_changed = then_target.rewrite(successor, &mut rewrite, tree);
                let else_changed = else_target.rewrite(successor, &mut rewrite, tree);

                then_changed || else_changed
            }
            Self::Check {
                success, failure, ..
            }
            | Self::NewZeroedTry {
                success, failure, ..
            }
            | Self::NewUninitTry {
                success, failure, ..
            }
            | Self::NewSliceZeroedTry {
                success, failure, ..
            }
            | Self::NewSliceUninitTry {
                success, failure, ..
            } => {
                let success_changed = success.rewrite(successor, &mut rewrite, tree);
                let failure_changed = failure.rewrite(successor, &mut rewrite, tree);

                success_changed || failure_changed
            }
            Self::Switch { default, cases, .. } => {
                let default_changed = default.rewrite(successor, &mut rewrite, tree);
                let cases_changed = cases.rewrite_successor(successor, &mut rewrite, tree);

                default_changed || cases_changed
            }
            Self::VariantSwitch { default, cases, .. } => {
                let default_changed = if let Some(default) = default {
                    default.rewrite(successor, &mut rewrite, tree)
                } else {
                    false
                };
                let cases_changed = cases.rewrite_successor(successor, &mut rewrite, tree);

                default_changed || cases_changed
            }
            Self::Invoke { target, unwind, .. } => {
                let target_changed = target.rewrite(successor, &mut rewrite, tree);
                let unwind_changed = unwind.rewrite(successor, &mut rewrite, tree);

                target_changed || unwind_changed
            }
            Self::Error
            | Self::Return { .. }
            | Self::Panic { .. }
            | Self::UnwindResume
            | Self::Abort { .. }
            | Self::Unreachable
            | Self::TailCall { .. } => false,
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
            Terminator::Return { value: Some(value) } => smallvec![*value],
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
            Terminator::Jump { target } if target.block == successor => target.arguments(tree),

            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                if then_target.block == successor {
                    then_target.arguments(tree)
                } else if else_target.block == successor {
                    else_target.arguments(tree)
                } else {
                    &[]
                }
            }
            Terminator::Check {
                success, failure, ..
            } => {
                if success.block == successor {
                    success.arguments(tree)
                } else if failure.block == successor {
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
                if success.block == successor {
                    success.arguments(tree)
                } else if failure.block == successor {
                    failure.arguments(tree)
                } else {
                    &[]
                }
            }

            Terminator::Switch { default, cases, .. } => {
                if default.block == successor {
                    return default.arguments(tree);
                }

                for case in tree.get_switch_cases(*cases) {
                    if case.target.block == successor {
                        return case.target.arguments(tree);
                    }
                }

                &[]
            }
            Terminator::VariantSwitch { default, cases, .. } => {
                if let Some(default) = default
                    && default.block == successor
                {
                    return default.arguments(tree);
                }

                for case in tree.get_switch_cases(*cases) {
                    if case.target.block == successor {
                        return case.target.arguments(tree);
                    }
                }

                &[]
            }

            Terminator::Invoke { target, unwind, .. } => {
                if target.block == successor {
                    target.arguments(tree)
                } else if unwind.block == successor {
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

        // skip values produced directly by the terminator
        let result_count = self.successor_result_count(successor);
        if parameters.len() == arguments.len() + result_count {
            &parameters[result_count..]
        } else {
            parameters
        }
    }

    /// Return the number of values produced directly for one successor.
    pub fn successor_result_count(&self, successor: LocalNodeId<Block>) -> usize {
        match self {
            Terminator::Invoke { target, .. } => usize::from(target.block == successor),
            Terminator::NewZeroedTry { success, .. }
            | Terminator::NewUninitTry { success, .. }
            | Terminator::NewSliceZeroedTry { success, .. }
            | Terminator::NewSliceUninitTry { success, .. } => {
                usize::from(success.block == successor)
            }
            _ => 0,
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

#[cfg(test)]
mod tests {
    use crate::parse::{ParseOptions, Parser, test_file};
    use crate::{BlockId, Function, Terminator, Tree, Type};

    /// Parse one MIR tree for terminator owner-method tests.
    fn parse_tree(source: &str) -> Tree {
        let file = test_file(source);

        Parser::parse(&file, ParseOptions::default())
            .expect("MIR parser requires text content")
            .finish()
            .expect("parse failed")
            .0
    }

    /// Return one block from the first function.
    fn block(tree: &Tree, index: usize) -> BlockId {
        let (_, function) = tree
            .iter_nodes::<Function>()
            .next()
            .expect("missing function");

        function.block(index)
    }

    /// Return one block's terminator.
    fn terminator(tree: &Tree, index: usize) -> &Terminator {
        let block = tree.get(block(tree, index));

        tree.get(block.terminator)
    }

    /// Return a block parameter's type.
    fn block_parameter_type(tree: &Tree, block: BlockId, index: usize) -> &Type {
        let block = tree.get(block);
        let parameter = block.parameters.get(index).expect("missing parameter");

        tree.get(parameter.ty)
    }

    /// Parse fallible allocation edges with explicit success and failure payloads.
    fn parse_fallible_allocation_tree() -> Tree {
        parse_tree(
            r#"
function test(v0: int64, v9: int32): int32 {
entry(v0: int64, v9: int32):
    new.slice.uninit.try int32, v0 => b1(v9) | b2(v9)

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
        let tree = parse_fallible_allocation_tree();
        let terminator = terminator(&tree, 0);
        let success = block(&tree, 1);
        let failure = block(&tree, 2);
        let success_arguments = terminator.successor_arguments(&tree, success);
        let failure_arguments = terminator.successor_arguments(&tree, failure);

        // success and failure edges carry the explicit payload
        assert_eq!(success_arguments.len(), 1);
        assert_eq!(success_arguments, failure_arguments);
    }

    /// Fallible allocation success parameters skip the implicit result.
    #[test]
    fn test_successor_parameters_skip_fallible_allocation_result() {
        let tree = parse_fallible_allocation_tree();
        let terminator = terminator(&tree, 0);
        let success = block(&tree, 1);
        let failure = block(&tree, 2);

        // success receives an implicit allocation result before explicit payloads
        assert!(matches!(
            block_parameter_type(&tree, success, 0),
            Type::Uninit { .. }
        ));
        assert_eq!(terminator.successor_parameters(&tree, success).len(), 1);

        // failure has no implicit result and binds every explicit payload
        assert_eq!(terminator.successor_parameters(&tree, failure).len(), 1);
    }

    /// Fallible allocation results only apply to success edges.
    #[test]
    fn test_successor_result_count_marks_only_fallible_allocation_success() {
        let tree = parse_fallible_allocation_tree();
        let terminator = terminator(&tree, 0);
        let success = block(&tree, 1);
        let failure = block(&tree, 2);

        assert_eq!(terminator.successor_result_count(success), 1);
        assert_eq!(terminator.successor_result_count(failure), 0);
    }
}
