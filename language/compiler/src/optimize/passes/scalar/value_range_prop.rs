use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
use destack_mir::{Mutation, RangeAnalysis, instruction_is_pure};

declare_pass! {
    /// Fold values that range analysis proves constant.
    ///
    /// Range analysis can prove that some comparisons are always true or false.
    /// This pass replaces those values with constants to enable further
    /// simplification and guard elimination.
    ///
    /// ```mir
    /// function before(): boolean {
    /// b0:
    ///     v0 = 1int32
    ///     v1 = 2int32
    ///     v2 = int.lt.s v0, v1
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): boolean {
    /// b0:
    ///     v0 = 1int32
    ///     v1 = 2int32
    ///     v2 = true
    ///     return v2
    /// }
    /// ```
    #[pass(id = "value-range-prop")]
    pub ValueRangePropagation,
    "Fold values proven constant by range analysis"
}

impl FunctionPass for ValueRangePropagation {
    /// Run the value range propagation pass.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        // skip imported functions
        if function.entry.is_none() {
            return Mutation::NONE;
        }

        // gather range analysis
        let ranges = { analyses.get::<RangeAnalysis>(function, tree).clone() };

        // fold instructions with constant ranges
        let changed = run_value_range_propagation(function, tree, &ranges);

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "ValueRangePropagation"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "value-range-prop"
    }
}

/// Apply range-based constant folding to a function.
fn run_value_range_propagation(
    function: &mir::Function,
    tree: &mut mir::Tree,
    ranges: &RangeAnalysis,
) -> bool {
    // track whether any instruction was replaced
    let mut changed = false;

    // walk blocks and fold constant range results
    for &block_id in &function.blocks {
        // read block ranges and instructions
        let block = tree.get(block_id);
        let exit_ranges = ranges.exit(block_id);
        let instruction_ids: Vec<_> = block.instructions.clone();

        // scan instructions in order
        for instruction_id in instruction_ids {
            // read instruction and destination
            let instruction = tree.get(instruction_id);
            let Some(destination) = instruction.destination() else {
                continue;
            };

            // skip instructions that are not pure
            if !instruction_is_pure(instruction) {
                continue;
            }

            // fold when the destination range is a single constant
            let Some(range) = exit_ranges.get(destination) else {
                continue;
            };
            let Some(constant) = range.as_constant() else {
                continue;
            };

            // skip when the instruction already matches the constant
            if let mir::Instruction::Const { value, .. } = instruction
                && *value == constant
            {
                continue;
            }

            // replace the instruction with a constant
            let new_instruction = mir::Instruction::Const {
                destination,
                value: constant.clone(),
            };
            tree.set(instruction_id, new_instruction);
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Constant comparisons fold to constant booleans.
    #[test]
    fn test_value_range_prop_constant_comparison() {
        let input = r#"
function test(): boolean {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: boolean = int.lt.s v0, v1
    return v2
}
"#;
        let expected = r#"
function test(): boolean {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: boolean = true
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ValueRangePropagation);
        test.assert_output(expected);
    }

    /// Constant equality folds to true.
    #[test]
    fn test_value_range_prop_constant_equals() {
        let input = r#"
function test(): boolean {
entry:
    v0: int32 = 4
    v1: int32 = 4
    v2: boolean = int.eq v0, v1
    return v2
}
"#;
        let expected = r#"
function test(): boolean {
entry:
    v0: int32 = 4
    v1: int32 = 4
    v2: boolean = true
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ValueRangePropagation);
        test.assert_output(expected);
    }

    /// Non-constant comparisons are preserved.
    #[test]
    fn test_value_range_prop_preserves_non_constant() {
        let input = r#"
function test(v0: int32, v1: int32): boolean {
entry(v0: int32, v1: int32):
    v2: boolean = int.lt.s v0, v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ValueRangePropagation);
        test.assert_output(input);
    }
}
