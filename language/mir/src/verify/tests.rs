use destack_base::StringPool;
use destack_source::FileId;

use crate::parse::Parser;
use crate::{
    ArgumentSlice, Block, CallArgumentMetadata, CallEffects, Function, Instruction, Local,
    Mutability, Ownership, Type, Value,
};

use super::{Verifier, VerifierOptions};

/// Local references must resolve to locals declared on the function.
#[test]
fn test_verify_rejects_local_not_in_function() {
    let mut tree = crate::NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("test");

    let ty = tree.insert(Type::Void);
    let local_ty = tree.insert(Type::Int {
        width: 32,
        signed: true,
    });
    let local = tree.insert(Local::new(
        local_ty,
        Mutability::Immutable,
        Ownership::Owned,
    ));

    let instruction = tree.insert(Instruction::LocalGet {
        destination: Value::new(0),
        local,
    });
    let block = Block {
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: crate::Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::new(name, Vec::new(), ty, block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let verifier = Verifier::new_with_options(&tree, VerifierOptions::basic());
    let error = verifier
        .verify_function(function_id)
        .expect_err("expected verification failure");
    let expected = format!("local reference local{} not defined in function", local.id);
    assert_eq!(error.to_string(), expected);
}

/// Reject duplicate value definitions.
#[test]
fn test_reject_duplicate_value_definitions() {
    let source = r#"function @dup() -> i32 {
block0:
    v0 = iconst 0i32
    jump block1(v0)
block1(v0: i32):
    return v0
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(error.message, "duplicate value definition v0");
}

/// Reject block argument count mismatches.
#[test]
fn test_reject_block_argument_mismatch() {
    let source = r#"function @bad() -> void {
block0:
    jump block1
block1(v0: i32):
    return
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(
        error.message,
        "block argument count mismatch for block1 expected 1 got 0"
    );
}

/// Reject call argument count mismatches.
#[test]
fn test_reject_call_argument_mismatch() {
    let source = r#"extern function @callee(i32) -> i32
function @caller() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = call @callee(v0, v1) -> fn(i32) -> i32
    return v2
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(
        error.message,
        "call argument count mismatch expected 1 got 2"
    );
}

/// Reject call results for void functions.
#[test]
fn test_reject_call_return_value_for_void() {
    let source = r#"extern function @noop() -> void
function @caller() -> void {
block0:
    v0 = call @noop() -> fn() -> void
    return
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(
        error.message,
        "call return value not allowed for void function"
    );
}

/// Reject local references that are not declared in the function.
#[test]
fn test_reject_local_reference_not_in_function() {
    let source = r#"function @first() -> void {
local0: i32
block0:
    return
}

function @bad() -> void {
block0:
    v0 = local.get local0
    return
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(
        error.message,
        "invalid node reference id0 expected Local got Type"
    );
}

/// Reject tail calls with mismatched return types.
#[test]
fn test_reject_tailcall_return_type_mismatch() {
    let source = r#"extern function @noop() -> void
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    tailcall @noop()
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(error.message, "tail call return type mismatch");
}

/// Reject use of undefined values.
#[test]
fn test_reject_use_of_undefined_value() {
    let source = r#"function @bad() -> i32 {
block0:
    return v0
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(error.message, "use of undefined value v0");
}

/// Reject missing return values for non void functions.
#[test]
fn test_reject_missing_return_value() {
    let source = r#"function @bad() -> i32 {
block0:
    return
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(error.message, "return value required for non void function");
}

/// Reject duplicate switch case values.
#[test]
fn test_reject_duplicate_switch_case_value() {
    let source = r#"function @dispatch(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block1, 0 => block1, 0 => block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
}"#;

    let error = Parser::parse(FileId::new(0), source).expect_err("expected parse failure");
    assert_eq!(error.message, "duplicate switch case value 0");
}

/// Reject duplicate instruction ids within a block.
#[test]
fn test_reject_duplicate_instruction_id() {
    let mut tree = crate::NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("dup_inst");

    let ty = tree.insert(Type::Void);
    let value = Value::new(0);
    let instruction = tree.insert(Instruction::Const {
        destination: value,
        value: crate::Constant::int32(1),
    });
    let assume = tree.insert(Instruction::Assume { condition: value });
    let block = Block {
        parameters: Vec::new(),
        instructions: vec![instruction, assume, assume],
        terminator: crate::Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::new(name, Vec::new(), ty, block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let verifier = Verifier::new_with_options(&tree, VerifierOptions::basic());
    let error = verifier
        .verify_function(function_id)
        .expect_err("expected verification failure");
    let expected = format!("duplicate instruction id inst{}", assume.id);
    assert_eq!(error.to_string(), expected);
}

/// Reject duplicate local ids on a function.
#[test]
fn test_reject_duplicate_local_id() {
    let mut tree = crate::NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("dup_local");

    let ty = tree.insert(Type::Void);
    let local_ty = tree.insert(Type::Int {
        width: 32,
        signed: true,
    });
    let local = tree.insert(Local::new(
        local_ty,
        Mutability::Immutable,
        Ownership::Owned,
    ));
    let block = Block {
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: crate::Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::new(name, Vec::new(), ty, block_id);
    function.locals = vec![local, local];
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let verifier = Verifier::new_with_options(&tree, VerifierOptions::basic());
    let error = verifier
        .verify_function(function_id)
        .expect_err("expected verification failure");
    let expected = format!("duplicate local id local{}", local.id);
    assert_eq!(error.to_string(), expected);
}

/// Reject argument slices that exceed the argument buffer.
#[test]
fn test_reject_argument_slice_out_of_bounds() {
    let mut tree = crate::NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("arg_slice");

    let void_ty = tree.insert(Type::Void);
    let signature = tree.insert(Type::FunctionPointer {
        parameters: Vec::new(),
        result: void_ty,
    });
    let callee = Function::import(name, Vec::new(), void_ty);
    let callee_id = tree.insert(callee);

    let instruction = tree.insert(Instruction::Call {
        destination: None,
        function: callee_id,
        arguments: ArgumentSlice::new(0, 1),
        signature,
        effects: None,
    });
    let block = Block {
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: crate::Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::new(name, Vec::new(), void_ty, block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let verifier = Verifier::new_with_options(&tree, VerifierOptions::strict());
    let error = verifier
        .verify_function(function_id)
        .expect_err("expected verification failure");
    assert_eq!(
        error.to_string(),
        "argument slice out of bounds start 0 count 1 len 0"
    );
}

/// Reject call effects with mismatched argument metadata lengths.
#[test]
fn test_reject_call_effect_argument_count_mismatch() {
    let mut tree = crate::NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("bad_effects");

    let void_ty = tree.insert(Type::Void);
    let signature = tree.insert(Type::FunctionPointer {
        parameters: Vec::new(),
        result: void_ty,
    });
    let callee = Function::import(name, Vec::new(), void_ty);
    let callee_id = tree.insert(callee);

    let effects = CallEffects {
        argument_metadata: vec![CallArgumentMetadata::default()],
        ..CallEffects::default()
    };
    let instruction = tree.insert(Instruction::Call {
        destination: None,
        function: callee_id,
        arguments: ArgumentSlice::new(0, 0),
        signature,
        effects: Some(effects),
    });
    let block = Block {
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: crate::Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::new(name, Vec::new(), void_ty, block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let verifier = Verifier::new_with_options(&tree, VerifierOptions::strict());
    let error = verifier
        .verify_function(function_id)
        .expect_err("expected verification failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: call effects argument count mismatch expected 0 got 1"
    );
}
