use crate::{Compiler, CompilerTask};

#[allow(dead_code)]
impl<'s> Compiler<'s> {
    /// Runs the compiler loop until there is nothing left to do.
    pub fn compile(&self) {
        // process all tasks
        while let Some(task) = self.queue.pop_front() {
            self.process(task);
        }
    }

    /// Enqueue a task to the compiler.
    pub fn enqueue(&self, task: CompilerTask) {
        self.queue.push_back(task);
    }

    /// Process a compiler task.
    #[inline]
    pub(super) fn process(&self, task: CompilerTask) {
        match task {
            CompilerTask::Import(import_task) => {
                let mut tree = self.session.tree.write();
                if let Err(error) = self.process_import(import_task, &mut tree) {
                    self.error(error);
                }
            }
            CompilerTask::Resolve(resolve_task) => {
                let mut tree = self.session.tree.write();
                if let Err(error) = self.process_resolve(resolve_task, &mut tree) {
                    self.error(error);
                }
            }
            CompilerTask::Validate(validate_task) => {
                if let Err(error) = self.process_validate(validate_task) {
                    self.error(error);
                }
            }
            CompilerTask::Lower(lower_task) => {
                if let Err(error) = self.process_lower(lower_task) {
                    self.error(error);
                }
            }
            CompilerTask::Execute(execute_task) => {
                if let Err(error) = self.process_execute(execute_task) {
                    self.error(error);
                }
            }
            CompilerTask::Optimize(optimize_task) => {
                if let Err(error) = self.process_optimize(optimize_task) {
                    self.error(error);
                }
            }
            CompilerTask::Build(build_task) => {
                if let Err(error) = self.process_build(build_task) {
                    self.error(error);
                }
            }
            CompilerTask::Link(link_task) => {
                if let Err(error) = self.process_link(link_task) {
                    self.error(error);
                }
            }
        }
    }
}
