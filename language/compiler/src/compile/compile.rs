use crate::{CompileTask, Compiler};

#[allow(dead_code)]
impl<'s> Compiler<'s> {
    /// Runs the compiler loop until there is nothing left to do.
    pub fn compile(&self) {
        // process all tasks
        while let Some(task) = self.queue.pop_front() {
            self.process(task);
        }
        // flush remaining diagnostics
        self.flush_diagnostics();
    }

    /// Enqueue a task to the compiler.
    pub fn enqueue(&self, task: CompileTask) {
        self.queue.push_back(task);
    }

    /// Process a compiler task.
    #[inline]
    pub(super) fn process(&self, task: CompileTask) {
        match task {
            CompileTask::Import(import_task) => {
                if let Err(error) = self.process_import(import_task) {
                    self.error(error);
                }
            }
            CompileTask::Bind(bind_task) => {
                if let Err(error) = self.process_bind(bind_task) {
                    self.error(error);
                }
            }
            CompileTask::Resolve(resolve_task) => {
                if let Err(error) = self.process_resolve(resolve_task) {
                    self.error(error);
                }
            }
            CompileTask::Validate(validate_task) => {
                if let Err(error) = self.process_validate(validate_task) {
                    self.error(error);
                }
            }
            CompileTask::Elaborate(elaborate_task) => {
                if let Err(error) = self.process_elaborate(elaborate_task) {
                    self.error(error);
                }
            }
            CompileTask::Lower(lower_task) => {
                if let Err(error) = self.process_lower(lower_task) {
                    self.error(error);
                }
            }
            CompileTask::Execute(execute_task) => {
                if let Err(error) = self.process_execute(execute_task) {
                    self.error(error);
                }
            }
            CompileTask::Optimize(optimize_task) => {
                if let Err(error) = self.process_optimize(optimize_task) {
                    self.error(error);
                }
            }
            CompileTask::Build(build_task) => {
                if let Err(error) = self.process_build(build_task) {
                    self.error(error);
                }
            }
            CompileTask::Link(link_task) => {
                if let Err(error) = self.process_link(link_task) {
                    self.error(error);
                }
            }
        }
    }
}
