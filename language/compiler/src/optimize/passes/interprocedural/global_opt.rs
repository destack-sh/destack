use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::common::build_value_definition_map;
use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

declare_pass! {
    /// Optimize immutable globals and fold constant loads.
    ///
    /// This pass promotes mutable globals to immutable when they are never
    /// written, then rewrites direct loads to `global.const` for faster access.
    ///
    /// ```mir
    /// global @value: i32 = 42i32 ; mut
    /// function @root() -> i32 {
    /// block0:
    ///     v0 = global.addr @value -> ref<raw mut i32>
    ///     v1 = load v0 -> i32
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// global @value: i32 = 42i32 ; const
    /// function @root() -> i32 {
    /// block0:
    ///     v1 = global.const @value
    ///     return v1
    /// }
    /// ```
    #[pass(id = "global-opt")]
    pub GlobalOpt,
    "Optimize immutable globals"
}

impl ModulePass for GlobalOpt {
    /// Run global optimization for the module.
    fn run(&self, tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
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
fn run_global_opt(tree: &mut mir::NodeTree) -> bool {
    // collect global address definitions and pointer uses
    let addr_info = collect_global_addr_info(tree);
    let use_maps = build_value_use_maps(tree);

    // identify globals that are written
    let written_globals = collect_written_globals(tree, &addr_info);

    // track whether anything changed
    let mut changed = false;

    // scan globals and rewrite constant loads
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

        // ensure all global.addr uses are direct loads
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

        let mut global_changed = false;

        // rewrite direct loads into global.const
        for entry in addr_entries {
            let Some(uses) = use_maps.get(&entry.function_id) else {
                continue;
            };

            let mut entry_changed = false;

            for &use_id in uses
                .instruction_uses
                .get(&entry.destination)
                .into_iter()
                .flatten()
            {
                let mir::Instruction::Load { destination, .. } = tree.get(use_id) else {
                    continue;
                };

                // rewrite load into global.const
                *tree.get_mut(use_id) = mir::Instruction::GlobalConst {
                    destination: *destination,
                    global: global_id,
                };

                // drop memory metadata for the load
                tree.memory_table
                    .memory_accesses_by_instruction_id
                    .remove(&use_id);
                entry_changed = true;
            }

            if entry_changed {
                // remove the global.addr instruction after rewriting loads
                let block = tree.get_mut(entry.block_id);
                block.instructions.retain(|id| *id != entry.instruction_id);
                tree.debug_info
                    .instruction_locations
                    .remove(&entry.instruction_id);
                update_debug_for_removed_global_addr(
                    entry.function_id,
                    entry.destination,
                    global_id,
                    tree,
                );
                global_changed = true;
            }
        }

        // mark the global as immutable now that all loads are const
        if global_changed {
            let global = tree.get_mut(global_id);
            global.mutability = mir::Mutability::Immutable;
            changed = true;
        }
    }

    changed
}

/// Description of a global.addr instruction.
#[derive(Debug, Clone, Copy)]
struct GlobalAddrEntry {
    /// The function containing the instruction.
    function_id: mir::LocalNodeId<mir::Function>,
    /// The block containing the instruction.
    block_id: mir::LocalNodeId<mir::Block>,
    /// The instruction id for the global.addr.
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    /// The destination value for the global.addr.
    destination: mir::Value,
}

/// Collected global.addr instructions for the module.
#[derive(Debug, Default)]
struct GlobalAddrInfo {
    /// Map from global id to its address instructions.
    by_global: HashMap<mir::LocalNodeId<mir::Global>, Vec<GlobalAddrEntry>>,
    /// Map from pointer value to global id.
    by_value: HashMap<mir::Value, mir::LocalNodeId<mir::Global>>,
}

/// Collect global.addr instructions for the module.
fn collect_global_addr_info(tree: &mir::NodeTree) -> GlobalAddrInfo {
    // prepare the address info container
    let mut info = GlobalAddrInfo::default();

    // scan function bodies for global.addr
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

                let entry = GlobalAddrEntry {
                    function_id,
                    block_id,
                    instruction_id,
                    destination: *destination,
                };

                info.by_global.entry(*global).or_default().push(entry);
                info.by_value.insert(*destination, *global);
            }
        }
    }

    info
}

