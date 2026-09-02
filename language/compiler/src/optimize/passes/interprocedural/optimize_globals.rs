use destack_core::{FxIndexMap, FxIndexSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{MirOptimized, ModulePass, PipelineContext};
use destack_mir::{DefinitionTable, EffectTable, Mutation, UseTable, ValueUse};

declare_pass! {
    /// Mark private globals readonly when no write can reach them.
    ///
    /// ```mir
    /// global value: int32 = 42
    /// function root(): int32 {
    /// entry:
    ///     v0: ref<int32, borrowed, mutable> = global.address value
    ///     v1: int32 = load v0
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// readonly global value: int32 = 42
    /// function root(): int32 {
    /// entry:
    ///     v0: ref<int32, borrowed, mutable> = global.address value
    ///     v1: int32 = load v0
    ///     return v1
    /// }
    /// ```
    #[pass(id = "optimize-globals")]
    pub OptimizeGlobals,
    "Optimize immutable globals"
}

impl ModulePass for OptimizeGlobals {
    /// Run global optimization for the module.
    fn run(
        &self,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mut mir::AnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let accesses = &optimized.accesses;
        let effects = &mut optimized.effects;
        let dispatch = &optimized.dispatch;

        let resolution = analyses.resolution(tree, dispatch);
        let function_effects = analyses.effect(tree, accesses, effects, dispatch);
        let changed = run_optimize_globals(tree, effects, &resolution, &function_effects);

        // report what this pass changed
        if changed {
            ctx.strings.intern("optimize-globals");
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Run global optimizations over the module.
fn run_optimize_globals(
    tree: &mut mir::Tree,
    effects: &mir::EffectTable,
    resolution: &mir::ResolutionTable,
    function_effects: &EffectTable,
) -> bool {
    // collect global address definitions and pointer uses
    let addresses = collect_global_addresses(tree);
    let use_tables: FxIndexMap<_, _> = tree
        .iter_nodes::<mir::Function>()
        .filter(|(_, function)| function.entry().is_some())
        .map(|(function_id, function)| (function_id, UseTable::build(function, tree)))
        .collect();

    // identify globals that are written
    let written_globals =
        collect_written_globals(tree, &addresses, effects, resolution, function_effects);

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
        let Some(entries) = addresses.by_global.get(&global_id) else {
            continue;
        };

        // require every global.address use to be a direct load
        if !entries.iter().all(|entry| {
            let uses = use_tables.get(&entry.function_id).unwrap_or_else(|| {
                panic!(
                    "missing global address use table for function: {:?}",
                    entry.function_id
                )
            });

            uses.uses(entry.destination)
                .iter()
                .all(|value_use| match value_use {
                    ValueUse::Instruction { instruction, .. } => {
                        matches!(tree.get(*instruction), mir::Instruction::Load { .. })
                    }
                    ValueUse::Terminator { .. } => false,
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

/// One global.address instruction and the function holding it.
#[derive(Debug, Clone, Copy)]
struct GlobalAddress {
    /// The function containing the instruction.
    function_id: mir::LocalNodeId<mir::Function>,
    /// The destination value for the global.address.
    destination: mir::Value,
}

/// The global.address instructions of one module, indexed by global and by reference value.
#[derive(Debug, Default)]
struct GlobalAddresses {
    /// Map from global id to its address instructions.
    by_global: FxIndexMap<mir::LocalNodeId<mir::Global>, Vec<GlobalAddress>>,
    /// Map from reference value to global id.
    by_value: FxIndexMap<mir::Value, mir::LocalNodeId<mir::Global>>,
}

/// Collect global.address instructions for the module.
fn collect_global_addresses(tree: &mir::Tree) -> GlobalAddresses {
    let mut addresses = GlobalAddresses::default();

    // scan function bodies for global.address
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry().is_none() {
            continue;
        }

        for &block_id in function.blocks() {
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

                let destination = *destination;
                let global = *global;

                let entry = GlobalAddress {
                    function_id,
                    destination,
                };

                addresses.by_global.entry(global).or_default().push(entry);
                addresses.by_value.insert(destination, global);
            }
        }
    }

    addresses
}

/// Collect globals that are written by stores or memory effects.
fn collect_written_globals(
    tree: &mir::Tree,
    addresses: &GlobalAddresses,
    effects: &mir::EffectTable,
    resolution: &mir::ResolutionTable,
    function_effects: &EffectTable,
) -> FxIndexSet<mir::LocalNodeId<mir::Global>> {
    // prepare the written set
    let mut written = FxIndexSet::default();

    // scan each function for writes
    for (_function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry().is_none() {
            continue;
        }

        let definitions = DefinitionTable::build(function, tree);

        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                // detect direct stores through global pointers
                if let mir::Instruction::Store { pointer, .. } = instruction
                    && let Some(global_id) =
                        global_addr_base(*pointer, &definitions, addresses, tree)
                {
                    written.insert(global_id);
                    continue;
                }

                // detect local stores of global pointers
                if let mir::Instruction::LocalSet { value, .. } = instruction
                    && let Some(global_id) = global_addr_base(*value, &definitions, addresses, tree)
                {
                    written.insert(global_id);
                    continue;
                }

                // detect frees through global pointers
                if let mir::Instruction::Free { value: pointer } = instruction
                    && let Some(global_id) =
                        global_addr_base(*pointer, &definitions, addresses, tree)
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
                    && any_argument_global(arguments, &definitions, addresses, tree)
                {
                    written.extend(globals_from_arguments(
                        arguments,
                        &definitions,
                        addresses,
                        tree,
                    ));
                    continue;
                }

                // detect calls that may write memory
                if let mir::Instruction::Call { call, .. } = instruction
                    && call_writes_memory(
                        instruction_id,
                        instruction,
                        effects,
                        resolution,
                        function_effects,
                    )
                    && any_argument_global(&call.arguments, &definitions, addresses, tree)
                {
                    written.extend(globals_from_arguments(
                        &call.arguments,
                        &definitions,
                        addresses,
                        tree,
                    ));
                    continue;
                }
            }

            // detect invokes and tail calls that may write memory
            let terminator = tree.get(block.terminator);
            let terminator_arguments = terminator_write_arguments(
                tree,
                block_id,
                terminator,
                resolution,
                function_effects,
            );
            if let Some(arguments) = terminator_arguments
                && any_argument_global_values(&arguments, &definitions, addresses, tree)
            {
                written.extend(globals_from_values(
                    &arguments,
                    &definitions,
                    addresses,
                    tree,
                ));
            }
        }
    }

    written
}

/// Return true when any argument is derived from a global pointer.
fn any_argument_global(
    arguments: &mir::ValueSlice,
    definitions: &DefinitionTable,
    addresses: &GlobalAddresses,
    tree: &mir::Tree,
) -> bool {
    tree.get_values(*arguments)
        .iter()
        .copied()
        .any(|value| global_addr_base(value, definitions, addresses, tree).is_some())
}

/// Return true when any value is derived from a global pointer.
fn any_argument_global_values(
    arguments: &[mir::Value],
    definitions: &DefinitionTable,
    addresses: &GlobalAddresses,
    tree: &mir::Tree,
) -> bool {
    arguments
        .iter()
        .copied()
        .any(|value| global_addr_base(value, definitions, addresses, tree).is_some())
}

/// Collect globals referenced by argument slice values.
fn globals_from_arguments(
    arguments: &mir::ValueSlice,
    definitions: &DefinitionTable,
    addresses: &GlobalAddresses,
    tree: &mir::Tree,
) -> FxIndexSet<mir::LocalNodeId<mir::Global>> {
    globals_from_values(tree.get_values(*arguments), definitions, addresses, tree)
}

/// Collect globals referenced by value list.
fn globals_from_values(
    values: &[mir::Value],
    definitions: &DefinitionTable,
    addresses: &GlobalAddresses,
    tree: &mir::Tree,
) -> FxIndexSet<mir::LocalNodeId<mir::Global>> {
    let mut globals = FxIndexSet::default();

    for value in values.iter().copied() {
        if let Some(global_id) = global_addr_base(value, definitions, addresses, tree) {
            globals.insert(global_id);
        }
    }

    globals
}

/// Return the base global for a derived pointer, if any.
fn global_addr_base(
    value: mir::Value,
    definitions: &DefinitionTable,
    addresses: &GlobalAddresses,
    tree: &mir::Tree,
) -> Option<mir::LocalNodeId<mir::Global>> {
    // walk reference definitions to find the base address
    let mut current = value;
    let mut visited = FxIndexSet::default();

    loop {
        if !visited.insert(current) {
            return None;
        }

        if let Some(global) = addresses.by_value.get(&current) {
            return Some(*global);
        }

        let instruction_id = definitions.instruction(current)?;
        let instruction = tree.get(instruction_id);

        match instruction {
            mir::Instruction::FieldAddr { aggregate, .. } => {
                current = *aggregate;
            }
            mir::Instruction::ElementAddr { base, .. } => {
                current = *base;
            }
            mir::Instruction::Cast { argument, .. } => {
                current = *argument;
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
                    current = *tree.get_values(*arguments).first()?;
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
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    effects: &mir::EffectTable,
    resolution: &mir::ResolutionTable,
    function_effects: &EffectTable,
) -> bool {
    let callsite = mir::Point::Instruction(instruction_id);
    let tables = effects.call(callsite);
    if let Some(tables) = tables
        && tables.memory != mir::MemoryEffect::unknown()
    {
        return tables.memory.writes();
    }

    let function = instruction
        .call_direct_target()
        .or_else(|| resolution.target(callsite));
    let Some(function) = function else {
        return true;
    };

    function_memory_writes(function_effects, function)
}

/// Return terminator arguments when the terminator may write memory.
fn terminator_write_arguments(
    tree: &mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
    resolution: &mir::ResolutionTable,
    function_effects: &EffectTable,
) -> Option<Vec<mir::Value>> {
    match terminator {
        mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => {
            let callsite = mir::Point::Terminator(block_id);
            let target = call
                .callee
                .function()
                .or_else(|| resolution.target(callsite));

            let may_write = target
                .map(|function| function_memory_writes(function_effects, function))
                .unwrap_or(true);

            if !may_write {
                return None;
            }

            Some(call.uses(tree).into_iter().collect())
        }
        _ => None,
    }
}

/// Return true when a function effect may write memory.
fn function_memory_writes(
    effects: &EffectTable,
    function: mir::LocalNodeId<mir::Function>,
) -> bool {
    effects
        .function(function)
        .map(|effect| effect.memory.writes())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Unwritten private globals are marked readonly.
    #[test]
    fn test_optimize_globals_marks_unwritten_global_readonly() {
        let input = r#"
global value: int32 = 42

function root(): int32 {
entry:
    v0: ref<int32, borrowed, mutable> = global.address value
    v1: int32 = load v0
    return v1
}
"#;

        let expected = r#"
readonly global value: int32 = 42

function root(): int32 {
entry:
    v0: ref<int32, borrowed, mutable> = global.address value
    v1: int32 = load v0
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&OptimizeGlobals);
        test.assert_output(expected);
    }

    /// Globals that are stored to remain mutable.
    #[test]
    fn test_optimize_globals_skips_written_global() {
        let input = r#"
global value: int32 = 0

function root(): void {
entry:
    v0: ref<int32, borrowed, mutable> = global.address value
    v1: int32 = 1
    store v0, v1
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&OptimizeGlobals);
        test.assert_output(input);
    }

    /// Space casts that feed stores keep globals mutable.
    #[test]
    fn test_optimize_globals_skips_space_cast_store() {
        let input = r#"
global value: int32 = 0

function root(): void {
entry:
    v0: ref<int32, borrowed, mutable> = global.address value
    v1: ref<int32, borrowed, mutable> = intrinsic.space.cast(v0)
    v2: int32 = 1
    store v1, v2
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&OptimizeGlobals);
        test.assert_output(input);
    }

    /// Terminator uses keep globals mutable.
    #[test]
    fn test_optimize_globals_skips_terminator_use() {
        let input = r#"
global value: int32 = 42

function root(): ref<int32, borrowed, mutable> {
entry:
    v0: ref<int32, borrowed, mutable> = global.address value
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&OptimizeGlobals);
        test.assert_output(input);
    }

    /// Call terminators keep written globals mutable.
    #[test]
    fn test_optimize_globals_skips_invoke_global_write() {
        let input = r#"
global value: int32 = 0

function write(v0: ref<int32, borrowed, mutable>): void {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = 1
    store v0, v1
    return
}

function root(): void {
entry:
    v1: ref<int32, borrowed, mutable> = global.address value
    invoke write(v1): (ref<int32, borrowed, mutable>) => void => b1 | b2

b1:
    return

b2:
    unwind.resume
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&OptimizeGlobals);
        test.assert_output(input);
    }
}
