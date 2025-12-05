//! MIR node visitor trait.

#![allow(unused_variables)]

use crate::{
    Block, Function, Instruction, Local, LocalNodeId, NodeTree, NodeType, Type,
    walk_block, walk_function, walk_instruction, walk_local, walk_type,
};

/// Options for the NodeVisitor.
#[derive(Debug, Clone, Default)]
pub struct NodeVisitorOptions {}

/// A visitor for traversing MIR nodes.
pub trait NodeVisitor {
    /// Get the options for the visitor.
    fn options(&self) -> &NodeVisitorOptions;

    /// Visit any node (called before the specific visit method).
    #[inline]
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {
        // nothing by default
    }

    /// Visit a Function.
    fn visit_function(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Function>,
        function: &Function,
    ) {
        walk_function(self, tree, id, function);
    }

    /// Visit a Block.
    fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    /// Visit an Instruction.
    fn visit_instruction(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        walk_instruction(self, tree, id, instruction);
    }

    /// Visit a Local.
    fn visit_local(&mut self, tree: &NodeTree, id: LocalNodeId<Local>, local: &Local) {
        walk_local(self, tree, id, local);
    }

    /// Visit a Type.
    fn visit_type(&mut self, tree: &NodeTree, id: LocalNodeId<Type>, ty: &Type) {
        walk_type(self, tree, id, ty);
    }
}

