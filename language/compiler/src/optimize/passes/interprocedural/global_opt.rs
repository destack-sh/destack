use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::build_value_definition_map;
use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

declare_mir_pass! {
    /// Mark private globals readonly when no write can reach them.
    ///
    /// This pass promotes mutable globals to immutable when they are never written.
    ///
    /// ```mir
    /// global value: int32 = 42int32
    /// function root(): int32 {
    /// b0:
    ///     v0 = global.address value -> ref<int32, raw>
    ///     v1 = load v0 -> int32
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// readonly global value: int32 = 42int32
    /// function root(): int32 {
    /// b0:
    ///     v0 = global.address value -> ref<int32, raw>
    ///     v1 = load v0 -> int32
    ///     return v1
    /// }
    /// ```
    #[pass(id = "global-opt", requires(call_effects))]
    pub GlobalOpt,
    "Optimize immutable globals"
}

impl ModulePass for GlobalOpt {
    /// Run global optimization for the module.
    fn run(&self, tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        let changed = run_global_opt(tree);

        // report analysis preservation based on whether changes occurred
        if changed {
            ctx.strings.intern("global-opt");
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "GlobalOpt"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "global-opt"
    }
}

/// Run global optimizations over the module.
fn run_global_opt(tree: &mut mir::Tree) -> bool {
    // collect global address definitions and pointer uses
    let addr_info = collect_global_addr_info(tree);
    let use_maps = build_value_use_maps(tree);

    // identify globals that are written
    let written_globals = collect_written_globals(tree, &addr_info);

    // track whether anything changed
    let mut changed = false;

    // scan globals that can become readonly
    let globals: Vec<_> = tree
        .iter_nodes::<mir::Global>()
        .map(|(global_id, global)| (global_id, global.linkage, global.mutability))
        .collect();

    for (global_id, linkage, mutability) in globals {
        if linkage.is_exported() || linkage.is_import() {
            continue;
        }
        if mutability != mir::Mutability::Mutable {
            continue;
        }
        if written_globals.contains(&global_id) {
            continue;
        }
        let Some(addr_entries) = addr_info.by_global.get(&global_id) else {
            continue;
        };

        // ensure all global.address uses are direct loads
        if !addr_entries.iter().all(|entry| {
            let Some(uses) = use_maps.get(&entry.function_id) else {
                return true;
            };

            if uses.terminator_uses.contains(&entry.destination) {
                return false;
            }

            uses.instruction_uses
                .get(&entry.destination)
                .is_none_or(|uses| {
                    uses.iter()
                        .all(|use_id| matches!(tree.get(*use_id), mir::Instruction::Load { .. }))
                })
        }) {
            continue;
        }

        // mark the global as immutable when no writes remain
        let global = tree.get_mut(global_id);
        global.mutability = mir::Mutability::Immutable;
        changed = true;
    }

    changed
}

/// Description of a global.address instruction.
#[derive(Debug, Clone, Copy)]
struct GlobalAddrEntry {
    /// The function containing the instruction.
    function_id: mir::LocalNodeId<mir::Function>,
    /// The destination value for the global.address.
    destination: mir::Value,
}

/// Collected global.address instructions for the module.
#[derive(Debug, Default)]
struct GlobalAddrInfo {
    /// Map from global id to its address instructions.
    by_global: HashMap<mir::LocalNodeId<mir::Global>, Vec<GlobalAddrEntry>>,
    /// Map from pointer value to global id.
    by_value: HashMap<mir::Value, mir::LocalNodeId<mir::Global>>,
}

/// Collect global.address instructions for the module.
fn collect_global_addr_info(tree: &mir::Tree) -> GlobalAddrInfo {
    // prepare the address info container
    let mut info = GlobalAddrInfo::default();

    // scan function bodies for global.address
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry.is_none() {
            continue;
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let mir::Instruction::GlobalAddr {
                    destination,
                    global,
                    ..
                } = tree.get(instruction_id)
                else {
                    continue;
                };

                let Some(destination) = destination.value() else {
                    continue;
                };
                let Some(global) = global.global() else {
                    continue;
                };

                let entry = GlobalAddrEntry {
                    function_id,
                    destination,
                };

                info.by_global.entry(global).or_default().push(entry);
                info.by_value.insert(destination, global);
            }
        }
    }

    info
}

