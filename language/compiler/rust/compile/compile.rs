use std::collections::VecDeque;

use dyst_dir::{
    Annotation, Argument, Expression, Node, NodeId, NodeIdAny, NodeTreeStore, NodeType, Type,
};

use crate::{
    Compiler, CompilerMode, ExecuteRequest, LowerRequest, ResolveRequest, ValidateRequest,
};

/// Message from the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerMessage {
    /// Request to resolve something statically.
    ResolveRequest(ResolveRequest),
    /// Request to validate something statically.
    ValidateRequest(ValidateRequest),
    /// Execute something at compile time.
    ExecuteRequest(ExecuteRequest),
    /// Lower something to DIR/MIR.
    LowerRequest(LowerRequest),
}

/// A result of compiling something.
pub trait CompileResult {
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

impl<'s> Compiler<'s> {
    /// Resolve compile time constructs and check compile time invariants.
    /// Runs until there is nothing left to resolve.
    pub fn compile(&mut self) {
        assert!(self.mode == CompilerMode::Parsed);
        self.mode = CompilerMode::Compiling;

        // process all unresolved nodes
        self.queue_all_unresolved_nodes();
        while let Some(message) = self.queue.pop_front() {
            self.process_message(message);
        }

        self.mode = CompilerMode::Compiled;
    }

    // Generate messages for all unresolved nodes.
    fn queue_all_unresolved_nodes(&mut self) {
        assert!(self.queue.is_empty());

        // expressions
        for (expression_id, expression) in self.tree.iter_nodes::<Expression>() {
            if !expression.is_resolved() {
                self.queue.push_back(CompilerMessage::ResolveRequest(
                    ResolveRequest::ResolveExpression {
                        expression: expression_id,
                    },
                ));
            }
        }

        // types
        for (type_id, type_) in self.tree.iter_nodes::<Type>() {
            if !type_.is_resolved() {
                self.queue.push_back(CompilerMessage::ResolveRequest(
                    ResolveRequest::ResolveType { ty: type_id },
                ));
            }
        }

        // arguments
        for (argument_id, argument) in self.tree.iter_nodes::<Argument>() {
            if !argument.is_resolved() {
                self.queue.push_back(CompilerMessage::ResolveRequest(
                    ResolveRequest::ResolveArgument {
                        argument: argument_id,
                    },
                ));
            }
        }

        // annotations
        for (annotation_id, annotation) in self.tree.iter_nodes::<Annotation>() {
            if !annotation.is_resolved() {
                self.queue.push_back(CompilerMessage::ResolveRequest(
                    ResolveRequest::ResolveAnnotation {
                        annotation: annotation_id,
                    },
                ));
            }
        }
    }

    // Queue a node to be resolved.
    pub(crate) fn queue_resolve_node<T>(&mut self, node_id: NodeId<T>)
    where
        Self: NodeTreeStore<T>,
        T: Node,
    {
        match T::KIND {
            NodeType::Expression => {
                self.queue.push_back(CompilerMessage::ResolveRequest(
                    ResolveRequest::ResolveExpression {
                        expression: NodeId::new(node_id.id),
                    },
                ));
            }
            NodeType::Type => {
                self.queue.push_back(CompilerMessage::ResolveRequest(
                    ResolveRequest::ResolveType {
                        ty: NodeId::new(node_id.id),
                    },
                ));
            }
            NodeType::Argument => {
                self.queue.push_back(CompilerMessage::ResolveRequest(
                    ResolveRequest::ResolveArgument {
                        argument: NodeId::new(node_id.id),
                    },
                ));
            }
            NodeType::Annotation => {
                self.queue.push_back(CompilerMessage::ResolveRequest(
                    ResolveRequest::ResolveAnnotation {
                        annotation: NodeId::new(node_id.id),
                    },
                ));
            }
            _ => panic!("unexpected node to resolve: {node_id:?}"),
        }
    }

    /// Process a compiler message.
    fn process_message(&mut self, message: CompilerMessage) {
        match message {
            CompilerMessage::ResolveRequest(request) => {
                let result = match request {
                    ResolveRequest::ResolveExpression { expression } => {
                        self.resolve_expression(expression)
                    }
                    ResolveRequest::ResolveType { ty } => self.resolve_type(ty),
                    ResolveRequest::ResolveArgument { argument } => self.resolve_argument(argument),
                    ResolveRequest::ResolveAnnotation { annotation } => {
                        self.resolve_annotation(annotation)
                    }
                };
            }
            _ => todo!("process_message({message:?})"),
        }
    }
}
