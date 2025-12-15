use crate::{Compiler, LinkResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::PackageId;

/// Task to link generated artifacts.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Link)]
pub enum LinkTask {
    /// Link all modules for a target and emit output.
    #[task(code = 1, trace = "package={package} target={target}")]
    LinkTarget {
        /// The package containing the target.
        package: PackageId,
        /// The target name.
        target: String,
    },
}

impl Compiler {
    /// Process a link task.
    pub fn process_link(&self, task: LinkTask) -> LinkResult<()> {
        match task {
            LinkTask::LinkTarget { package, target } => self.link_target(package, &target),
        }
    }

    /// Ensure a target has been linked.
    pub fn require_link_module(
        &self,
        package: PackageId,
        target: &str,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LinkTask::LinkTarget {
            package,
            target: target.to_string(),
        })
    }
}