/// Build value use maps for each function.
fn build_value_use_maps(
    tree: &mir::Tree,
) -> HashMap<mir::LocalNodeId<mir::Function>, ValueUseInfo> {
    // prepare the cache container
    let mut cache = HashMap::new();

    // build use maps per function
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry.is_none() {
            continue;
        }

        let mut uses: HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Instruction>>> = HashMap::new();
        let mut terminator_uses = HashSet::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                for value in instruction
                    .uses()
                    .into_iter()
                    .filter_map(|value| value.value())
                {
                    uses.entry(value).or_default().push(instruction_id);
                }

                if let Some(args) = instruction.argument_slice() {
                    for arg in tree
                        .get_arguments(args)
                        .iter()
                        .cloned()
                        .filter_map(|value| value.value())
                    {
                        uses.entry(arg).or_default().push(instruction_id);
                    }
                }
            }

            // collect terminator uses
            let terminator = tree.get(block.terminator);
            for value in terminator
                .uses()
                .into_iter()
                .filter_map(|value| value.value())
            {
                terminator_uses.insert(value);
            }
        }

        cache.insert(
            function_id,
            ValueUseInfo {
                instruction_uses: uses,
                terminator_uses,
            },
        );
    }

    cache
}

/// Use information for a function's values.
#[derive(Debug, Default)]
struct ValueUseInfo {
    /// Instruction uses keyed by value.
    instruction_uses: HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Instruction>>>,
    /// Terminator uses keyed by value.
    terminator_uses: HashSet<mir::Value>,
}

/// Collect globals that are written by stores or memory effects.
fn collect_written_globals(
    tree: &mir::Tree,
    addr_info: &GlobalAddrInfo,
) -> HashSet<mir::LocalNodeId<mir::Global>> {
    // prepare the written set
    let mut written = HashSet::new();

    // scan each function for writes
    for (_function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry.is_none() {
            continue;
        }

        let definitions = build_value_definition_map(function, tree);

        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                // detect direct stores through global pointers
                if let mir::Instruction::Store { pointer, .. } = instruction
                    && let Some(global_id) = pointer.value().and_then(|pointer| {
                        global_addr_base(pointer, &definitions, addr_info, tree)
                    })
                {
                    written.insert(global_id);
                    continue;
                }

                // detect local stores of global pointers
                if let mir::Instruction::LocalSet { value, .. } = instruction
                    && let Some(global_id) = value
                        .value()
                        .and_then(|value| global_addr_base(value, &definitions, addr_info, tree))
                {
                    written.insert(global_id);
                    continue;
                }

                // detect frees through global pointers
                if let mir::Instruction::Free { value: pointer } = instruction
                    && let Some(global_id) = pointer.value().and_then(|pointer| {
                        global_addr_base(pointer, &definitions, addr_info, tree)
                    })
                {
                    written.insert(global_id);
                    continue;
                }

                // detect memory writing intrinsics
                if let mir::Instruction::Intrinsic {
                    intrinsic,
                    arguments,
                    ..
                } = instruction
                    && intrinsic_writes_memory(*intrinsic)
                    && any_argument_global(arguments, &definitions, addr_info, tree)
                {
                    written.extend(globals_from_arguments(
                        arguments,
                        &definitions,
                        addr_info,
                        tree,
                    ));
                    continue;
                }

                // detect calls that may write memory
                if let mir::Instruction::Call { call, .. }
                | mir::Instruction::CallVirtual { call, .. }
                | mir::Instruction::CallDynamic { call, .. } = instruction
                    && call_writes_memory(tree, instruction_id, instruction)
                    && any_argument_global(&call.arguments, &definitions, addr_info, tree)
                {
                    written.extend(globals_from_arguments(
                        &call.arguments,
                        &definitions,
                        addr_info,
                        tree,
                    ));
                    continue;
                }

                if let mir::Instruction::CallIndirect { call, .. } = instruction
                    && call_writes_memory(tree, instruction_id, instruction)
                    && any_argument_global(&call.arguments, &definitions, addr_info, tree)
                {
                    written.extend(globals_from_arguments(
                        &call.arguments,
                        &definitions,
                        addr_info,
                        tree,
                    ));
                    continue;
                }
            }

            // detect call terminators that may write memory
            let terminator = tree.get(block.terminator);
            let terminator_arguments = terminator_write_arguments(tree, block_id, terminator);
            if let Some(arguments) = terminator_arguments
                && any_argument_global_values(&arguments, &definitions, addr_info, tree)
            {
                written.extend(globals_from_values(
                    &arguments,
                    &definitions,
                    addr_info,
                    tree,
                ));
            }
        }
    }

    written
}

