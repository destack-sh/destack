use std::collections::{HashMap, HashSet, VecDeque};

use crate::declare_mir_pass;
use destack_mir as mir;
use mir::{Instruction, Terminator, Value, ValueReference};

use crate::common::mir::terminator_arguments_for_successor;
use crate::verify::VerifyState;
use crate::verify::value::{instruction_consumes, instruction_uses, terminator_consumes};

declare_mir_pass! {
    /// Insert explicit last-use drops for verified owned values.
    #[pass(id = "drop-insert")]
    pub(crate) DropInsert,
    "Insert last-use drops"
}

/// Location where a drop should be inserted inside one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DropSite {
    /// Insert after one instruction index.
    AfterInstruction(usize),
    /// Insert before the block terminator.
    BeforeTerminator,
}

/// Availability of move-only values at a program point.
type AvailableOwned = HashSet<Value>;

/// One planned drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlannedDrop {
    /// Insertion site inside the block.
    site: DropSite,
    /// Value to drop.
    value: Value,
}

/// Drop insertions for one function.
struct DropPlan {
    /// Move-only values that need cleanup unless consumed.
    owned: HashSet<Value>,
    /// Values available at block entry.
    available_at_entry: HashMap<mir::LocalNodeId<mir::Block>, AvailableOwned>,
    /// Planned drops by block.
    drops_by_block: HashMap<mir::LocalNodeId<mir::Block>, Vec<PlannedDrop>>,
}

impl DropPlan {
    /// Build a drop plan for one function.
    fn new(function: &mir::Function, tree: &mir::Tree) -> Self {
        let owned = owned_values(function, tree);
        let available_at_entry = compute_available_entries(function, tree, &owned);
        let mut plan = Self {
            owned,
            available_at_entry,
            drops_by_block: HashMap::new(),
        };

        plan.plan(function, tree);

        plan
    }

    /// Plan drops for every reachable block.
    fn plan(&mut self, function: &mir::Function, tree: &mir::Tree) {
        let liveness = mir::FunctionLiveness::build(function, tree);

        for &block_id in &function.blocks {
            let Some(mut available) = self.available_at_entry.get(&block_id).cloned() else {
                continue;
            };

            self.plan_block(block_id, &mut available, tree, &liveness);
        }
    }

    /// Plan drops inside one block.
    fn plan_block(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        available: &mut AvailableOwned,
        tree: &mir::Tree,
        liveness: &mir::FunctionLiveness,
    ) {
        let block = tree.get(block_id);

        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            let instruction = tree.get(instruction_id);
            let consumed = consumed_by_instruction(instruction, tree, &self.owned);

            self.drop_after_instruction(
                block_id,
                index,
                instruction,
                available,
                &consumed,
                tree,
                liveness,
            );
            remove_consumed(available, &consumed);
            insert_owned_destination(available, instruction, &self.owned);
        }

        let terminator = tree.get(block.terminator);
        let consumed = consumed_by_terminator(terminator, &self.owned);
        let carried = carried_terminator_values(terminator, tree, available);

        remove_consumed(available, &consumed);
        for value in values_to_drop_before_terminator(available, &carried, block_id, liveness) {
            self.add_drop(block_id, DropSite::BeforeTerminator, value);
            available.remove(&value);
        }
    }

    /// Plan drops after one instruction.
    fn drop_after_instruction(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        index: usize,
        instruction: &Instruction,
        available: &mut AvailableOwned,
        consumed: &HashSet<Value>,
        tree: &mir::Tree,
        liveness: &mir::FunctionLiveness,
    ) {
        let used = owned_instruction_uses(instruction, tree, &self.owned);
        for value in used {
            if consumed.contains(&value) || !available.contains(&value) {
                continue;
            }
            if liveness.is_value_live_after_instruction(block_id, index, value, tree) {
                continue;
            }

            self.add_drop(block_id, DropSite::AfterInstruction(index), value);
            available.remove(&value);
        }
    }

    /// Add one planned drop.
    fn add_drop(&mut self, block: mir::LocalNodeId<mir::Block>, site: DropSite, value: Value) {
        self.drops_by_block
            .entry(block)
            .or_default()
            .push(PlannedDrop { site, value });
    }
}

