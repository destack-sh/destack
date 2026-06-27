use std::collections::HashMap;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{DevirtualizeGuardedOptions, MirOptimized, ModulePass, PipelineContext};
use mir::{Mutation, ProfilePoint, ValueProfile};

declare_pass! {
    /// Rewrite hot virtual and dynamic calls to guarded direct calls.
    #[pass(id = "devirtualize-guarded")]
    pub DevirtualizeGuarded,
    "Devirtualize hot calls with receiver guards"
}

impl ModulePass for DevirtualizeGuarded {
    /// Run DevirtualizeGuarded.
    fn run(
        &self,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mir::TreeAnalysisCache,
    ) -> Mutation {
        let Some(profile) = ctx.profile() else {
            return Mutation::NONE;
        };

        let changed = DevirtualizeGuardedState::new(
            &optimized.profile,
            analyses.dispatch(),
            profile,
            ctx.options.devirtualize_guarded,
        )
        .apply(&mut optimized.tree);

        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "DevirtualizeGuarded"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "devirtualize-guarded"
    }
}

/// State for one DevirtualizeGuarded run.
struct DevirtualizeGuardedState<'a> {
    /// Static profile point table.
    profile_table: &'a mir::ProfileTable,
    /// Static dispatch table.
    dispatch: &'a mir::DispatchTable,
    /// Collected runtime profile.
    profile: &'a mir::Profile,
    /// Promotion thresholds.
    options: DevirtualizeGuardedOptions,
}

impl<'a> DevirtualizeGuardedState<'a> {
    /// Create DevirtualizeGuarded state.
    fn new(
        profile_table: &'a mir::ProfileTable,
        dispatch: &'a mir::DispatchTable,
        profile: &'a mir::Profile,
        options: DevirtualizeGuardedOptions,
    ) -> Self {
        Self {
            profile_table,
            dispatch,
            profile,
            options,
        }
    }

    /// Apply DevirtualizeGuarded across one module.
    fn apply(&self, tree: &mut mir::Tree) -> bool {
        let mut changed = false;
        let functions_by_symbol = Self::functions_by_symbol(tree);
        let function_ids = tree
            .iter_nodes::<mir::Function>()
            .map(|(function, _)| function)
            .collect::<Vec<_>>();

        // promote one site at a time because each rewrite changes block layout
        for function_id in function_ids {
            while self.promote_next_function_call(tree, function_id, &functions_by_symbol) {
                changed = true;
            }
        }

        changed
    }

    /// Return functions keyed by stable symbol.
    fn functions_by_symbol(tree: &mir::Tree) -> HashMap<mir::Symbol, mir::FunctionId> {
        tree.iter_nodes::<mir::Function>()
            .map(|(function_id, function)| (function.symbol, function_id))
            .collect()
    }

    /// Promote the first eligible call in one function.
    fn promote_next_function_call(
        &self,
        tree: &mut mir::Tree,
        function_id: mir::FunctionId,
        functions_by_symbol: &HashMap<mir::Symbol, mir::FunctionId>,
    ) -> bool {
        let function = tree.get(function_id);
        if !function.is_defined() {
            return false;
        }

        let Some(function_profile) = self.profile.function(function.symbol) else {
            return false;
        };

        let Some(profile_map) = self.profile_table.function(function_id) else {
            return false;
        };

        if profile_map.hash != function_profile.hash {
            return false;
        }

        let Some(promotion) = self.find_promotion(
            tree,
            function,
            function_profile,
            profile_map,
            functions_by_symbol,
        ) else {
            return false;
        };

        let mut function = tree.get(function_id).clone();
        promotion.apply(&mut function, tree);
        tree.set(function_id, function);

        true
    }

