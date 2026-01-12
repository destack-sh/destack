use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::common::function_thread_jumps;
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Thread jumps through empty blocks.
    ///
    /// Collapses jump chains by bypassing empty passthrough blocks and
    /// replacing jumps to empty return blocks with direct returns.
    ///
    /// ```mir
    /// function @before() -> void {
    /// block0:
    ///     jump block1
    /// block1:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after() -> void {
    /// block0:
    ///     return
    /// block1:
    ///     return
    /// }
    /// ```
    #[pass(id = "jump-threading")]
    pub JumpThreading,
    "Thread jumps through empty blocks"
}

impl FunctionPass for JumpThreading {
    /// Run jump threading on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // thread jumps through empty or passthrough blocks
        let changed = function_thread_jumps(function, tree);

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "JumpThreading"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "jump-threading"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Jumps to empty return blocks are threaded.
    #[test]
    fn test_jump_threading_return_block() {
        let input = r#"function @test() -> void {
block0:
    jump block1
block1:
    return
}"#;
        let expected = r#"function @test() -> void {
block0:
    return
block1:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&JumpThreading);
        program.assert_output(expected);
    }

    /// Passthrough blocks forward their arguments directly.
    #[test]
    fn test_jump_threading_passthrough_block() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block1(v0)
block1(v1: i32):
    jump block2(v1)
block2(v2: i32):
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block2(v0)
block1(v1: i32):
    jump block2(v1)
block2(v2: i32):
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&JumpThreading);
        program.assert_output(expected);
    }

    /// Empty jump targets fold into direct returns.
    #[test]
    fn test_jump_threading_branch_targets() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block4
block3:
    return
block4:
    return
}"#;
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    return
block2:
    return
block3:
    return
block4:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&JumpThreading);
        program.assert_output(expected);
    }

    /// Empty check targets fold into direct returns.
    #[test]
    fn test_jump_threading_check_targets() {
        let input = r#"function @test(v0: bool, v1: u32, v2: u32, v3: [u32; 4]) -> void {
block0(v0: bool, v1: u32, v2: u32, v3: [u32; 4]):
    check v0, bounds.unsigned v1, v2, v3, block1, block2
block1:
    jump block3
block2:
    jump block4
block3:
    return
block4:
    return
}"#;
        let expected = r#"function @test(v0: bool, v1: u32, v2: u32, v3: [u32; 4]) -> void {
block0(v0: bool, v1: u32, v2: u32, v3: [u32; 4]):
    check v0, bounds.unsigned v1, v2, v3, block1, block2
block1:
    return
block2:
    return
block3:
    return
block4:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&JumpThreading);
        program.assert_output(expected);
    }

    /// Non passthrough blocks are not threaded.
    #[test]
    fn test_jump_threading_preserves_non_passthrough() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block1(v0)
block1(v1: i32):
    jump block2(v0)
block2(v2: i32):
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&JumpThreading);
        program.assert_output(input);
    }
}
