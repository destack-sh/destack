use crate::timing::tags;
use crate::{Compiler, LinkResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::{PackageId, PackageStamp};
use destack_workspace::TargetId;

/// Task to link generated artifacts.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Link)]
pub enum LinkTask {
    /// Link all modules for a target and emit output.
    #[task(code = 1, trace = "package={package} target={target}")]
    LinkTarget {
        /// The package containing the target.
        package: PackageStamp,
        /// The target id.
        target: TargetId,
    },
}

impl Compiler {
    /// Process a link task.
    pub fn process_link(&self, task: LinkTask) -> LinkResult<()> {
        match task {
            LinkTask::LinkTarget { package, target } => {
                if !self.package_version_matches(package.id, package.version) {
                    return Ok(());
                }
                let _timing = self.timing_scope(tags::LINK_TARGET);
                self.link_target(package.id, &target)
            }
        }
    }

    /// Ensure a target has been linked.
    pub fn require_link_module(
        &self,
        package: PackageId,
        target: &TargetId,
    ) -> Result<(), TaskDependencyError> {
        let package = self.package_stamp(package);
        self.do_require_task_internal_only(LinkTask::LinkTarget {
            package,
            target: target.clone(),
        })
    }
}
