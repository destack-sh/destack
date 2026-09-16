#![allow(unused_variables)]

use crate::{
    Block, Field, FieldId, Function, Global, Instruction, Local, LocalNodeId, NodeType, Terminator,
    Tree, Type, TypeDeclaration, TypeId, walk_block, walk_field, walk_function, walk_global,
    walk_instruction, walk_local, walk_terminator, walk_type, walk_type_declaration,
};

/// A visitor for traversing MIR nodes.
pub trait NodeVisitor {
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
    fn visit_type(&mut self, tree: &Tree, id: TypeId, ty: &Type) {
        walk_type(self, tree, ty);
    }

    /// Visit a TypeDeclaration.
    fn visit_type_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeDeclaration>,
        type_declaration: &TypeDeclaration,
    ) {
        walk_type_declaration(self, tree, id, type_declaration);
    }

    /// Visit a Field.
    fn visit_field(&mut self, tree: &Tree, id: FieldId, field: &Field) {
        walk_field(self, tree, field);
    }

    /// Visit a Global.
    fn visit_global(&mut self, tree: &Tree, id: LocalNodeId<Global>, global: &Global) {
        walk_global(self, tree, id, global);
    }
}
