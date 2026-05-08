use crate::Compiler;

use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use super::ComptimeOutput;

/// Patch information for a single comptime slot.
#[derive(Debug, Clone)]
pub(crate) struct ComptimePatch {
    /// The expression to replace.
    pub expression_id: dir::LocalNodeIdAny,
    /// The computed result.
    pub result: Option<ComptimeOutput>,
}

impl Compiler {
    /// Apply comptime results by patching DIR.
    pub(crate) fn apply_comptime_patch(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        tree: &dir::Tree,
        types: &dir::TypeTable,
        dir_patch: &mut dir::Patch,
        comptime_patch: ComptimePatch,
    ) {
        let _ = (module_id, profile_id, tree, types, dir_patch, comptime_patch);

        todo!("FUGU #Incomplete: apply comptime output through DIR patch")
    }
}
