use dyst_dir::{
    Annotation, Argument, Expression, Node, NodeId, NodeTreeImpl, NodeType, Type,
};

use crate::{Compiler, CompilerTask, EvaluateTask};

#[allow(dead_code)]
impl<'s> Compiler<'s> {
    /// Runs the compiler loop until there is nothing left to do.
    pub fn compile(&mut self) {
        // process all unevaluated nodes
        self.queue_all_unevaluated();
        while let Some(task) = self.queue.pop_front() {
            self.process(task);
        }
    }

    /// Queue a task to the compiler.
    pub(super) fn queue(&mut self, task: CompilerTask) {
        self.queue.push_back(task);
    }

    /// Generate tasks for all unevaluated nodes.
    pub(super) fn queue_all_unevaluated(&mut self) {
        // expressions
        for (expression_id, expression) in self.tree.iter_nodes::<Expression>() {
            if !expression.is_evaluated() {
                self.queue
                    .push_back(CompilerTask::Evaluate(EvaluateTask::EvaluateExpression {
                        expression: expression_id,
                    }));
            }
        }

        // types
        for (type_id, type_) in self.tree.iter_nodes::<Type>() {
            if !type_.is_evaluated() {
                self.queue
                    .push_back(CompilerTask::Evaluate(EvaluateTask::EvaluateType {
                        ty: type_id,
                    }));
            }
        }

        // arguments
        for (argument_id, argument) in self.tree.iter_nodes::<Argument>() {
            if !argument.is_evaluated() {
                self.queue
                    .push_back(CompilerTask::Evaluate(EvaluateTask::EvaluateArgument {
                        argument: argument_id,
                    }));
            }
        }

        // annotations
        for (annotation_id, annotation) in self.tree.iter_nodes::<Annotation>() {
            if !annotation.is_evaluated() {
                self.queue
                    .push_back(CompilerTask::Evaluate(EvaluateTask::EvaluateAnnotation {
                        annotation: annotation_id,
                    }));
            }
        }
    }

    /// Queue a node to be evaluated as needed.
    pub(crate) fn queue_evaluate<T>(&mut self, node_id: NodeId<T>)
    where
        Self: NodeTreeImpl<T>,
        T: Node,
    {
        match T::TYPE {
            NodeType::Expression => {
                self.queue
                    .push_back(CompilerTask::Evaluate(EvaluateTask::EvaluateExpression {
                        expression: NodeId::new(node_id.id),
                    }));
            }
            NodeType::Type => {
                self.queue
                    .push_back(CompilerTask::Evaluate(EvaluateTask::EvaluateType {
                        ty: NodeId::new(node_id.id),
                    }));
            }
            NodeType::Argument => {
                self.queue
                    .push_back(CompilerTask::Evaluate(EvaluateTask::EvaluateArgument {
                        argument: NodeId::new(node_id.id),
                    }));
            }
            NodeType::Annotation => {
                self.queue
                    .push_back(CompilerTask::Evaluate(EvaluateTask::EvaluateAnnotation {
                        annotation: NodeId::new(node_id.id),
                    }));
            }
            _ => {
                // nothing to do
            }
        }
    }

    /// Process a compiler task.
    #[inline]
    pub(super) fn process(&mut self, task: CompilerTask) {
        match task {
            CompilerTask::Load(task) => self.process_load(task),
            CompilerTask::Evaluate(task) => self.process_evaluate(task),
            CompilerTask::Validate(task) => self.process_validate(task),
            CompilerTask::Execute(task) => self.process_execute(task),
            CompilerTask::Optimize(task) => self.process_optimize(task),
            CompilerTask::Build(task) => self.process_build(task),
        }
    }
}
