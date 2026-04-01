use std::collections::HashSet;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

declare_pass! {
    /// Remove unused local globals from the module.
    ///
    /// ```mir
    /// global @live: i32 = 1i32 ; readonly
    /// global @dead: i32 = 2i32 ; readonly
    /// function @root() -> i32 {
    /// block0:
    ///     v0 = global.const @live
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// global @live: i32 = 1i32 ; readonly
    /// extern global @dead: i32 ; readonly
    /// function @root() -> i32 {
    /// block0:
    ///     v0 = global.const @live
    ///     return v0
    /// }
    /// ```
    #[pass(id = "global-dead-code-eliminate")]
    pub GlobalDeadCodeEliminate,
    "Eliminate unused globals"
}

impl ModulePass for GlobalDeadCodeEliminate {
    /// Run global dead code elimination for the module.
    fn run(&self, tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        let changed = run_global_dead_code_eliminate(tree);

        // report analysis preservation based on whether changes occurred
        if changed {
            ctx.strings.intern("global-dead-code-eliminate");
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "GlobalDeadCodeEliminate"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "global-dead-code-eliminate"
    }
}

/// Run global dead code elimination over the module.
pub(crate) fn run_global_dead_code_eliminate(tree: &mut mir::NodeTree) -> bool {
    // collect globals referenced by instructions
    let used_globals = collect_used_globals(tree);

    // track whether anything changed
    let mut changed = false;

    // scan globals and drop unused locals
    let global_ids: Vec<_> = tree
        .iter_nodes::<mir::Global>()
        .map(|(global_id, global)| (global_id, global.linkage))
        .collect();

    for (global_id, linkage) in global_ids {
        if linkage.is_exported() || linkage.is_import() {
            continue;
        }

        if used_globals.contains(&global_id) {
            continue;
        }

        let global = tree.get_mut(global_id);
        global.linkage = mir::Linkage::Import;
        global.initializer = None;
        changed = true;
    }

    changed
}

/// Collect globals referenced by instructions in the module.
fn collect_used_globals(tree: &mir::NodeTree) -> HashSet<mir::LocalNodeId<mir::Global>> {
    // scan the module for global references
    let mut used = HashSet::new();

    for (_, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry.is_none() {
            continue;
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                // record direct global references
                match tree.get(instruction_id) {
                    mir::Instruction::GlobalAddr { global, .. }
                    | mir::Instruction::GlobalConst { global, .. } => {
                        used.insert(*global);
                    }
                    _ => {}
                }
            }
        }
    }

    // record globals referenced by debug locations
    for ranges in tree.debug_table.binding_location_ranges.values() {
        for range in ranges {
            collect_debug_location_globals(&range.location, &mut used);
        }
    }

    used
}

/// Collect globals referenced by one debug value location.
fn collect_debug_location_globals(
    location: &mir::DebugValueLocation,
    used: &mut HashSet<mir::LocalNodeId<mir::Global>>,
) {
    // direct global locations
    if let mir::DebugValueLocation::Global(global_id) = location {
        used.insert(*global_id);
        return;
    }

    // composite fragments
    if let mir::DebugValueLocation::Composite(fragments) = location {
        for fragment in fragments {
            collect_debug_location_globals(&fragment.location, used);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use destack_source::{FileId, Span};

    /// Unused globals are downgraded to imports.
    #[test]
    fn test_global_dead_code_eliminate_unused_global() {
        let input = r#"global @live: i32 = 1i32 ; readonly
global @dead: i32 = 2i32 ; readonly
function @root() -> i32 {
block0:
    v0: i32 = global.const @live
    return v0
}"#;

        let expected = r#"global @live: i32 = 1i32 ; readonly
extern global @dead: i32 ; readonly
function @root() -> i32 {
block0:
    v0: i32 = global.const @live
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalDeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Globals referenced via global.addr are preserved.
    #[test]
    fn test_global_dead_code_eliminate_keeps_global_addr() {
        let input = r#"global @live: i32 = 1i32 ; readonly
global @dead: i32 = 2i32 ; readonly
function @root() -> ref<raw readonly i32> {
block0:
    v0: ref<raw readonly i32> = global.addr @live
    return v0
}"#;

        let expected = r#"global @live: i32 = 1i32 ; readonly
extern global @dead: i32 ; readonly
function @root() -> ref<raw readonly i32> {
block0:
    v0: ref<raw readonly i32> = global.addr @live
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalDeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Debug references keep globals alive.
    #[test]
    fn test_global_dead_code_eliminate_keeps_debug_globals() {
        let input = r#"global @live: i32 = 1i32 ; readonly
global @dead: i32 = 2i32 ; readonly
function @root() -> i32 {
block0:
    v0: i32 = global.const @live
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let live_global = test
            .entry_instructions(root_id)
            .into_iter()
            .find_map(|instruction_id| match test.tree.get(instruction_id) {
                mir::Instruction::GlobalConst { global, .. } => Some(*global),
                _ => None,
            })
            .expect("missing global.const");
        let global_id = test
            .tree
            .iter_nodes::<mir::Global>()
            .map(|(id, _)| id)
            .find(|id| *id != live_global)
            .expect("missing debug global");
        let global_name = test.tree.get(global_id).name;
        let global_type = test.tree.get(global_id).ty;
        let file_id = FileId::new(0);
        let span = Span::empty(file_id);
        let scope_id =
            test.tree
                .debug_table
                .create_scope(mir::DebugScopeKind::Lexical, None, span, None);
        let binding_id = test.tree.debug_table.create_binding(
            global_name,
            global_type,
            scope_id,
            mir::DebugBindingKind::Local,
        );
        test.tree.debug_table.binding_location_ranges.insert(
            binding_id,
            vec![mir::DebugBindingLocationRange {
                binding: binding_id,
                location: mir::DebugValueLocation::Global(global_id),
                start: mir::DebugRangeStart::function_entry(),
                end: None,
            }],
        );

        test.run_module_pass(&GlobalDeadCodeEliminate);
        test.assert_output(input);
        let global = test.tree.get(global_id);
        assert!(global.linkage.is_defined());
    }
}