    /// Return the first eligible call promotion in one function.
    fn find_promotion(
        &self,
        tree: &mir::Tree,
        function: &mir::Function,
        function_profile: &mir::FunctionProfile,
        profile_map: &mir::FunctionProfileTable,
        functions_by_symbol: &HashMap<mir::Symbol, mir::FunctionId>,
    ) -> Option<Promotion> {
        for &block in function.blocks() {
            let node = tree.get(block);

            // prefer instruction calls before the block terminator
            for &instruction in &node.instructions {
                let call = CallNode::Instruction(instruction);
                if let Some(promotion) = self.promotion_for_call(
                    tree,
                    call,
                    function_profile,
                    profile_map,
                    functions_by_symbol,
                ) {
                    return Some(promotion);
                }
            }

            // inspect the terminator after instruction calls
            let call = CallNode::Terminator(block);
            if let Some(promotion) = self.promotion_for_call(
                tree,
                call,
                function_profile,
                profile_map,
                functions_by_symbol,
            ) {
                return Some(promotion);
            }
        }

        None
    }

    /// Return the promotion for one profiled call when the profile is strong enough.
    fn promotion_for_call(
        &self,
        tree: &mir::Tree,
        node: CallNode,
        function_profile: &mir::FunctionProfile,
        profile_map: &mir::FunctionProfileTable,
        functions_by_symbol: &HashMap<mir::Symbol, mir::FunctionId>,
    ) -> Option<Promotion> {
        let callsite = node.callsite();
        let function = self.profiled_call_target(
            function_profile,
            profile_map,
            callsite,
            functions_by_symbol,
        )?;
        let expected = self.profiled_receiver_type(function_profile, profile_map, callsite)?;
        let call = DispatchCall::from_node(tree, node)?;
        if call.target(self.dispatch, expected) != Some(function) {
            return None;
        }

        Some(Promotion {
            node,
            function,
            receiver: call.receiver(),
            expected,
        })
    }

    /// Return the hot profiled call target for one callsite.
    fn profiled_call_target(
        &self,
        function_profile: &mir::FunctionProfile,
        profile_map: &mir::FunctionProfileTable,
        callsite: mir::CallSite,
        functions_by_symbol: &HashMap<mir::Symbol, mir::FunctionId>,
    ) -> Option<mir::FunctionId> {
        let counter = profile_map.counter(&ProfilePoint::CallTarget(callsite))?;
        let value_profile = function_profile.values.get(&counter)?;
        let ValueProfile::Calls(histogram) = value_profile else {
            return None;
        };
        let (symbol, count) = histogram.dominant()?;
        let total = histogram.total().get();

        if !self
            .options
            .accepts(count.get(), histogram.unknown.get(), total)
        {
            return None;
        }

        functions_by_symbol.get(symbol).copied()
    }

    /// Return the hot profiled receiver type for one callsite.
    fn profiled_receiver_type(
        &self,
        function_profile: &mir::FunctionProfile,
        profile_map: &mir::FunctionProfileTable,
        callsite: mir::CallSite,
    ) -> Option<mir::TypeId> {
        let counter = profile_map.counter(&ProfilePoint::ReceiverType(callsite))?;
        let value_profile = function_profile.values.get(&counter)?;
        let ValueProfile::Types(histogram) = value_profile else {
            return None;
        };
        let (ty, count) = histogram.dominant()?;
        let total = histogram.total().get();

        if !self
            .options
            .accepts(count.get(), histogram.unknown.get(), total)
        {
            return None;
        }

        Some(*ty)
    }
}

/// One profiled call promotion.
#[derive(Debug, Clone)]
struct Promotion {
    /// The call node being promoted.
    node: CallNode,
    /// The hot direct target.
    function: mir::FunctionId,
    /// The receiver value guarded by the promotion.
    receiver: mir::Value,
    /// The expected concrete receiver type.
    expected: mir::TypeId,
}

impl Promotion {
    /// Apply this promotion to a function.
    fn apply(self, function: &mut mir::Function, tree: &mut mir::Tree) {
        match self.node {
            CallNode::Instruction(instruction) => {
                self.apply_instruction(function, tree, instruction);
            }
            CallNode::Terminator(block) => {
                self.apply_terminator(function, tree, block);
            }
        }
    }