impl DropInsert {
    /// Insert drops in one MIR tree.
    pub(crate) fn run(&self, tree: &mut mir::Tree, _state: &mut VerifyState<'_>) {
        let functions: Vec<_> = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        for function_id in functions {
            let function = tree.get(function_id).clone();
            if function.entry.is_none() {
                continue;
            }

            let plan = DropPlan::new(&function, tree);
            self.insert_function_drops(tree, plan.drops_by_block);
        }
    }

    /// Insert drops into one function body.
    fn insert_function_drops(
        &self,
        tree: &mut mir::Tree,
        drops_by_block: HashMap<mir::LocalNodeId<mir::Block>, Vec<PlannedDrop>>,
    ) {
        for (block_id, mut drops) in drops_by_block {
            drops.sort_by_key(|drop| {
                let index = match drop.site {
                    DropSite::AfterInstruction(index) => index,
                    DropSite::BeforeTerminator => usize::MAX,
                };

                (index, drop.value)
            });

            let mut inserted = 0;
            for drop in drops {
                let instruction_id = tree.insert(Instruction::Drop {
                    value: drop.value.into(),
                });
                let block = tree.get_mut(block_id);
                match drop.site {
                    DropSite::AfterInstruction(index) => {
                        block
                            .instructions
                            .insert(index + 1 + inserted, instruction_id);
                        inserted += 1;
                    }
                    DropSite::BeforeTerminator => {
                        block.instructions.push(instruction_id);
                    }
                }
            }
        }
    }
}

/// Compute available move-only values at each block entry.
fn compute_available_entries(
    function: &mir::Function,
    tree: &mir::Tree,
    owned: &HashSet<Value>,
) -> HashMap<mir::LocalNodeId<mir::Block>, AvailableOwned> {
    let Some(entry) = function.entry else {
        return HashMap::new();
    };

    let graph = mir::ControlFlowGraph::build(function, tree);
    let mut entries = HashMap::new();
    let mut exits = HashMap::new();
    let mut worklist = VecDeque::new();

    entries.insert(entry, owned_parameters(function, owned));
    worklist.push_back(entry);

    while let Some(block_id) = worklist.pop_front() {
        let entry_values = entry_values(block_id, function, tree, &graph, &entries, &exits, owned);
        let old_entry = entries.insert(block_id, entry_values.clone());
        let exit_values = transfer_available_block(block_id, entry_values, tree, owned);
        let old_exit = exits.insert(block_id, exit_values);

        if old_entry.as_ref() == entries.get(&block_id) && old_exit.as_ref() == exits.get(&block_id)
        {
            continue;
        }

        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        for successor in terminator.successors() {
            let Some(successor) = successor.block() else {
                continue;
            };
            if !worklist.contains(&successor) {
                worklist.push_back(successor);
            }
        }
    }

    entries
}

/// Return available values at one block entry.
fn entry_values(
    block_id: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mir::Tree,
    graph: &mir::ControlFlowGraph,
    entries: &HashMap<mir::LocalNodeId<mir::Block>, AvailableOwned>,
    exits: &HashMap<mir::LocalNodeId<mir::Block>, AvailableOwned>,
    owned: &HashSet<Value>,
) -> AvailableOwned {
    if Some(block_id) == function.entry {
        return entries.get(&block_id).cloned().unwrap_or_default();
    }

    let mut predecessors = graph
        .predecessors(block_id)
        .iter()
        .copied()
        .filter(|predecessor| exits.contains_key(predecessor));
    let Some(first) = predecessors.next() else {
        return HashSet::new();
    };
    let mut merged = edge_available_values(first, block_id, exits, tree, owned);

    for predecessor in predecessors {
        let next = edge_available_values(predecessor, block_id, exits, tree, owned);
        merged.retain(|value| next.contains(value));
    }

    merged.retain(|value| owned.contains(value));

    merged
}

/// Return available values after traversing one predecessor edge.
fn edge_available_values(
    predecessor: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    exits: &HashMap<mir::LocalNodeId<mir::Block>, AvailableOwned>,
    tree: &mir::Tree,
    owned: &HashSet<Value>,
) -> AvailableOwned {
    let mut available = exits.get(&predecessor).cloned().unwrap_or_default();
    let predecessor = tree.get(predecessor);
    let terminator = tree.get(predecessor.terminator);
    let arguments = terminator_arguments_for_successor(terminator, successor);
    let successor_block = tree.get(successor);

    for (parameter, argument) in successor_block.parameters.iter().zip(arguments) {
        let (Some(parameter), Some(argument)) = (parameter.value.value(), argument.value()) else {
            continue;
        };
        if !available.remove(&argument) || !owned.contains(&parameter) {
            continue;
        }

        available.insert(parameter);
    }

    available
}

