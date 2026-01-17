use std::collections::HashSet;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

declare_pass! {
    /// Remove unused local globals from the module.
    ///
    /// ```mir
    /// global @live: i32 = 1i32 ; const
    /// global @dead: i32 = 2i32 ; const
    /// function @root() -> i32 {
    /// block0:
    ///     v0 = global.const @live
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// global @live: i32 = 1i32 ; const
    /// extern global @dead: i32 ; const
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
    for location in tree.debug_info.variable_locations.values() {
        if let mir::DebugValueLocation::Global(global_id) = location {
            used.insert(*global_id);
        }
    }

    used
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use destack_source::{FileId, Span};

    /// Unused globals are downgraded to imports.
    #[test]
    fn test_global_dead_code_eliminate_unused_global() {
        let input = r#"global @live: i32 = 1i32 ; const
global @dead: i32 = 2i32 ; const
function @root() -> i32 {
block0:
    v0 = global.const @live
    return v0
}"#;

        let expected = r#"global @live: i32 = 1i32 ; const
extern global @dead: i32 ; const
function @root() -> i32 {
block0:
    v0 = global.const @live
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&GlobalDeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Globals referenced via global.addr are preserved.
    #[test]
    fn test_global_dead_code_eliminate_keeps_global_addr() {
        let input = r#"global @live: i32 = 1i32 ; const
global @dead: i32 = 2i32 ; const
function @root() -> ref<raw i32> {
block0:
    v0 = global.addr @live -> ref<raw i32>
    return v0
}"#;

        let expected = r#"global @live: i32 = 1i32 ; const
extern global @dead: i32 ; const
function @root() -> ref<raw i32> {
block0:
    v0 = global.addr @live -> ref<raw i32>
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&GlobalDeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Debug references keep globals alive.
    #[test]
    fn test_global_dead_code_eliminate_keeps_debug_globals() {
        let input = r#"global @live: i32 = 1i32 ; const
global @dead: i32 = 2i32 ; const
function @root() -> i32 {
block0:
    v0 = global.const @live
    return v0
}"#;

        let mut program = TestProgram::new(input);
        let root_id = program.function_id_by_name("root");
        let live_global = program
            .entry_instructions(root_id)
            .into_iter()
            .find_map(|instruction_id| match program.tree.get(instruction_id) {
                mir::Instruction::GlobalConst { global, .. } => Some(*global),
                _ => None,
            })
            .expect("missing global.const");
        let global_id = program
            .tree
            .iter_nodes::<mir::Global>()
            .map(|(id, _)| id)
            .find(|id| *id != live_global)
            .expect("missing debug global");
        let global_name = program.tree.get(global_id).name;
        let global_type = program.tree.get(global_id).ty;
        let file_id = FileId::new(0);
        let span = Span::empty(file_id);
        let scope_id =
            program
                .tree
                .debug_info
                .create_scope(mir::DebugScopeKind::Lexical, None, span, None);
        let var_id = program.tree.debug_info.create_variable(
            global_name,
            global_type,
            scope_id,
            false,
            false,
        );
        program
            .tree
            .debug_info
            .variable_locations
            .insert(var_id, mir::DebugValueLocation::Global(global_id));

        program.run_module_pass(&GlobalDeadCodeEliminate);
        program.assert_output(input);
        let global = program.tree.get(global_id);
        assert!(global.linkage.is_defined());
    }
}