/// Return true when any argument is derived from a global pointer.
fn any_argument_global(
    arguments: &mir::ArgumentSlice,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    addr_info: &GlobalAddrInfo,
    tree: &mir::Tree,
) -> bool {
    tree.get_arguments(*arguments)
        .iter()
        .filter_map(|value| value.value())
        .any(|value| global_addr_base(value, definitions, addr_info, tree).is_some())
}

/// Return true when any value is derived from a global pointer.
fn any_argument_global_values(
    arguments: &[mir::ValueReference],
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    addr_info: &GlobalAddrInfo,
    tree: &mir::Tree,
) -> bool {
    arguments
        .iter()
        .filter_map(|value| value.value())
        .any(|value| global_addr_base(value, definitions, addr_info, tree).is_some())
}

/// Collect globals referenced by argument slice values.
fn globals_from_arguments(
    arguments: &mir::ArgumentSlice,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    addr_info: &GlobalAddrInfo,
    tree: &mir::Tree,
) -> HashSet<mir::LocalNodeId<mir::Global>> {
    globals_from_values(tree.get_arguments(*arguments), definitions, addr_info, tree)
}

/// Collect globals referenced by value list.
fn globals_from_values(
    values: &[mir::ValueReference],
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    addr_info: &GlobalAddrInfo,
    tree: &mir::Tree,
) -> HashSet<mir::LocalNodeId<mir::Global>> {
    let mut globals = HashSet::new();

    for value in values.iter().filter_map(|value| value.value()) {
        if let Some(global_id) = global_addr_base(value, definitions, addr_info, tree) {
            globals.insert(global_id);
        }
    }

    globals
}

/// Return the base global for a derived pointer, if any.
fn global_addr_base(
    value: mir::Value,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    addr_info: &GlobalAddrInfo,
    tree: &mir::Tree,
) -> Option<mir::LocalNodeId<mir::Global>> {
    // walk pointer definitions to find the base address
    let mut current = value;
    let mut visited = HashSet::new();

    loop {
        if !visited.insert(current) {
            return None;
        }

        if let Some(global) = addr_info.by_value.get(&current) {
            return Some(*global);
        }

        let instruction_id = definitions.get(&current)?;
        let instruction = tree.get(*instruction_id);

        match instruction {
            mir::Instruction::FieldAddr { aggregate, .. } => {
                current = aggregate.value()?;
            }
            mir::Instruction::ElementAddr { array, .. } => {
                current = array.value()?;
            }
            mir::Instruction::Cast { argument, .. } => {
                current = argument.value()?;
            }
            mir::Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => {
                if matches!(
                    intrinsic,
                    mir::Intrinsic::SpaceCast | mir::Intrinsic::Transmute
                ) {
                    let argument = tree.get_arguments(*arguments).first()?;
                    current = argument.value()?;
                    continue;
                }

                return None;
            }
            _ => return None,
        }
    }
}

/// Return true when the intrinsic may write memory.
fn intrinsic_writes_memory(intrinsic: mir::Intrinsic) -> bool {
    matches!(
        intrinsic,
        mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove | mir::Intrinsic::Memset
    )
}

/// Return true when a callsite may write memory.
fn call_writes_memory(
    tree: &mir::Tree,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
) -> bool {
    let callsite = mir::CallSite::Instruction(instruction_id);
    if let Some(metadata) = tree.metadata.functions.call(callsite) {
        return metadata.memory.writes;
    }

    let Some(function) = instruction
        .call_direct_target()
        .and_then(|target| target.function())
    else {
        return true;
    };

    function_memory_writes_from_tree(tree, function)
}

