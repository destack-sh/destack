use crate::{Compiler, CompilerTask};

#[allow(dead_code)]
impl<'s> Compiler<'s> {
    /// Runs the compiler loop until there is nothing left to do.
    pub fn compile(&mut self) {
        // process all unresolved nodes
        self.queue_all_unresolved();
        while let Some(task) = self.queue.pop_front() {
            self.process(task);
        }
    }

    /// Queue a task to the compiler.
    pub(super) fn queue(&mut self, task: CompilerTask) {
        self.queue.push_back(task);
    }

    /// Generate tasks for all unresolved nodes.
    pub(super) fn queue_all_unresolved(&mut self) {
        // expressions, types, arguments, annotations, ...
    }

    /// Process a compiler task.
    #[inline]
    pub(super) fn process(&mut self, task: CompilerTask) {
        match task {
            CompilerTask::Import(task) => {
                self.process_import(task);
            }
            CompilerTask::Resolve(task) => {
                self.process_resolve(task);
            }
            CompilerTask::Validate(task) => {
                self.process_validate(task);
            }
            CompilerTask::Execute(task) => {
                self.process_execute(task);
            }
            CompilerTask::Optimize(task) => {
                self.process_optimize(task);
            }
            CompilerTask::Build(task) => {
                self.process_build(task);
            }
        }
    }
}