/// Build value use maps for each function.
fn build_value_use_maps(
    tree: &mir::NodeTree,
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

                for value in instruction.uses() {
                    uses.entry(value).or_default().push(instruction_id);
                }

                if let Some(args) = instruction.argument_slice() {
                    for &arg in tree.get_arguments(args) {
                        uses.entry(arg).or_default().push(instruction_id);
                    }
                }
            }

            // collect terminator uses
            for value in block.terminator.uses() {
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
    tree: &mir::NodeTree,
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
                    && let Some(global_id) =
                        global_addr_base(*pointer, &definitions, addr_info, tree)
                {
                    written.insert(global_id);
                    continue;
                }

                // detect local stores of global pointers
                if let mir::Instruction::LocalSet { value, .. } = instruction
                    && let Some(global_id) = global_addr_base(*value, &definitions, addr_info, tree)
                {
                    written.insert(global_id);
                    continue;
                }

                // detect raw frees or drops through global pointers
                if let mir::Instruction::RawFree { pointer }
                | mir::Instruction::RawDrop { value: pointer }
                | mir::Instruction::StackDrop { value: pointer } = instruction
                    && let Some(global_id) =
                        global_addr_base(*pointer, &definitions, addr_info, tree)
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
                if let mir::Instruction::Call { arguments, .. }
                | mir::Instruction::CallIndirect { arguments, .. } = instruction
                    && call_writes_memory(instruction_id, tree)
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
            }

            // detect tail calls that may write memory
            if let mir::Terminator::TailCall { arguments, .. }
            | mir::Terminator::TailCallIndirect { arguments, .. } = &block.terminator
                && any_argument_global_values(arguments, &definitions, addr_info, tree)
            {
                written.extend(globals_from_values(
                    arguments,
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
    tree: &mir::NodeTree,
) -> bool {
    tree.get_arguments(*arguments)
        .iter()
        .any(|value| global_addr_base(*value, definitions, addr_info, tree).is_some())
}

/// Return true when any value is derived from a global pointer.
fn any_argument_global_values(
    arguments: &[mir::Value],
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    addr_info: &GlobalAddrInfo,
    tree: &mir::NodeTree,
) -> bool {
    arguments
        .iter()
        .any(|value| global_addr_base(*value, definitions, addr_info, tree).is_some())
}

/// Collect globals referenced by argument slice values.
fn globals_from_arguments(
    arguments: &mir::ArgumentSlice,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    addr_info: &GlobalAddrInfo,
    tree: &mir::NodeTree,
) -> HashSet<mir::LocalNodeId<mir::Global>> {
    globals_from_values(tree.get_arguments(*arguments), definitions, addr_info, tree)
}

/// Collect globals referenced by value list.
fn globals_from_values(
    values: &[mir::Value],
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    addr_info: &GlobalAddrInfo,
    tree: &mir::NodeTree,
) -> HashSet<mir::LocalNodeId<mir::Global>> {
    let mut globals = HashSet::new();

    for value in values {
        if let Some(global_id) = global_addr_base(*value, definitions, addr_info, tree) {
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
    tree: &mir::NodeTree,
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
                current = *aggregate;
            }
            mir::Instruction::ElementAddr { array, .. } => {
                current = *array;
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
                    mir::Intrinsic::AddrSpaceCast | mir::Intrinsic::Transmute
                ) {
                    let argument = tree.get_arguments(*arguments).first()?;
                    current = *argument;
                    continue;
                }

                return None;
            }
            _ => return None,
        }
    }
}

/// Update debug info for a removed global.addr value.
fn update_debug_for_removed_global_addr(
    function_id: mir::LocalNodeId<mir::Function>,
    removed_value: mir::Value,
    global_id: mir::LocalNodeId<mir::Global>,
    tree: &mut mir::NodeTree,
) {
    // read the function scope for debug updates
    let Some(function_scope) = tree.debug_info.function_scopes.get(&function_id).copied() else {
        return;
    };

    // collect debug variables referencing the removed value
    let mut to_update = Vec::new();
    for (index, variable) in tree.debug_info.variables.iter().enumerate() {
        // skip variables outside the function scope
        if !scope_in_function(variable.scope, function_scope, &tree.debug_info) {
            continue;
        }

        // read the current debug value location
        let var_id = mir::DebugVariableId::new(index as u32);
        let Some(mir::DebugValueLocation::Value(value)) =
            tree.debug_info.variable_locations.get(&var_id)
        else {
            continue;
        };

        // collect variables tied to the removed value
        if *value == removed_value {
            to_update.push(var_id);
        }
    }

    // rewrite debug locations to the global value
    for var_id in to_update {
        tree.debug_info
            .variable_locations
            .insert(var_id, mir::DebugValueLocation::Global(global_id));
    }
}

/// Return true when a debug scope belongs to a function scope.
fn scope_in_function(
    scope: mir::DebugScopeId,
    function_scope: mir::DebugScopeId,
    debug_info: &mir::DebugInfoTable,
) -> bool {
    // walk the scope chain to find the function scope
    let mut current = Some(scope);

    while let Some(scope_id) = current {
        // stop once the function scope is found
        if scope_id == function_scope {
            return true;
        }

        // step to the parent scope
        current = debug_info.scope(scope_id).parent;
    }

    false
}