    /// Apply a promotion to an instruction call by splitting the containing block.
    fn apply_instruction(
        self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) {
        let block = function
            .instruction_block(instruction)
            .unwrap_or_else(|| unreachable!("promoted instruction is not in the function"));
        let position = function
            .instruction_position(instruction)
            .unwrap_or_else(|| unreachable!("promoted instruction has no block position"));
        let original = tree.get(instruction).clone();
        let destination = original.destination();
        let block_node = tree.get(block).clone();
        let prefix = block_node.instructions[..position].to_vec();
        let suffix = block_node.instructions[position + 1..].to_vec();
        let old_terminator = tree.insert(tree.get(block_node.terminator).clone());

        // build continuation block around the original suffix and terminator
        let mut continuation = mir::Block::new(old_terminator);
        continuation.instructions = suffix;
        if let Some(destination) = destination {
            let ty = function
                .value_type(destination)
                .unwrap_or_else(|| unreachable!("missing promoted call result type"));
            continuation.parameters.push(mir::BlockParameter {
                value: destination,
                ty,
            });
        }
        let continuation = tree.insert(continuation);
        let (direct, fallback) = self
            .instruction_terminators(&original, continuation)
            .unwrap_or_else(|| unreachable!("promoted instruction is not a dispatch call"));

        // build hot and fallback call blocks
        let hot = Self::insert_block_with_terminator(tree, direct);
        let fallback = Self::insert_block_with_terminator(tree, fallback);

        // replace the original block with the dispatch guard
        function.replace_block_instructions(block, prefix, tree);
        let check = self.check_terminator(hot, fallback);
        tree.set(tree.get(block).terminator, check);

        // keep the promoted blocks next to the original control flow
        function.insert_block_after(block, hot, tree);
        function.insert_block_after(hot, fallback, tree);
        function.insert_block_after(fallback, continuation, tree);
    }

    /// Apply a promotion to a terminator call by inserting guarded call blocks.
    fn apply_terminator(
        self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        block: mir::BlockId,
    ) {
        let terminator = tree.get(tree.get(block).terminator).clone();
        let (direct, fallback) = self
            .terminator_replacements(&terminator)
            .unwrap_or_else(|| unreachable!("promoted terminator is not a dispatch call"));

        // build hot and fallback terminator blocks
        let hot = Self::insert_block_with_terminator(tree, direct);
        let fallback = Self::insert_block_with_terminator(tree, fallback);

        // replace the original terminator with the guard
        let check = self.check_terminator(hot, fallback);
        tree.set(tree.get(block).terminator, check);

        // place the cloned calls after the guard block
        function.insert_block_after(block, hot, tree);
        function.insert_block_after(hot, fallback, tree);
    }

    /// Return the type guard terminator for this promotion.
    fn check_terminator(&self, hot: mir::BlockId, fallback: mir::BlockId) -> mir::Terminator {
        mir::Terminator::Check {
            constraint: mir::CheckConstraint::IsType {
                value: self.receiver,
                expected: self.expected,
            },
            success: mir::BlockTarget::new(hot, mir::ValueSlice::default()),
            failure: mir::BlockTarget::new(fallback, mir::ValueSlice::default()),
        }
    }

    /// Insert a block containing one terminator.
    fn insert_block_with_terminator(
        tree: &mut mir::Tree,
        terminator: mir::Terminator,
    ) -> mir::BlockId {
        let terminator = tree.insert(terminator);

        tree.insert(mir::Block::new(terminator))
    }

    /// Return terminator calls for an instruction call promotion.
    fn instruction_terminators(
        &self,
        instruction: &mir::Instruction,
        continuation: mir::BlockId,
    ) -> Option<(mir::Terminator, mir::Terminator)> {
        let target = mir::BlockTarget::new(continuation, mir::ValueSlice::default());

        match instruction {
            mir::Instruction::CallVirtual {
                receiver,
                class,
                slot,
                call,
                ..
            } => {
                let direct = mir::Terminator::Call {
                    function: self.function,
                    call: call.clone(),
                    target: target.clone(),
                    unwind: None,
                };
                let fallback = mir::Terminator::CallVirtual {
                    receiver: *receiver,
                    class: *class,
                    slot: *slot,
                    call: call.clone(),
                    target,
                    unwind: None,
                };

                Some((direct, fallback))
            }
            mir::Instruction::CallDynamic {
                receiver,
                constraint,
                slot,
                call,
                ..
            } => {
                let direct = mir::Terminator::Call {
                    function: self.function,
                    call: call.clone(),
                    target: target.clone(),
                    unwind: None,
                };
                let fallback = mir::Terminator::CallDynamic {
                    receiver: *receiver,
                    constraint: *constraint,
                    slot: *slot,
                    call: call.clone(),
                    target,
                    unwind: None,
                };

                Some((direct, fallback))
            }
            _ => None,
        }
    }

