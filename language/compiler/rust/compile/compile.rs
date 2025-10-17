use std::collections::VecDeque;

use dyst_dir::{
    Annotation, Argument, Expression, Node, NodeId, NodeIdAny, NodeTreeStore, NodeType, Type,
};

use crate::{AstNodeIdAny, Compiler, CompilerMode};

/// Request to statically evaluate something to a value in-place.
#[derive(Debug, Clone)]
pub enum EvaluateRequest {
    /// Evaluate an Expression fully (in-place).
    EvaluateExpression { expression: NodeId<Expression> },
    /// Evaluate a Type to its Type value (in-place).
    EvaluateType { ty: NodeId<Type> },
    /// Evaluate an Argument (in-place).
    EvaluateArgument { argument: NodeId<Argument> },
    /// Evaluate an Annotation fully (in-place).
    EvaluateAnnotation { annotation: NodeId<Annotation> },
}

/// Error when evaluating something statically.
#[derive(Debug, Clone)]
pub enum EvaluateError {
    NotYetEvaluatable {
        node_id: NodeIdAny,
        depends_on: Option<NodeIdAny>,
    },
}

impl std::fmt::Display for EvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub type EvaluateResult<T> = Result<T, EvaluateError>;

/// Request to check something statically.
#[derive(Debug, Clone)]
pub enum CheckRequest {
    /// Typecheck a node.
    CheckType { node: NodeIdAny },
}

/// Request to lower something.
#[derive(Debug, Clone)]
pub enum LowerRequest {
    /// Lower a node to DIR.
    LowerToDir { node: AstNodeIdAny },
    /// Lower a node to MIR.
    LowerToMir { node: AstNodeIdAny },
}

/// Message from the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerMessage {
    /// Request to evaluate something statically.
    EvaluateRequest(EvaluateRequest),
    /// Request to check something statically.
    CheckRequest(CheckRequest),
    /// Request to lower something.
    LowerRequest(LowerRequest),
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

impl<'s> Compiler<'s> {
    /// Evaluate compile time constructs and check compile time invariants.
    /// Runs until there is nothing left to evaluate.
    pub fn compile(&mut self) {
        assert!(self.mode == CompilerMode::Parsed);
        self.mode = CompilerMode::Compiling;

        // process all unevaluated nodes
        self.queue_all_unevaluated_nodes();
        while let Some(message) = self.queue.pop_front() {
            self.process_message(message);
        }

        self.mode = CompilerMode::Compiled;
    }

    // Generate messages for all unevaluated nodes.
    fn queue_all_unevaluated_nodes(&mut self) {
        assert!(self.queue.is_empty());

        // expressions
        for (expression_id, expression) in self.tree.iter_nodes::<Expression>() {
            if !expression.is_evaluated_self() {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateExpression {
                        expression: expression_id,
                    },
                ));
            }
        }

        // types
        for (type_id, type_) in self.tree.iter_nodes::<Type>() {
            if !type_.is_evaluated_self() {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateType { ty: type_id },
                ));
            }
        }

        // arguments
        for (argument_id, argument) in self.tree.iter_nodes::<Argument>() {
            if !argument.is_evaluated_self() {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateArgument {
                        argument: argument_id,
                    },
                ));
            }
        }

        // annotations
        for (annotation_id, annotation) in self.tree.iter_nodes::<Annotation>() {
            if !annotation.is_evaluated_self() {
                self.queue.push_back(CompilerMessage::EvaluateRequest(
                    EvaluateRequest::EvaluateAnnotation {
                        annotation: annotation_id,
                    },
                ));
            }
        }
    }

    // Queue a node to be evaluated.
    fn queue_evaluate_node<T>(&mut self, node_id: NodeId<T>)
    where
        Self: NodeTreeStore<T>,
        T: Node,
    {
        match T::KIND {
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

    fn process_message(&mut self, message: CompilerMessage) {
        match message {
            CompilerMessage::EvaluateRequest(request) => {
                let result = match request {
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
