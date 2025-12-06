//! MIR formatter tests.

use destack_source::{ImmutableStringPool, StringId, StringPool};

use crate::{
    BinaryOperator, Block, Constant, Function, Instruction, Local, LocalNodeId, MirFormatOptions,
    Mutability, NodeTree, Ownership, Terminator, Type, TypedValue, Value, format_mir,
};

/// Helper to build MIR for tests.
struct TreeBuilder {
    tree: NodeTree,
    strings: StringPool,
}

impl TreeBuilder {
    fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            strings: StringPool::new(),
        }
    }

    /// Intern a string and return its ID.
    fn intern(&self, s: &str) -> StringId {
        self.strings.intern(s)
    }

    /// Insert a type node.
    fn ty(&mut self, ty: Type) -> LocalNodeId<Type> {
        self.tree.insert(ty)
    }

    /// Insert a local node.
    fn local(
        &mut self,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
        ownership: Ownership,
    ) -> LocalNodeId<Local> {
        self.tree.insert(Local::new(ty, mutability, ownership))
    }

    /// Insert a block node.
    fn block(&mut self, block: Block) -> LocalNodeId<Block> {
        self.tree.insert(block)
    }

    /// Insert an instruction node.
    fn instruction(&mut self, inst: Instruction) -> LocalNodeId<Instruction> {
        self.tree.insert(inst)
    }

    /// Insert a function node.
    fn function(&mut self, func: Function) -> LocalNodeId<Function> {
        self.tree.insert(func)
    }

    /// Finish building the tree.
    fn finish(self) -> (NodeTree, ImmutableStringPool) {
        (self.tree, self.strings.into_immutable())
    }
}

/// Build and format a simple add function.
#[test]
fn test_format_simple_add() {
    let mut b = TreeBuilder::new();

    // types
    let i32_ty = b.ty(Type::INT32);
    let void_ty = b.ty(Type::Void);

    // block with add instruction
    let v0 = Value::new(0);
    let v1 = Value::new(1);
    let v2 = Value::new(2);

    let add_inst = b.instruction(Instruction::Binary {
        destination: v2,
        operator: BinaryOperator::Add,
        left: v0,
        right: v1,
    });

    let entry = b.block(Block {
        parameters: vec![TypedValue::new(v0, i32_ty), TypedValue::new(v1, i32_ty)],
        instructions: vec![add_inst],
        terminator: Terminator::Return { value: Some(v2) },
    });

    // function
    let name = b.intern("add");
    let _function = b.function(Function {
        name,
        parameters: vec![TypedValue::new(v0, i32_ty), TypedValue::new(v1, i32_ty)],
        return_type: void_ty,
        locals: vec![],
        blocks: vec![entry],
        entry,
        next_value_id: 3,
    });

    let (tree, strings) = b.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());

    let expected = "\
function @add(v0: i32, v1: i32) -> void {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}";

    assert_eq!(output, expected);
}

/// Build and format a function with local variables.
#[test]
fn test_format_with_locals() {
    let mut b = TreeBuilder::new();

    // types
    let i64_ty = b.ty(Type::INT64);

    // local variable
    let local = b.local(i64_ty, Mutability::Mutable, Ownership::Owned);

    // instructions
    let v0 = Value::new(0);
    let v1 = Value::new(1);

    let const_inst = b.instruction(Instruction::Constant {
        destination: v0,
        value: Constant::int64(42),
    });

    let store_inst = b.instruction(Instruction::LocalSet { local, value: v0 });

    let load_inst = b.instruction(Instruction::LocalGet {
        destination: v1,
        local,
    });

    let entry = b.block(Block {
        parameters: vec![],
        instructions: vec![const_inst, store_inst, load_inst],
        terminator: Terminator::Return { value: Some(v1) },
    });

    // function
    let name = b.intern("with_locals");
    let _function = b.function(Function {
        name,
        parameters: vec![],
        return_type: i64_ty,
        locals: vec![local],
        blocks: vec![entry],
        entry,
        next_value_id: 2,
    });

    let (tree, strings) = b.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());

    let expected = "\
function @with_locals() -> i64 {
    local0: i64 ; owned, var
block0:
    v0 = iconst 42i64
    store_local local0, v0
    v1 = load_local local0
    return v1
}";

    assert_eq!(output, expected);
}

/// Build and format a function with a branch instruction.
#[test]
fn test_format_branch() {
    let mut b = TreeBuilder::new();

    // types
    let bool_ty = b.ty(Type::Boolean);
    let i32_ty = b.ty(Type::INT32);

    // values
    let v0 = Value::new(0); // condition
    let v1 = Value::new(1); // then result
    let v2 = Value::new(2); // else result
    let v3 = Value::new(3); // merged result

    // merge block (created first to get its ID for forward references)
    let merge_block = b.block(Block {
        parameters: vec![TypedValue::new(v3, i32_ty)],
        instructions: vec![],
        terminator: Terminator::Return { value: Some(v3) },
    });

    // then block
    let then_const = b.instruction(Instruction::Constant {
        destination: v1,
        value: Constant::int32(1),
    });
    let then_block = b.block(Block {
        parameters: vec![],
        instructions: vec![then_const],
        terminator: Terminator::Jump {
            target: merge_block,
            arguments: vec![v1],
        },
    });

    // else block
    let else_const = b.instruction(Instruction::Constant {
        destination: v2,
        value: Constant::int32(0),
    });
    let else_block = b.block(Block {
        parameters: vec![],
        instructions: vec![else_const],
        terminator: Terminator::Jump {
            target: merge_block,
            arguments: vec![v2],
        },
    });

    // entry block
    let entry = b.block(Block {
        parameters: vec![TypedValue::new(v0, bool_ty)],
        instructions: vec![],
        terminator: Terminator::Branch {
            condition: v0,
            then_target: then_block,
            then_arguments: vec![],
            else_target: else_block,
            else_arguments: vec![],
        },
    });

    // function (blocks listed in order they appear in output)
    let name = b.intern("select");
    let _function = b.function(Function {
        name,
        parameters: vec![TypedValue::new(v0, bool_ty)],
        return_type: i32_ty,
        locals: vec![],
        blocks: vec![entry, then_block, else_block, merge_block],
        entry,
        next_value_id: 4,
    });

    let (tree, strings) = b.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());

    // blocks are numbered by their position in function.blocks:
    // entry=0, then=1, else=2, merge=3
    let expected = "\
function @select(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    jump block3(v1)
block2:
    v2 = iconst 0i32
    jump block3(v2)
block3(v3: i32):
    return v3
}";

    assert_eq!(output, expected);
}

/// Build and format a function with a void return.
#[test]
fn test_format_void_return() {
    let mut b = TreeBuilder::new();

    let void_ty = b.ty(Type::Void);

    let entry = b.block(Block {
        parameters: vec![],
        instructions: vec![],
        terminator: Terminator::Return { value: None },
    });

    let name = b.intern("noop");
    let _function = b.function(Function {
        name,
        parameters: vec![],
        return_type: void_ty,
        locals: vec![],
        blocks: vec![entry],
        entry,
        next_value_id: 0,
    });

    let (tree, strings) = b.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());

    let expected = "\
function @noop() -> void {
block0:
    return
}";

    assert_eq!(output, expected);
}