    /// Return terminator replacements for a terminator call promotion.
    fn terminator_replacements(
        &self,
        terminator: &mir::Terminator,
    ) -> Option<(mir::Terminator, mir::Terminator)> {
        match terminator {
            mir::Terminator::CallVirtual {
                receiver,
                class,
                slot,
                call,
                target,
                unwind,
            } => {
                let direct = mir::Terminator::Call {
                    function: self.function,
                    call: call.clone(),
                    target: target.clone(),
                    unwind: unwind.clone(),
                };
                let fallback = mir::Terminator::CallVirtual {
                    receiver: *receiver,
                    class: *class,
                    slot: *slot,
                    call: call.clone(),
                    target: target.clone(),
                    unwind: unwind.clone(),
                };

                Some((direct, fallback))
            }
            mir::Terminator::CallDynamic {
                receiver,
                constraint,
                slot,
                call,
                target,
                unwind,
            } => {
                let direct = mir::Terminator::Call {
                    function: self.function,
                    call: call.clone(),
                    target: target.clone(),
                    unwind: unwind.clone(),
                };
                let fallback = mir::Terminator::CallDynamic {
                    receiver: *receiver,
                    constraint: *constraint,
                    slot: *slot,
                    call: call.clone(),
                    target: target.clone(),
                    unwind: unwind.clone(),
                };

                Some((direct, fallback))
            }
            mir::Terminator::TailCallVirtual {
                receiver,
                class,
                slot,
                call,
            } => {
                let direct = mir::Terminator::TailCall {
                    function: self.function,
                    call: call.clone(),
                };
                let fallback = mir::Terminator::TailCallVirtual {
                    receiver: *receiver,
                    class: *class,
                    slot: *slot,
                    call: call.clone(),
                };

                Some((direct, fallback))
            }
            mir::Terminator::TailCallDynamic {
                receiver,
                constraint,
                slot,
                call,
            } => {
                let direct = mir::Terminator::TailCall {
                    function: self.function,
                    call: call.clone(),
                };
                let fallback = mir::Terminator::TailCallDynamic {
                    receiver: *receiver,
                    constraint: *constraint,
                    slot: *slot,
                    call: call.clone(),
                };

                Some((direct, fallback))
            }
            _ => None,
        }
    }
}

/// MIR location for one promotable call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallNode {
    /// Call stored as an instruction.
    Instruction(mir::LocalNodeId<mir::Instruction>),
    /// Call stored as a block terminator.
    Terminator(mir::BlockId),
}

impl CallNode {
    /// Return this node as a stable profile point.
    fn callsite(self) -> mir::CallSite {
        match self {
            Self::Instruction(instruction) => mir::CallSite::Instruction(instruction),
            Self::Terminator(block) => mir::CallSite::Terminator(block),
        }
    }
}

/// Static dispatch data for a virtual or dynamic call.
#[derive(Debug, Clone, Copy)]
enum DispatchCall {
    /// Virtual call dispatch data.
    Virtual {
        /// The receiver value.
        receiver: mir::Value,
        /// The class type declaring the dispatch slot.
        ty: mir::TypeId,
        /// The dispatch slot.
        slot: mir::DispatchSlot,
    },
    /// Dynamic call dispatch data.
    Dynamic {
        /// The receiver value.
        receiver: mir::Value,
        /// The dynamic constraint type declaring the dispatch slot.
        ty: mir::TypeId,
        /// The dispatch slot.
        slot: mir::DispatchSlot,
    },
}

