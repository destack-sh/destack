use std::collections::HashMap;

use destack_mir as mir;

/// Common information collected from a function for alias analysis.
///
/// This includes constant values and instruction definitions that are
/// needed by multiple alias analysis implementations.
#[derive(Debug, Default)]
pub(super) struct FunctionAA {
    /// Map from value to constant integer value.
    pub constants: HashMap<mir::Value, i64>,
    /// Map from value to its defining instruction.
    pub definitions: HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    /// Function parameters.
    pub parameters: Vec<mir::Parameter>,
}

impl FunctionAA {
    /// Collect function information for alias analysis.
    ///
    /// Scans all instructions to build maps of constants and definitions.
    /// Returns a default (empty) result for imported functions without bodies.
    pub(super) fn collect(function: &mir::Function, tree: &mir::Tree) -> Self {
        // imports have no body
        if function.entry.is_none() {
            return Self::default();
        }

        let mut constants = HashMap::new();
        let mut definitions = HashMap::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let inst = tree.get(instruction_id);

                // record definitions
                if let Some(dest) = inst.destination().and_then(|value| value.value()) {
                    definitions.insert(dest, instruction_id);
                }

                // track integer constants
                if let mir::Instruction::Const { destination, value } = inst
                    && let mir::Constant::Int { value: v, .. } = value
                    && let Some(destination) = destination.value()
                {
                    constants.insert(destination, *v);
                }
            }
        }

        Self {
            constants,
            definitions,
            parameters: function.parameters.clone(),
        }
    }
}
