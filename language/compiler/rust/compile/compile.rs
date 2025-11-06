use std::collections::VecDeque;

use dyst_dir::{
    Annotation, Argument, Expression, Node, NodeId, NodeIdAny, NodeTreeImpl, NodeType, Type,
};

use crate::{
    Compiler, CompilerStatus, EvaluateRequest, ExecuteRequest, LoadRequest, LowerRequest,
    ValidateRequest,
};

/// Message from the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerMessage {
    /// Request to load something.
    LoadRequest(LoadRequest),
    /// Request to evaluate something.
    EvaluateRequest(EvaluateRequest),
    /// Request to validate something.
    ValidateRequest(ValidateRequest),
    /// Execute something at compile time.
    ExecuteRequest(ExecuteRequest),
    /// Lower something to DIR/MIR.
    LowerRequest(LowerRequest),
}

/// A result of compiling something.
pub trait CompilerResult {
    /// The node that this result depends on.
    fn depends_on(&self) -> Option<NodeIdAny>;
}

/// Queue of compiler messages.
#[derive(Debug, Clone)]
pub struct CompilerQueue {
    messages: VecDeque<CompilerMessage>,
}

impl Default for CompilerQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerQueue {
    /// Create a new empty queue.
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
        }
    }

    /// Push a message to the back of the queue.
    pub fn push_back(&mut self, message: CompilerMessage) {
        self.messages.push_back(message);
    }

    /// Pop a message from the front of the queue.
    pub fn pop_front(&mut self) -> Option<CompilerMessage> {
        self.messages.pop_front()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[allow(dead_code)]
impl<'s> Compiler<'s> {
    /// Evaluate compile time constructs and check compile time invariants.
    /// Runs until there is nothing left to evaluate.
    pub fn compile(&mut self) {
        assert!(self.status == CompilerStatus::Parsed);
        self.status = CompilerStatus::Compiling;

        // process all unevaluated nodes
        self.queue_all_unevaluated_nodes();
        while let Some(message) = self.queue.pop_front() {
            self.process_message(message);
        }

        self.status = CompilerStatus::Compiled;
    }

    // Generate messages for all unevaluated nodes.
    fn queue_all_unevaluated_nodes(&mut self) {
        assert!(self.queue.is_empty());

        // expressions
        for (expression_id, expression) in self.tree.iter_nodes::<Expression>() {
            if !expression.is_evaluated() {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateExpression {
                        expression: expression_id,
                    },
                ));
            }
        }

        // types
        for (type_id, type_) in self.tree.iter_nodes::<Type>() {
            if !type_.is_evaluated() {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateType { ty: type_id },
                ));
            }
        }

        // arguments
        for (argument_id, argument) in self.tree.iter_nodes::<Argument>() {
            if !argument.is_evaluated() {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateArgument {
                        argument: argument_id,
                    },
                ));
            }
        }

        // annotations
        for (annotation_id, annotation) in self.tree.iter_nodes::<Annotation>() {
            if !annotation.is_evaluated() {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateAnnotation {
                        annotation: annotation_id,
                    },
                ));
            }
        }
    }

    // Queue a node to be evaluated.
    pub(crate) fn queue_evaluate_node<T>(&mut self, node_id: NodeId<T>)
    where
        Self: NodeTreeImpl<T>,
        T: Node,
    {
        match T::TYPE {
            NodeType::Expression => {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateExpression {
                        expression: NodeId::new(node_id.id),
                    },
                ));
            }
            NodeType::Type => {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateType {
                        ty: NodeId::new(node_id.id),
                    },
                ));
            }
            NodeType::Argument => {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateArgument {
                        argument: NodeId::new(node_id.id),
                    },
                ));
            }
            NodeType::Annotation => {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateAnnotation {
                        annotation: NodeId::new(node_id.id),
                    },
                ));
            }
            _ => panic!("unexpected node to evaluate: {node_id:?}"),
        }
    }

    /// Process a compiler message.
    fn process_message(&mut self, message: CompilerMessage) {
        match message {
            CompilerMessage::EvaluateRequest(request) => {
                let _result = match request {
                    EvaluateRequest::EvaluateExpression { expression } => {
                        self.evaluate_expression(expression)
                    }
                    EvaluateRequest::EvaluateType { ty } => self.evaluate_type(ty),
                    EvaluateRequest::EvaluateArgument { argument } => {
                        self.evaluate_argument(argument)
                    }
                    EvaluateRequest::EvaluateAnnotation { annotation } => {
                        self.evaluate_annotation(annotation)
                    }
                };
            }
            _ => todo!("process_message({message:?})"),
        }
    }
}
