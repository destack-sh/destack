use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::{Compiler, TaskDependencyError};

/// Stage contract for cross-module analyze table reads.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalyzeDependencyStage {
    /// Read data owned by declare.
    Declare,
    /// Read data owned by interface.
    Interface,
    /// Read data owned by infer.
    Infer,
    /// Read data owned by validate.
    Validate,
}

impl Compiler {
    /// Require one stage gate when a read targets a different module id.
    pub(crate) fn require_stage_for_remote_module_read(
        &self,
        local_module_id: ModuleId,
        target_module_id: ModuleId,
        profile: ProfileId,
        stage: AnalyzeDependencyStage,
    ) -> Result<(), TaskDependencyError> {
        // local reads do not need stage gating
        if local_module_id == target_module_id {
            return Ok(());
        }

        self.require_module_stage_for_read(target_module_id, profile, stage)
    }

    /// Ensure a module has completed the stage required for one cross-module read.
    pub(crate) fn require_module_stage_for_read(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        stage: AnalyzeDependencyStage,
    ) -> Result<(), TaskDependencyError> {
        // gate reads by stage ownership
        match stage {
            AnalyzeDependencyStage::Declare => {
                self.require_analyze_module_declare(module_id, profile)
            }
            AnalyzeDependencyStage::Interface => {
                self.require_analyze_module_interface(module_id, profile)
            }
            AnalyzeDependencyStage::Infer => self.require_analyze_module_infer(module_id, profile),
            AnalyzeDependencyStage::Validate => {
                self.require_analyze_module_validate(module_id, profile)
            }
        }
    }
}
