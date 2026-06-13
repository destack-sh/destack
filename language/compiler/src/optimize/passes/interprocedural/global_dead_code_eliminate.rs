use std::collections::HashSet;

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

declare_mir_pass! {
    /// Remove unused local globals from the module.
    ///
    /// ```mir
    /// readonly global live: int32 = 1int32
    /// readonly global dead: int32 = 2int32
    /// function root(): int32 {
    /// b0:
    ///     v0 = global.address live
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// readonly global live: int32 = 1int32
    /// external readonly global dead: int32
    /// function root(): int32 {
    /// b0:
    ///     v0 = global.address live
    ///     return v0
    /// }
    /// ```
    #[pass(id = "global-dead-code-eliminate")]
    pub GlobalDeadCodeEliminate,
    "Eliminate unused globals"
}

impl ModulePass for GlobalDeadCodeEliminate {
    /// Run global dead code elimination for the module.
    fn run(&self, tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
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
pub(crate) fn run_global_dead_code_eliminate(tree: &mut mir::Tree) -> bool {
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
fn collect_used_globals(tree: &mir::Tree) -> HashSet<mir::LocalNodeId<mir::Global>> {
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
                if let mir::Instruction::GlobalAddr { global, .. } = tree.get(instruction_id)
                    && let Some(global) = global.global()
                {
                    used.insert(global);
                }
            }
        }
    }

    used
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Unused globals are downgraded to imports.
    #[test]
    fn test_global_dead_code_eliminate_unused_global() {
        let input = r#"
readonly global live: int32 = 1int32
readonly global dead: int32 = 2int32
function root(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address live
    v1: int32 = load v0
    return v1
}"#;

        let expected = r#"
readonly global live: int32 = 1int32
external readonly global dead: int32
function root(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address live
    v1: int32 = load v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalDeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Globals referenced via global.address are preserved.
    #[test]
    fn test_global_dead_code_eliminate_keeps_global_addr() {
        let input = r#"
readonly global live: int32 = 1int32
readonly global dead: int32 = 2int32
function root(): ref<int32, raw, readonly> {
b0:
    v0: ref<int32, raw, readonly> = global.address live
    return v0
}"#;

        let expected = r#"
readonly global live: int32 = 1int32
external readonly global dead: int32
function root(): ref<int32, raw, readonly> {
b0:
    v0: ref<int32, raw, readonly> = global.address live
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&GlobalDeadCodeEliminate);
        test.assert_output(expected);
    }
}
