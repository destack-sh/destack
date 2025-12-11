use crate::{Compiler, LinkResult, Task, TaskDebug, TaskDependencyError, TaskOutput};

use destack_source::PackageId;
use destack_workspace::Program;

/// Task to link generated artifacts.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LinkTask {
    /// Link all modules for a target and emit output.
    LinkTarget {
        /// The package containing the target.
        package: PackageId,
        /// The target name.
        target: String,
    },
}

impl LinkTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::LinkTarget { .. } => 1,
        }
    }
}

impl TaskDebug for LinkTask {
    fn name(&self) -> &'static str {
        match self {
            Self::LinkTarget { .. } => "target",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::LinkTarget { package, target } => {
                let package = program.packages.get(*package);
                let uri = package.read().uri.clone().to_string();
                format!(r#"package="{uri}" target="{target}""#)
            }
        }
    }
}

impl From<LinkTask> for Task {
    fn from(task: LinkTask) -> Self {
        Task::Link(task)
    }
}

/// Output of a link task.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkOutput {}

impl From<LinkOutput> for TaskOutput {
    fn from(output: LinkOutput) -> Self {
        TaskOutput::Link(output)
    }
}

impl Compiler {
    /// Process a link task.
    pub fn process_link(&self, task: LinkTask) -> LinkResult<LinkOutput> {
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