impl DispatchCall {
    /// Return static dispatch data for one profiled call node.
    fn from_node(tree: &mir::Tree, node: CallNode) -> Option<Self> {
        match node {
            CallNode::Instruction(instruction) => Self::from_instruction(tree.get(instruction)),
            CallNode::Terminator(block) => {
                Self::from_terminator(tree.get(tree.get(block).terminator))
            }
        }
    }

    /// Return static dispatch data for one instruction call.
    fn from_instruction(instruction: &mir::Instruction) -> Option<Self> {
        match instruction {
            mir::Instruction::CallVirtual {
                receiver,
                class,
                slot,
                ..
            } => Some(Self::Virtual {
                receiver: *receiver,
                ty: *class,
                slot: *slot,
            }),
            mir::Instruction::CallDynamic {
                receiver,
                constraint,
                slot,
                ..
            } => Some(Self::Dynamic {
                receiver: *receiver,
                ty: *constraint,
                slot: *slot,
            }),
            _ => None,
        }
    }

    /// Return static dispatch data for one terminator call.
    fn from_terminator(terminator: &mir::Terminator) -> Option<Self> {
        match terminator {
            mir::Terminator::CallVirtual {
                receiver,
                class,
                slot,
                ..
            }
            | mir::Terminator::TailCallVirtual {
                receiver,
                class,
                slot,
                ..
            } => Some(Self::Virtual {
                receiver: *receiver,
                ty: *class,
                slot: *slot,
            }),
            mir::Terminator::CallDynamic {
                receiver,
                constraint,
                slot,
                ..
            }
            | mir::Terminator::TailCallDynamic {
                receiver,
                constraint,
                slot,
                ..
            } => Some(Self::Dynamic {
                receiver: *receiver,
                ty: *constraint,
                slot: *slot,
            }),
            _ => None,
        }
    }

    /// Return the receiver value.
    fn receiver(self) -> mir::Value {
        match self {
            Self::Virtual { receiver, .. } | Self::Dynamic { receiver, .. } => receiver,
        }
    }

    /// Return the dispatch target for one concrete receiver type.
    fn target(
        self,
        dispatch: &mir::DispatchTable,
        receiver_type: mir::TypeId,
    ) -> Option<mir::FunctionId> {
        match self {
            Self::Virtual { ty, slot, .. } if ty == receiver_type => dispatch
                .virtual_table(receiver_type)?
                .methods
                .get(slot.index())
                .copied(),
            Self::Virtual { .. } => None,
            Self::Dynamic { ty, slot, .. } => {
                let entry = dispatch
                    .dynamic_table(receiver_type, ty)?
                    .entries
                    .get(slot.index())?;

                match entry {
                    mir::DynamicEntry::Function { function } => Some(*function),
                    mir::DynamicEntry::FieldOffset { .. } => None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::optimize::common::tests::TestProgram;
    use destack_mir as mir;

    use super::DevirtualizeGuarded;

    /// Hot virtual instruction calls split into a receiver guard and two call edges.
    #[test]
    fn test_devirtualize_guarded_rewrites_virtual_instruction_call() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    v2: int32 = int.add v1, v0
    return v2
}
"#;
        let mut test = TestProgram::new(input);
        let caller = test.function_id_by_name("test");
        let callee = test.function_id_by_name("callee");
        let class = int32_type(&test);
        let callsite = first_virtual_instruction_call(&test, caller);
        test.add_virtual_method_table(class, callee);
        let profile = test.profile_dispatch_call(caller, callsite, callee, class, 900, 0);

        test.run_module_pass_with_profile(&DevirtualizeGuarded, profile);

        test.assert_output(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    check is.type v0, int32 => b1, b2

b1:
    call callee(v0) => b3

b2:
    call.virtual v0, int32, 0(v0): (int32) => int32 => b3

b3(v1: int32):
    v2: int32 = int.add v1, v0
    return v2
}
"#,
        );
    }

    /// Hot dynamic terminator calls split into a receiver guard and two call edges.
    #[test]
    fn test_devirtualize_guarded_rewrites_dynamic_terminator_call() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    call.dynamic v0, int32, 0(v0): (int32) => int32 => b1
b1(v1: int32):
    return v1
}
"#;
        let mut test = TestProgram::new(input);
        let caller = test.function_id_by_name("test");
        let callee = test.function_id_by_name("callee");
        let constraint = int32_type(&test);
        let callsite = mir::CallSite::Terminator(test.entry_block_id(caller));
        test.add_dynamic_method_table(constraint, constraint, callee);
        let profile = test.profile_dispatch_call(caller, callsite, callee, constraint, 900, 0);

