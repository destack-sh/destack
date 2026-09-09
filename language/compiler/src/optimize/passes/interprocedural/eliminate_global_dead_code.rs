use destack_core::FxIndexSet;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{MirOptimized, ModulePass, PipelineContext};
use destack_mir::Mutation;

declare_pass! {
    /// Remove unused local globals from the module.
    ///
    /// ```mir
    /// readonly global live: int32 = 1
    /// readonly global dead: int32 = 2
    /// function root<'a>(): ref<int32, borrowed, 'a, readonly, local> {
    /// b0:
    ///     v0: ref<int32, borrowed, 'static, readonly, local> = global.address live
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// readonly global live: int32 = 1
    /// external readonly global dead: int32
    /// function root<'a>(): ref<int32, borrowed, 'a, readonly, local> {
    /// b0:
    ///     v0: ref<int32, borrowed, 'static, readonly, local> = global.address live
    ///     return v0
    /// }
    /// ```
    #[pass(id = "eliminate-global-dead-code")]
    pub EliminateGlobalDeadCode,
    "Eliminate unused globals"
}

impl ModulePass for EliminateGlobalDeadCode {
    /// Run global dead code elimination for the module.
    fn run(
        &self,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        _analyses: &mut mir::AnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;

        let changed = run_eliminate_global_dead_code(tree);

        // report what this pass changed
        if changed {
            ctx.strings.intern("eliminate-global-dead-code");
            Mutation::CONTROL
        } else {
            Mutation::NONE
        }
    }
}

/// Run global dead code elimination over the module.
pub(crate) fn run_eliminate_global_dead_code(tree: &mut mir::Tree) -> bool {
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
fn collect_used_globals(tree: &mir::Tree) -> FxIndexSet<mir::LocalNodeId<mir::Global>> {
    // scan the module for global references
    let mut used = FxIndexSet::default();

    for (_, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry().is_none() {
            continue;
        }

        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                // record direct global references
                if let mir::Instruction::GlobalAddr { global, .. } = tree.get(instruction_id) {
                    used.insert(*global);
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
    fn test_eliminate_global_dead_code_unused_global() {
        let input = r#"
readonly global live: int32 = 1

readonly global dead: int32 = 2

function root(): int32 {
entry:
    v0: ref<int32, borrowed, 'static, readonly, local> = global.address live
    v1: int32 = load v0
    return v1
}
"#;

        let expected = r#"
readonly global live: int32 = 1

external readonly global dead: int32

function root(): int32 {
entry:
    v0: ref<int32, borrowed, 'static, readonly, local> = global.address live
    v1: int32 = load v0
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateGlobalDeadCode);
        test.assert_output(expected);
    }

    /// Globals referenced via global.address are preserved.
    #[test]
    fn test_eliminate_global_dead_code_keeps_global_addr() {
        let input = r#"
readonly global live: int32 = 1

readonly global dead: int32 = 2

function root<'a>(): ref<int32, borrowed, 'a, readonly, local> {
entry:
    v0: ref<int32, borrowed, 'static, readonly, local> = global.address live
    return v0
}
"#;

        let expected = r#"
readonly global live: int32 = 1

external readonly global dead: int32

function root<'a>(): ref<int32, borrowed, 'a, readonly, local> {
entry:
    v0: ref<int32, borrowed, 'static, readonly, local> = global.address live
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateGlobalDeadCode);
        test.assert_output(expected);
    }
}
