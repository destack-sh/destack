use tspp_artifact::{MirElaborated, MirOptimized};
use tspp_mir::ModuleCache;

use crate::CompilerResult;
use crate::optimize::passes::{InsertSafepoints, InsertWriteBarriers};

use super::Step;

/// Required runtime transformations, in execution order.
const STEPS: &[Step<'_>] = &[Step::Functions(&[&InsertSafepoints, &InsertWriteBarriers])];

/// Optimize elaborated MIR and insert the required runtime operations.
pub(crate) fn optimize(elaborated: &MirElaborated) -> CompilerResult<MirOptimized> {
    // copy the elaborated module for transformation
    let mut optimized = MirOptimized {
        tree: elaborated.tree.clone(),
        target: elaborated.target,
        initializer: elaborated.initializer,
        layouts: elaborated.layouts.clone(),
        dispatch: elaborated.dispatch.clone(),
        drops: elaborated.drops.clone(),
        effects: elaborated.effects.clone(),
        profile: elaborated.profile.clone(),
    };

    // execute module transformations and function groups
    let target_layout = optimized.target;
    let mut analyses = ModuleCache::with_target_layout(target_layout);
    for step in STEPS {
        step.run(&mut optimized, &mut analyses)?;
    }

    Ok(optimized)
}
