use crate::optimize::declare_pass;
use destack_artifact::ProgramAnalysis;
use destack_mir as mir;

use crate::optimize::passes::interprocedural::{
    run_eliminate_dead_functions, run_eliminate_global_dead_code,
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
    #[pass(id = "eliminate-interprocedural-dead-code")]
    pub EliminateInterproceduralDeadCode,
    "Clean up dead code after interprocedural passes"
}

impl ModulePass for EliminateInterproceduralDeadCode {
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
            ctx.strings.intern("eliminate-interprocedural-dead-code");
            Mutation::CONTROL
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "EliminateInterproceduralDeadCode"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "eliminate-interprocedural-dead-code"
    }
}

/// Run interprocedural cleanup over the module.
fn run_interprocedural_dce_cleanup(tree: &mut mir::Tree, program: &ProgramAnalysis) -> bool {
    let mut changed = false;

    if run_eliminate_dead_functions(tree, program) {
        changed = true;
    }

    if run_eliminate_global_dead_code(tree) {
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
    fn test_eliminate_interprocedural_dead_code_removes_dead_items() {
        let input = r#"
readonly global dead: int32 = 1

function dead(): int32 {
entry:
    v0: int32 = 2
    return v0
}

export function root(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = int.add v0, v0
    return v0
}
"#;

        let expected = r#"
external readonly global dead: int32

external function dead(): int32

export function root(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = int.add v0, v0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateInterproceduralDeadCode);
        test.assert_output(expected);
    }

    /// Live globals are preserved during cleanup.
    #[test]
    fn test_eliminate_interprocedural_dead_code_preserves_live_globals() {
        let input = r#"
readonly global live: int32 = 1

readonly global dead: int32 = 2

export function root(): int32 {
entry:
    v0: ref<int32, raw, readonly> = global.address live
    v1: int32 = load v0
    return v1
}
"#;

        let expected = r#"
readonly global live: int32 = 1

external readonly global dead: int32

export function root(): int32 {
entry:
    v0: ref<int32, raw, readonly> = global.address live
    v1: int32 = load v0
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateInterproceduralDeadCode);
        test.assert_output(expected);
    }
}