        test.run_module_pass_with_profile(&DevirtualizeGuarded, profile);

        test.assert_output(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    check is.type v0, int32 => b1, b2

b1:
    call callee(v0) => b1_1

b2:
    call.dynamic v0, int32, 0(v0): (int32) => int32 => b1_1

b1_1(v1: int32):
    return v1
}
"#,
        );
    }

    /// Cold profiles leave dispatch untouched.
    #[test]
    fn test_devirtualize_guarded_preserves_cold_virtual_call() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#;
        let mut test = TestProgram::new(input);
        let caller = test.function_id_by_name("test");
        let callee = test.function_id_by_name("callee");
        let class = int32_type(&test);
        let callsite = first_virtual_instruction_call(&test, caller);
        test.add_virtual_method_table(class, callee);
        let profile = test.profile_dispatch_call(caller, callsite, callee, class, 20, 0);

        test.run_module_pass_with_profile(&DevirtualizeGuarded, profile);

        test.assert_unchanged(input);
    }

    /// Missing receiver profiles leave dispatch untouched.
    #[test]
    fn test_devirtualize_guarded_preserves_missing_receiver_profile() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#;
        let mut test = TestProgram::new(input);
        let caller = test.function_id_by_name("test");
        let callee = test.function_id_by_name("callee");
        let class = int32_type(&test);
        let callsite = first_virtual_instruction_call(&test, caller);
        test.add_virtual_method_table(class, callee);
        let mut profile = mir::Profile::new();
        test.record_function_entry(&mut profile, caller, 900);

        // record only the call target profile
        test.record_call_target_profile(&mut profile, caller, callsite, callee, 900, 0);

        test.run_module_pass_with_profile(&DevirtualizeGuarded, profile);

        test.assert_unchanged(input);
    }

    /// Stale profile hashes leave dispatch untouched.
    #[test]
    fn test_devirtualize_guarded_preserves_stale_profile() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#;
        let mut test = TestProgram::new(input);
        let caller = test.function_id_by_name("test");
        let callee = test.function_id_by_name("callee");
        let class = int32_type(&test);
        let callsite = first_virtual_instruction_call(&test, caller);
        test.add_virtual_method_table(class, callee);
        let mut profile = test.profile_dispatch_call(caller, callsite, callee, class, 900, 0);
        profile
            .functions
            .get_mut(&test.optimized.tree.get(caller).symbol)
            .expect("missing function profile")
            .hash = mir::FunctionHash(99);

        test.run_module_pass_with_profile(&DevirtualizeGuarded, profile);

        test.assert_unchanged(input);
    }

    /// Return the canonical int32 type id.
    fn int32_type(test: &TestProgram) -> mir::TypeId {
        test.optimized
            .tree
            .iter_nodes::<mir::Type>()
            .find(|(_, ty)| **ty == mir::Type::INT32)
            .expect("missing int32 type")
            .0
    }

    /// Return the first virtual instruction call in a function.
    fn first_virtual_instruction_call(
        test: &TestProgram,
        function: mir::FunctionId,
    ) -> mir::CallSite {
        let function = test.optimized.tree.get(function);

        // scan blocks in layout order
        for &block in function.blocks() {
            for &instruction in &test.optimized.tree.get(block).instructions {
                if matches!(
                    test.optimized.tree.get(instruction),
                    mir::Instruction::CallVirtual { .. }
                ) {
                    return mir::CallSite::Instruction(instruction);
                }
            }
        }

        unreachable!("missing virtual instruction call");
    }
}
