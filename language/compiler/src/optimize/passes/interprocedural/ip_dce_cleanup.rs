use crate::declare_pass;
use destack_mir as mir;

use crate::optimize::passes::interprocedural::{
    run_dead_function_eliminate, run_global_dead_code_eliminate,
};
use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

declare_pass! {
    /// Run interprocedural cleanup after cross function optimizations.
    ///
    /// This pass prunes dead functions and globals.
    ///
    /// ```mir
    /// global dead: int32, readonly = 1int32
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
    /// extern global dead: int32, readonly
    /// extern function dead(): int32
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
    fn run(&self, tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        // run the cleanup pass
        let changed = run_interprocedural_dce_cleanup(tree);

        // report analysis preservation based on whether changes occurred
        if changed {
            ctx.strings.intern("ip-dce-cleanup");
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
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
fn run_interprocedural_dce_cleanup(tree: &mut mir::Tree) -> bool {
    let mut changed = false;

    if run_dead_function_eliminate(tree) {
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
global dead: int32, readonly = 1int32
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
extern global dead: int32, readonly
extern function dead(): int32
export function root(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = int.add v0, v0
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralDceCleanup);
        test.assert_output(expected);
    }

    /// Live globals are preserved during cleanup.
    #[test]
    fn test_ip_dce_cleanup_preserves_live_globals() {
        let input = r#"
global live: int32, readonly = 1int32
global dead: int32, readonly = 2int32
export function root(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address live
    v1: int32 = load v0
    return v1
}"#;

        let expected = r#"
global live: int32, readonly = 1int32
extern global dead: int32, readonly
export function root(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address live
    v1: int32 = load v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralDceCleanup);
        test.assert_output(expected);
    }
}