/// Transfer availability through one block.
fn transfer_available_block(
    block_id: mir::LocalNodeId<mir::Block>,
    mut available: AvailableOwned,
    tree: &mir::Tree,
    owned: &HashSet<Value>,
) -> AvailableOwned {
    let block = tree.get(block_id);

    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        let consumed = consumed_by_instruction(instruction, tree, owned);

        remove_consumed(&mut available, &consumed);
        insert_owned_destination(&mut available, instruction, owned);
    }

    let terminator = tree.get(block.terminator);
    let consumed = consumed_by_terminator(terminator, owned);
    remove_consumed(&mut available, &consumed);

    available
}

/// Return all owned values in one function.
fn owned_values(function: &mir::Function, tree: &mir::Tree) -> HashSet<Value> {
    let mut owned = HashSet::new();

    for parameter in &function.parameters {
        let Some(value) = parameter.value.value() else {
            continue;
        };
        if value_is_owned(value, function, tree) {
            owned.insert(value);
        }
    }

    for (index, ty) in function.value_types.iter().enumerate() {
        let Some(ty) = ty else {
            continue;
        };
        if tree.get(*ty).copy().is_no() {
            owned.insert(Value::new(index as u32));
        }
    }

    owned
}

/// Return owned function parameters.
fn owned_parameters(function: &mir::Function, owned: &HashSet<Value>) -> AvailableOwned {
    function
        .parameters
        .iter()
        .filter_map(|parameter| {
            let value = parameter.value.value()?;
            owned.contains(&value).then_some(value)
        })
        .collect()
}

/// Insert an owned destination after it is defined.
fn insert_owned_destination(
    available: &mut AvailableOwned,
    instruction: &Instruction,
    owned: &HashSet<Value>,
) {
    let Some(destination) = instruction.destination().and_then(ValueReference::value) else {
        return;
    };
    if owned.contains(&destination) {
        available.insert(destination);
    }
}

/// Remove consumed values from availability.
fn remove_consumed(available: &mut AvailableOwned, consumed: &HashSet<Value>) {
    for value in consumed {
        available.remove(value);
    }
}

/// Return owned values used by one instruction.
fn owned_instruction_uses(
    instruction: &Instruction,
    tree: &mir::Tree,
    owned: &HashSet<Value>,
) -> HashSet<Value> {
    let mut used = HashSet::new();

    for value in instruction_uses(instruction, tree) {
        insert_owned_value(&mut used, value, owned);
    }

    used
}

/// Return owned values consumed by one instruction.
fn consumed_by_instruction(
    instruction: &Instruction,
    tree: &mir::Tree,
    owned: &HashSet<Value>,
) -> HashSet<Value> {
    let mut consumed = HashSet::new();

    for value in instruction_consumes(instruction, tree) {
        insert_owned_value(&mut consumed, value, owned);
    }

    consumed
}

/// Return owned values consumed by one terminator.
fn consumed_by_terminator(terminator: &Terminator, owned: &HashSet<Value>) -> HashSet<Value> {
    let mut consumed = HashSet::new();

    for value in terminator_consumes(terminator) {
        insert_owned_value(&mut consumed, value, owned);
    }

    consumed
}

/// Return owned values carried into successor block parameters.
fn carried_terminator_values(
    terminator: &Terminator,
    tree: &mir::Tree,
    available: &AvailableOwned,
) -> HashSet<Value> {
    let mut carried = HashSet::new();

    for successor in terminator.successors() {
        let Some(successor) = successor.block() else {
            continue;
        };
        let arguments = terminator_arguments_for_successor(terminator, successor);
        let successor_block = tree.get(successor);
        for (parameter, argument) in successor_block.parameters.iter().zip(arguments) {
            let (Some(_), Some(argument)) = (parameter.value.value(), argument.value()) else {
                continue;
            };
            if available.contains(&argument) {
                carried.insert(argument);
            }
        }
    }

    carried
}

