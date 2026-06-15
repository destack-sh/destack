use crate::optimize::declare_pass;
use destack_artifact::ProgramAnalysis;
use destack_mir as mir;

use crate::optimize::passes::interprocedural::{
    run_dead_function_eliminate, run_global_dead_code_eliminate,
};
use crate::optimize::{ModulePass, PipelineContext};
use destack_mir::Mutation;

declare_pass! {
    /// Run interprocedural cleanup after cross function optimizations.
    ///
    /// This pass prunes dead functions and globals.
    ///
    /// ```mir
    /// readonly global dead: int32 = 1int32
    /// function dead(): int32 {
    /// b0:
    ///     v0 = 2int32
    ///     return v0
    /// }
    /// function root(): int32 {
    /// b0:
    ///     v0 = 1int32
    ///     v1 = int.add v0, v0
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// external readonly global dead: int32
    /// external function dead(): int32
    /// function root(): int32 {
    /// b0:
    ///     v0 = 1int32
    ///     v1 = int.add v0, v0
    ///     return v0
    /// }
    /// ```
    #[pass(id = "ip-dce-cleanup")]
    pub InterproceduralDceCleanup,
    "Clean up dead code after interprocedural passes"
}

impl ModulePass for InterproceduralDceCleanup {
    /// Run interprocedural cleanup for the module.
    fn run(
        &self,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
        _analyses: &mir::ModuleAnalyses,
    ) -> Mutation {
        // run the cleanup pass
        let changed = run_interprocedural_dce_cleanup(tree, ctx.program_analysis());

        // report what this pass changed
        if changed {
            ctx.strings.intern("ip-dce-cleanup");
            Mutation::CONTROL_FLOW
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "InterproceduralDceCleanup"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "ip-dce-cleanup"
    }
}

/// Run interprocedural cleanup over the module.
fn run_interprocedural_dce_cleanup(tree: &mut mir::Tree, program: &ProgramAnalysis) -> bool {
    let mut changed = false;

    if run_dead_function_eliminate(tree, program) {
        changed = true;
    }

    if run_global_dead_code_eliminate(tree) {
        changed = true;
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Dead functions and globals are removed after cleanup.
    #[test]
    fn test_ip_dce_cleanup_removes_dead_items() {
        let input = r#"
readonly global dead: int32 = 1int32
function dead(): int32 {
b0:
    v0: int32 = 2int32
    return v0
}
export function root(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = int.add v0, v0
    return v0
}"#;

        let expected = r#"
external readonly global dead: int32

external function dead(): int32

export function root(): int32 {
entry0:
    value0: int32 = 1int32
    value1: int32 = int.add value0, value0
    return value0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralDceCleanup);
        test.assert_output(expected);
    }

    /// Live globals are preserved during cleanup.
    #[test]
    fn test_ip_dce_cleanup_preserves_live_globals() {
        let input = r#"
readonly global live: int32 = 1int32
readonly global dead: int32 = 2int32
export function root(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address live
    v1: int32 = load v0
    return v1
}"#;

        let expected = r#"
readonly global live: int32 = 1int32

external readonly global dead: int32

export function root(): int32 {
entry0:
    value0: ref<int32, raw, readonly> = global.address live
    value1: int32 = load value0
    return value1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralDceCleanup);
        test.assert_output(expected);
    }
}
