#![allow(unused_variables)]

use crate::{
    Block, Field, Function, Global, Instruction, Local, LocalNodeId, NodeType, Terminator, Tree,
    Type, TypeAlias, walk_block, walk_field, walk_function, walk_global, walk_instruction,
    walk_local, walk_terminator, walk_type, walk_type_alias,
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
    fn visit_any(&mut self, tree: &Tree, ty: NodeType, id: u32) {
        // nothing by default
    }

    /// Visit a Function.
    fn visit_function(&mut self, tree: &Tree, id: LocalNodeId<Function>, function: &Function) {
        walk_function(self, tree, id, function);
    }

    /// Visit a Block.
    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    /// Visit an Instruction.
    fn visit_instruction(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        walk_instruction(self, tree, id, instruction);
    }

    /// Visit a Terminator.
    fn visit_terminator(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Terminator>,
        terminator: &Terminator,
    ) {
        walk_terminator(self, tree, id, terminator);
    }

    /// Visit a Local.
    fn visit_local(&mut self, tree: &Tree, id: LocalNodeId<Local>, local: &Local) {
        walk_local(self, tree, id, local);
    }

    /// Visit a Type.
    fn visit_type(&mut self, tree: &Tree, id: LocalNodeId<Type>, ty: &Type) {
        walk_type(self, tree, id, ty);
    }

    /// Visit a TypeAlias.
    fn visit_type_alias(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeAlias>,
        type_alias: &TypeAlias,
    ) {
        walk_type_alias(self, tree, id, type_alias);
    }

    /// Visit a Field.
    fn visit_field(&mut self, tree: &Tree, id: LocalNodeId<Field>, field: &Field) {
        walk_field(self, tree, id, field);
    }

    /// Visit a Global.
    fn visit_global(&mut self, tree: &Tree, id: LocalNodeId<Global>, global: &Global) {
        walk_global(self, tree, id, global);
    }
}