/// Return values that should be dropped before one terminator.
fn values_to_drop_before_terminator(
    available: &AvailableOwned,
    carried: &HashSet<Value>,
    block_id: mir::LocalNodeId<mir::Block>,
    liveness: &mir::FunctionLiveness,
) -> Vec<Value> {
    available
        .iter()
        .copied()
        .filter(|value| !carried.contains(value))
        .filter(|value| !liveness.is_value_live_out(block_id, *value))
        .collect()
}

/// Insert one value if it is owned and concrete.
fn insert_owned_value(values: &mut HashSet<Value>, value: ValueReference, owned: &HashSet<Value>) {
    let Some(value) = value.value() else {
        return;
    };
    if owned.contains(&value) {
        values.insert(value);
    }
}

/// Return whether one value has move-only ownership.
fn value_is_owned(value: Value, function: &mir::Function, tree: &mir::Tree) -> bool {
    let Some(ty) = type_for_value(value, function) else {
        return false;
    };

    tree.get(ty).copy().is_no()
}

/// Return the known type for one value.
fn type_for_value(value: Value, function: &mir::Function) -> Option<mir::LocalNodeId<mir::Type>> {
    if let Some(ty) = function.value_type(value) {
        return Some(ty);
    }

    function
        .parameters
        .iter()
        .find(|parameter| parameter.value.value() == Some(value))
        .and_then(|parameter| parameter.ty.ty())
}

#[cfg(test)]
mod tests {
    use crate::verify::tests::VerifyProgram;

    #[test]
    fn test_insert_drop_after_last_owned_use() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, unique>): int32 {
b0(v0: ref<int32, unique>):
    v1: int32 = load v0
    return v1
}"#,
        );

        let output = program.run_drop();

        assert_eq!(
            output,
            r#"function test(value0: ref<int32, unique>): int32 {
entry0(value0: ref<int32, unique>):
    value1: int32 = load value0
    drop value0
    return value1
}
"#
        );
    }

    #[test]
    fn test_insert_drop_before_later_unrelated_work() {
        let mut program = VerifyProgram::new(
            r#"
function later(): void {
b0:
    return
}
function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    v1: int32 = load v0
    call later(): () -> void
    return
}"#,
        );

        let output = program.run_drop();

        assert_eq!(
            output,
            r#"function later(): void {
entry0:
    return
}

function test(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    value1: int32 = load value0
    drop value0
    call later(): () -> void
    return
}
"#
        );
    }

    #[test]
    fn test_insert_drop_for_unused_owned_parameter() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}"#,
        );

        let output = program.run_drop();

        assert_eq!(
            output,
            r#"function test(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    drop value0
    return
}
"#
        );
    }

    #[test]
    fn test_insert_drop_on_unconsumed_branch() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, unique>, v1: boolean): void {
b0(v0: ref<int32, unique>, v1: boolean):
    branch v1, b1(v0), b2(v0)
b1(v2: ref<int32, unique>):
    drop v2
    return
b2(v3: ref<int32, unique>):
    return
}"#,
        );

        let output = program.run_drop();

        assert_eq!(
            output,
            r#"function test(value0: ref<int32, unique>, value1: boolean): void {
entry0(value0: ref<int32, unique>, value1: boolean):
    branch value1, block1(value0), block2(value0)

block1(value2: ref<int32, unique>):
    drop value2
    return

block2(value3: ref<int32, unique>):
    drop value3
    return
}
"#
        );
    }

    #[test]
    fn test_skip_drop_after_owned_return() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, unique>): ref<int32, unique> {
b0(v0: ref<int32, unique>):
    return v0
}"#,
        );

        let output = program.run_drop();

        assert!(!output.contains("drop value0"));
    }

    #[test]
    fn test_skip_drop_after_owned_call() {
        let mut program = VerifyProgram::new(
            r#"
function consume(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}
function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    call consume(v0): (ref<int32, unique>) -> void
    return
}"#,
        );

        let output = program.run_drop();

        assert_eq!(
            output,
            r#"function consume(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    drop value0
    return
}

function test(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    call consume(value0): (ref<int32, unique>) -> void
    return
}
"#
        );
    }

    #[test]
    fn test_skip_drop_after_aggregate_field_move() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    ref<int32, unique>;
}
function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    v1: Box = struct Box (v0)
    return
}"#,
        );

        let output = program.run_drop();

        assert_eq!(
            output,
            r#"type Box {
    ref<int32, unique>;
}

function test(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    value1: Box = struct Box (value0)
    return
}
"#
        );
    }
}