/// Return true when the intrinsic may write memory.
fn intrinsic_writes_memory(intrinsic: mir::Intrinsic) -> bool {
    matches!(
        intrinsic,
        mir::Intrinsic::Memcpy
            | mir::Intrinsic::Memmove
            | mir::Intrinsic::Memset
            | mir::Intrinsic::VolatileStore
            | mir::Intrinsic::AtomicStore
            | mir::Intrinsic::AtomicCas
            | mir::Intrinsic::AtomicFetchAdd
            | mir::Intrinsic::AtomicFetchSub
            | mir::Intrinsic::AtomicFetchAnd
            | mir::Intrinsic::AtomicFetchOr
            | mir::Intrinsic::AtomicFetchXor
            | mir::Intrinsic::AtomicFetchMin
            | mir::Intrinsic::AtomicFetchMax
            | mir::Intrinsic::GcWriteBarrier
    )
}

/// Return true when a callsite may write memory.
fn call_writes_memory(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    tree: &mir::NodeTree,
) -> bool {
    let Some(metadata) = tree.call_table.call_metadata(instruction_id) else {
        return true;
    };

    let Some(effects) = metadata.memory_effects.as_ref() else {
        return true;
    };

    effects.writes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use destack_source::{FileId, Span};

    /// Loads from immutable globals are rewritten to global.const.
    #[test]
    fn test_global_opt_rewrites_loads() {
        let input = r#"global @value: i32 = 42i32 ; mut
function @root() -> i32 {
block0:
    v0 = global.addr @value -> ref<raw mut i32>
    v1 = load v0 -> i32
    return v1
}"#;

        let expected = r#"global @value: i32 = 42i32 ; const
function @root() -> i32 {
block0:
    v1 = global.const @value
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&GlobalOpt);
        program.assert_output(expected);
    }

    /// Globals that are stored to remain mutable.
    #[test]
    fn test_global_opt_skips_written_global() {
        let input = r#"global @value: i32 = 0i32 ; mut
function @root() -> void {
block0:
    v0 = global.addr @value -> ref<raw mut i32>
    v1 = iconst 1i32
    store v0, v1
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&GlobalOpt);
        program.assert_output(input);
    }

    /// Address space casts that feed stores keep globals mutable.
    #[test]
    fn test_global_opt_skips_addrspace_cast_store() {
        let input = r#"global @value: i32 = 0i32 ; mut
function @root() -> void {
block0:
    v0 = global.addr @value -> ref<raw mut i32>
    v1 = intrinsic.addrspace.cast(v0)
    v2 = iconst 1i32
    store v1, v2
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&GlobalOpt);
        program.assert_output(input);
    }

    /// Terminator uses prevent const rewriting.
    #[test]
    fn test_global_opt_skips_terminator_use() {
        let input = r#"global @value: i32 = 42i32 ; mut
function @root() -> ref<raw mut i32> {
block0:
    v0 = global.addr @value -> ref<raw mut i32>
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&GlobalOpt);
        program.assert_output(input);
    }

    /// Debug locations are updated when global.addr is removed.
    #[test]
    fn test_global_opt_updates_debug_locations() {
        let input = r#"global @value: i32 = 42i32 ; mut
function @root() -> i32 {
block0:
    v0 = global.addr @value -> ref<raw mut i32>
    v1 = load v0 -> i32
    return v1
}"#;

        let expected = r#"global @value: i32 = 42i32 ; const
function @root() -> i32 {
block0:
    v1 = global.const @value
    return v1
}"#;

        let mut program = TestProgram::new(input);
        let root_id = program.function_id_by_name("root");
        let instruction_id = program
            .entry_instructions(root_id)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    program.tree.get(*instruction_id),
                    mir::Instruction::GlobalAddr { .. }
                )
            })
            .expect("missing global.addr");

        let (destination, global_id) = match program.tree.get(instruction_id) {
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => (*destination, *global),
            _ => unreachable!(),
        };

        let root_name = program.tree.get(root_id).name;
        let global_type = program.tree.get(global_id).ty;
        let file_id = FileId::new(0);
        let span = Span::empty(file_id);
        let scope_id =
            program
                .tree
                .debug_info
                .create_scope(mir::DebugScopeKind::Function, None, span, None);
        program
            .tree
            .debug_info
            .function_scopes
            .insert(root_id, scope_id);
        let var_id =
            program
                .tree
                .debug_info
                .create_variable(root_name, global_type, scope_id, false, false);
        program
            .tree
            .debug_info
            .variable_locations
            .insert(var_id, mir::DebugValueLocation::Value(destination));
        program.tree.debug_info.instruction_locations.insert(
            instruction_id,
            mir::DebugLocation {
                span,
                scope: scope_id,
                inlined_at: None,
            },
        );

        program.run_module_pass(&GlobalOpt);
        program.assert_output(expected);
        let location = program
            .tree
            .debug_info
            .variable_locations
            .get(&var_id)
            .expect("missing debug variable location");

        assert_eq!(location, &mir::DebugValueLocation::Global(global_id));
        assert!(
            !program
                .tree
                .debug_info
                .instruction_locations
                .contains_key(&instruction_id)
        );
    }
}
