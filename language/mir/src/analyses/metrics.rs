use crate as mir;

/// MIR size metrics for a module.
#[derive(Debug, Clone, Copy, Default)]
pub struct MirStats {
    /// Number of functions.
    pub functions: usize,
    /// Total number of instructions across all functions.
    pub instructions: usize,
    /// Total number of blocks across all functions.
    pub blocks: usize,
}

impl MirStats {
    /// Count MIR size metrics for all functions in a tree.
    pub fn count(tree: &mir::Tree) -> Self {
        let mut metrics = Self::default();

        for (_, function) in tree.iter_nodes::<mir::Function>() {
            // skip imported functions with no body
            if function.entry.is_none() {
                continue;
            }

            metrics.functions += 1;
            metrics.blocks += function.blocks.len();

            for &block_id in &function.blocks {
                let block = tree.get(block_id);
                metrics.instructions += block.instructions.len();
            }
        }

        metrics
    }
}