/// Return terminator arguments when the terminator may write memory.
fn terminator_write_arguments(
    tree: &mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
) -> Option<Vec<mir::ValueReference>> {
    match terminator {
        mir::Terminator::Call { function, call, .. } => {
            let function = function.function()?;
            function_memory_writes_from_tree(tree, function).then(|| call.arguments.clone())
        }
        mir::Terminator::CallIndirect { call, .. } => Some(call.arguments.clone()),
        mir::Terminator::CallVirtual { receiver, call, .. }
        | mir::Terminator::CallDynamic { receiver, call, .. }
        | mir::Terminator::TailCallVirtual { receiver, call, .. }
        | mir::Terminator::TailCallDynamic { receiver, call, .. } => {
            let target = tree
                .metadata
                .functions
                .call(mir::CallSite::Terminator(block_id))
                .and_then(|metadata| metadata.target);

            let may_write = target
                .map(|function| function_memory_writes_from_tree(tree, function))
                .unwrap_or(true);

            if !may_write {
                return None;
            }

            let mut values = call.arguments.clone();
            values.push(*receiver);
            Some(values)
        }
        mir::Terminator::TailCall { function, call, .. } => {
            let function = function.function()?;
            function_memory_writes_from_tree(tree, function).then(|| call.arguments.clone())
        }
        mir::Terminator::TailCallIndirect { call, .. } => Some(call.arguments.clone()),
        _ => None,
    }
}

/// Return true when a function summary may write memory.
fn function_memory_writes_from_tree(
    tree: &mir::Tree,
    function: mir::LocalNodeId<mir::Function>,
) -> bool {
    tree.metadata
        .functions
        .function(function)
        .map(|metadata| metadata.memory.writes)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Unwritten private globals are marked readonly.
    #[test]
    fn test_global_opt_marks_unwritten_global_readonly() {
        let input = r#"
global value: int32 = 42int32
function root(): int32 {
b0:
    v0: ref<int32, raw> = global.address value
    v1: int32 = load v0
    return v1
}"#;

        let expected = r#"
readonly global value: int32 = 42int32
function root(): int32 {
b0:
    v0: ref<int32, raw> = global.address value
    v1: int32 = load v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalOpt);
        test.assert_output(expected);
    }

    /// Globals that are stored to remain mutable.
    #[test]
    fn test_global_opt_skips_written_global() {
        let input = r#"
global value: int32 = 0int32
function root(): void {
b0:
    v0: ref<int32, raw> = global.address value
    v1: int32 = 1int32
    store v0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalOpt);
        test.assert_output(input);
    }

    /// Space casts that feed stores keep globals mutable.
    #[test]
    fn test_global_opt_skips_space_cast_store() {
        let input = r#"
global value: int32 = 0int32
function root(): void {
b0:
    v0: ref<int32, raw> = global.address value
    v1: ref<int32, raw> = intrinsic.space.cast(v0)
    v2: int32 = 1int32
    store v1, v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalOpt);
        test.assert_output(input);
    }

    /// Terminator uses keep globals mutable.
    #[test]
    fn test_global_opt_skips_terminator_use() {
        let input = r#"
global value: int32 = 42int32
function root(): ref<int32, raw> {
b0:
    v0: ref<int32, raw> = global.address value
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalOpt);
        test.assert_output(input);
    }

    /// Call terminators keep written globals mutable.
    #[test]
    fn test_global_opt_skips_call_terminator_global_write() {
        let input = r#"
global value: int32 = 0int32
function write(v0: ref<int32, raw>): void {
b0(v0: ref<int32, raw>):
    v1: int32 = 1int32
    store v0, v1
    return
}
function root(v0: ref<void, managed, readonly>): void {
b0(v0: ref<void, managed, readonly>):
    v1: ref<int32, raw> = global.address value
    call write(v1): (ref<int32, raw>) -> void -> b1
b1:
    return
b2(v2: ref<void, managed, readonly>):
    panic v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalOpt);
        test.assert_output(input);
    }
}
