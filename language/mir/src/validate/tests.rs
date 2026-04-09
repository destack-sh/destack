use destack_core::StringPool;
use destack_source::{FileId, Span};

use crate::parse::{ParseError, ParseOptions, Parser};
use crate::{
    AddressSpace, AllocationMode, ArgumentSlice, Block, CallArgumentMetadata, CallBehavior,
    CallEffects, CallSite, Constant, Copyability, DebugBindingKind, DebugRangeStart,
    DebugScopeKind, DebugValueLocation, DevirtualizationMetadata, Field, Function, Instruction,
    Layout, LayoutField, LayoutType, Local, LocalNodeId, Mutability, NodeTree, Ownership,
    ProvenanceKind, ReferenceKind, Repeatability, Terminator, Type, UnwindBehavior, Value,
    VtableSlotId,
};

use super::Validator;

/// Parse MIR source and return the parse error.
fn parse_error(source: &str) -> ParseError {
    Parser::parse(FileId::new(0), source, ParseOptions::default())
        .expect_err("expected parse failure")
}

/// Parse MIR source and assert success.
fn parse_ok(source: &str) {
    Parser::parse(FileId::new(0), source, ParseOptions::default()).expect("expected parse success");
}

/// Local references must resolve to locals declared on the function.
#[test]
fn test_validate_rejects_local_not_in_function() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("test");

    let ty = tree.insert_type(Type::Void);
    let local_ty = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
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
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), ty, block_id);
    function.set_value_type(Value::new(0), local_ty);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    let expected = format!("local reference local{} not defined in function", local.id);
    assert_eq!(error.to_string(), expected);
}

/// Reject duplicate value definitions.
#[test]
fn test_reject_duplicate_value_definitions() {
    let source = r#"function dup(): int32 {
b0:
    v0: int32 = 0int32
    jump b1(v0)
b1(v0: int32):
    return v0
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "duplicate value definition v0");
}

/// Reject block argument count mismatches.
#[test]
fn test_reject_block_argument_mismatch() {
    let source = r#"function bad(): void {
b0:
    jump b1
b1(v0: int32):
    return
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "block argument count mismatch for block1 expected 1 got 0"
    );
}

/// Reject call argument count mismatches.
#[test]
fn test_reject_call_argument_mismatch() {
    let source = r#"extern function callee(int32): int32
function caller(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = call callee(v0, v1): (int32) -> int32
    return v2
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "call argument count mismatch expected 1 got 2"
    );
}

/// Reject call results for void functions.
#[test]
fn test_reject_call_return_value_for_void() {
    let source = r#"extern function noop(): void
function caller(): void {
b0:
    v0: void = call noop(): () -> void
    return
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "call return value not allowed for void function"
    );
}

/// Reject direct calls to functions that require an environment.
#[test]
fn test_reject_direct_call_to_environment_function() {
    let source = r#"@environment(ref<int32, managed>)
function callee(): int32 {
b0:
    v0: ref<int32, managed> = function.environment
    v1: int32 = load v0
    return v1
}

function caller(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: direct call cannot target a function with an environment"
    );
}

/// Reject tail calls to functions that require an environment.
#[test]
fn test_reject_tail_call_to_environment_function() {
    let source = r#"@environment(ref<int32, managed>)
function callee(): int32 {
b0:
    v0: ref<int32, managed> = function.environment
    v1: int32 = load v0
    return v1
}

function caller(): int32 {
b0:
    tailCall callee(): () -> int32
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: direct call cannot target a function with an environment"
    );
}

/// Reject taking a plain function address for an environment function.
#[test]
fn test_reject_function_addr_for_environment_function() {
    let source = r#"@environment(ref<int32, managed>)
function callee(): int32 {
b0:
    v0: ref<int32, managed> = function.environment
    v1: int32 = load v0
    return v1
}

function caller(v0: ref<int32, managed>): int32 {
b0(v0: ref<int32, managed>):
    v1: fn() -> int32 = function.address callee
    return v0
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: function.address cannot target a function with an environment"
    );
}

/// Reject function.bind when the environment operand type mismatches.
#[test]
fn test_reject_function_value_environment_type_mismatch() {
    let source = r#"@environment(ref<int32, managed>)
function callee(): int32 {
b0:
    v0: ref<int32, managed> = function.environment
    v1: int32 = load v0
    return v1
}

function caller(v0: ref<int64, managed>): int32 {
b0(v0: ref<int64, managed>):
    v1: closure() -> int32 = function.bind callee, v0
    return v0
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: function.bind environment type mismatch"
    );
}

/// Reject field projection on opaque callable values.
#[test]
fn test_reject_field_get_on_function_value() {
    let source = r#"@environment(ref<int32, managed>)
function callee(): int32 {
b0:
    v0: ref<int32, managed> = function.environment
    v1: int32 = load v0
    return v1
}

function caller(v0: ref<int32, managed>): fn() -> int32    {
b0(v0: ref<int32, managed>):
    v1: closure() -> int32 = function.bind callee, v0
    v2: fn() -> int32 = field.get v1, 0
    return v2
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: field.get does not support closure"
    );
}

/// Reject tail calls with mismatched return types.
#[test]
fn test_reject_tailcall_return_type_mismatch() {
    let source = r#"extern function noop(): void
function caller(v0: int32): int32 {
b0(v0: int32):
    tailCall noop(): () -> void
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "tail call return type mismatch");
}

/// Reject exceptional calls whose success continuation omits the result parameter.
#[test]
fn test_reject_call_terminator_missing_normal_result_parameter() {
    let source = r#"extern function callee(int32): int32
function caller(v0: int32): int32 {
b0(v0: int32):
    invoke callee(v0): (int32) -> int32 -> b1, catch b2
b1:
    v1: int32 = 0int32
    return v1
b2(v2: ref<int32, managed, readonly>):
    throw v2
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "block argument count mismatch for block1 expected 1 got 0"
    );
}

/// Reject panic traps without a managed payload.
#[test]
fn test_reject_trap_panic_without_payload() {
    let source = r#"function trapper(): void {
b0:
    trap.panic
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "expected value, got CloseBrace");
}

/// Reject panic traps with mutable managed payloads.
#[test]
fn test_reject_trap_panic_with_mutable_payload() {
    let source = r#"type PanicMessage { }
function trapper(v0: ref<PanicMessage, managed>): void {
b0(v0: ref<PanicMessage, managed>):
    trap.panic v0
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: trap.panic requires a non null readonly managed reference payload"
    );
}

/// Reject exceptional calls whose exception continuation does not accept a managed exception.
#[test]
fn test_reject_call_terminator_with_non_managed_unwind_parameter() {
    let source = r#"extern function callee(int32): int32
function caller(v0: int32): int32 {
b0(v0: int32):
    invoke callee(v0): (int32) -> int32 -> b1, catch b2
b1(v1: int32):
    return v1
b2(v2: int32):
    return v2
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: call exception continuation requires a managed exception parameter"
    );
}

/// Reject use of undefined values.
#[test]
fn test_reject_use_of_undefined_value() {
    let source = r#"function bad(): int32 {
b0:
    return v0
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "use of undefined value v0");
}

/// Reject missing return values for non void functions.
#[test]
fn test_reject_missing_return_value() {
    let source = r#"function bad(): int32 {
b0:
    return
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "return value required for non void function");
}

/// Reject duplicate switch case values.
#[test]
fn test_reject_duplicate_switch_case_value() {
    let source = r#"function dispatch(v0: int32): int32 {
b0(v0: int32):
    switch v0, b1, 0 => b1, 0 => b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "duplicate switch case value 0");
}

/// Reject duplicate instruction ids within a block.
#[test]
fn test_reject_duplicate_instruction_id() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("dup_inst");

    let ty = tree.insert_type(Type::Void);
    let value_ty = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let value = Value::new(0);
    let instruction = tree.insert(Instruction::Const {
        destination: value,
        value: Constant::int32(1),
    });
    let assume = tree.insert(Instruction::Assume { condition: value });
    let block = Block {
        parameters: Vec::new(),
        instructions: vec![instruction, assume, assume],
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), ty, block_id);
    function.set_value_type(value, value_ty);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    let expected = format!("duplicate instruction id inst{}", assume.id);
    assert_eq!(error.to_string(), expected);
}

/// Reject one empty debug binding location range.
#[test]
fn test_reject_debug_binding_empty_range() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("debug_range");

    let void_type = tree.insert_type(Type::Void);
    let value_type = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let instruction = tree.insert(Instruction::Const {
        destination: Value::new(0),
        value: Constant::int32(1),
    });
    let block_id = tree.insert(Block {
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: Terminator::Return { value: None },
    });

    let mut function = Function::local(name, Vec::new(), void_type, block_id);
    function.set_value_type(Value::new(0), value_type);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let span = Span::new(FileId::new(0), 0, 0);
    let scope_id = tree
        .debug_table
        .create_scope(DebugScopeKind::Function, Some(name), span, None);
    tree.debug_table.set_function_scope(function_id, scope_id);

    let binding_id =
        tree.debug_table
            .create_binding(name, value_type, scope_id, DebugBindingKind::Local);
    tree.debug_table.add_binding_location_range(
        binding_id,
        DebugValueLocation::Value(Value::new(0)),
        DebugRangeStart::instruction(instruction),
        Some(instruction),
    );

    let validator = Validator::new(&tree);
    let error = validator
        .validate()
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: debug binding ranges must be non-empty and ordered"
    );
}

/// Reject one cyclic provenance parent chain.
#[test]
fn test_reject_provenance_cycle() {
    let mut tree = NodeTree::new();
    let first = tree
        .provenance_table
        .create(ProvenanceKind::Derived, None, Vec::new(), Vec::new());
    let second =
        tree.provenance_table
            .create(ProvenanceKind::Derived, None, Vec::new(), vec![first]);
    tree.provenance_table.records[first.index()]
        .parents
        .push(second);

    let validator = Validator::new(&tree);
    let error = validator
        .validate()
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: provenance parent chain must be acyclic"
    );
}

/// Reject duplicate local ids on a function.
#[test]
fn test_reject_duplicate_local_id() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("dup_local");

    let ty = tree.insert_type(Type::Void);
    let local_ty = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let local = tree.insert(Local::new(
        local_ty,
        Mutability::Immutable,
        Ownership::Owned,
    ));
    let block = Block {
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), ty, block_id);
    function.locals = vec![local, local];
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    let expected = format!("duplicate local id local{}", local.id);
    assert_eq!(error.to_string(), expected);
}

/// Reject malformed function return type ids.
#[test]
fn test_reject_function_return_type_wrong_node_kind() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("bad_return_type");

    let void_type = tree.insert_type(Type::Void);
    let block_id = tree.insert(Block {
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: Terminator::Return { value: None },
    });

    let mut function = Function::local(name, Vec::new(), void_type, block_id);
    function.return_type = LocalNodeId::new(block_id.id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        format!(
            "invalid node reference id{} expected Type got Block",
            block_id.id
        )
    );
}

/// Reject malformed function environment type ids.
#[test]
fn test_reject_function_environment_type_wrong_node_kind() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("bad_environment_type");

    let void_type = tree.insert_type(Type::Void);
    let block_id = tree.insert(Block {
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: Terminator::Return { value: None },
    });

    let mut function = Function::local(name, Vec::new(), void_type, block_id);
    function.environment = Some(LocalNodeId::new(block_id.id));
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        format!(
            "invalid node reference id{} expected Type got Block",
            block_id.id
        )
    );
}

/// Reject argument slices that exceed the argument buffer.
#[test]
fn test_reject_argument_slice_out_of_bounds() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("arg_slice");

    let void_ty = tree.insert_type(Type::Void);
    let signature = tree.insert_type(Type::FunctionPointer {
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
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), void_ty, block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "argument slice out of bounds start 0 count 1 len 0"
    );
}

/// Reject call effects with mismatched argument metadata lengths.
#[test]
fn test_reject_call_effect_argument_count_mismatch() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("bad_effects");

    let void_ty = tree.insert_type(Type::Void);
    let signature = tree.insert_type(Type::FunctionPointer {
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
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), void_ty, block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: call effects argument count mismatch expected 0 got 1"
    );
}

/// Pure effects must not suspend execution.
#[test]
fn test_reject_pure_effect_with_suspend() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("pure_suspend");

    let void_ty = tree.insert_type(Type::Void);
    let signature = tree.insert_type(Type::FunctionPointer {
        parameters: Vec::new(),
        result: void_ty,
    });
    let callee = Function::import(name, Vec::new(), void_ty);
    let callee_id = tree.insert(callee);

    let behavior = CallBehavior {
        repeatability: Repeatability::Pure,
        unwind_behavior: UnwindBehavior::CannotUnwind,
        may_suspend: true,
        ..CallBehavior::none()
    };
    let effects = CallEffects {
        behavior: Some(behavior),
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
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), void_ty, block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: pure effect cannot suspend"
    );
}

/// Pure effects must not unwind.
#[test]
fn test_reject_pure_effect_with_unwind() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("pure_unwind");

    let void_ty = tree.insert_type(Type::Void);
    let signature = tree.insert_type(Type::FunctionPointer {
        parameters: Vec::new(),
        result: void_ty,
    });
    let callee = Function::import(name, Vec::new(), void_ty);
    let callee_id = tree.insert(callee);

    let behavior = CallBehavior {
        repeatability: Repeatability::Pure,
        unwind_behavior: UnwindBehavior::MayUnwind,
        may_suspend: false,
        ..CallBehavior::none()
    };
    let effects = CallEffects {
        behavior: Some(behavior),
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
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), void_ty, block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: pure effect cannot unwind"
    );
}

/// Dispatch metadata must not carry empty sparse entries.
#[test]
fn test_reject_empty_dispatch_callsite_metadata() {
    let mut tree = NodeTree::new();
    tree.dispatch_table.insert_callsite_metadata(
        CallSite::Instruction(LocalNodeId::new(0)),
        DevirtualizationMetadata::default(),
    );

    let error = Validator::new(&tree)
        .validate()
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: dispatch callsite metadata must carry at least one fact"
    );
}

/// Reject pointer-producing instructions with mismatched pointee type.
#[test]
fn test_reject_pointer_result_pointee_mismatch() {
    let source = r#"function bad(): void {
b0:
    v0: ref<int32, raw> = stack.alloc int64
    return
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: pointer-producing instruction result type mismatches pointee"
    );
}

/// Reject element projections with non integer index values.
#[test]
fn test_reject_element_get_with_non_integer_index() {
    let source = r#"function bad(v0: int32[4], v1: float32): int32 {
b0(v0: int32[4], v1: float32):
    v2: int32 = element.get v0, v1
    return v2
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: element.get index must be an integer type"
    );
}

/// Reject field projections with out-of-bounds static indices.
#[test]
fn test_reject_field_get_with_out_of_bounds_index() {
    let source = r#"type Pair {
    int32;
    int32;
}
function bad(v0: Pair): int32 {
b0(v0: Pair):
    v1: int32 = field.get v0, 3
    return v1
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: field.get field index 3 out of bounds for struct with 2 fields"
    );
}

/// Reject pointer to integer casts for non raw references.
#[test]
fn test_reject_ptr_to_int_for_managed_reference() {
    let source = r#"type Box {
    x: int32;
}
function bad(): int64 {
b0:
    v0: ref<Box, managed> = managed.alloc Box
    v1: int64 = cast.pointerToInt v0 -> int64
    return v1
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: ptr_to_int requires raw pointer source and integer destination"
    );
}

/// Reject bitcasts that change storage size.
#[test]
fn test_reject_bitcast_with_size_mismatch() {
    let source = r#"function bad(v0: int32): int64 {
b0(v0: int32):
    v1: int64 = cast.bit v0 -> int64
    return v1
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: cast.bit requires equal storage size, got 4 and 8 bytes"
    );
}

/// Allow bitcasts through transparent newtype wrappers.
#[test]
fn test_allow_bitcast_with_transparent_newtype() {
    let source = r#"
type Handle newtype<ref<int32, raw>>
function ok(v0: ref<int32, raw>): Handle {
b0(v0: ref<int32, raw>):
    v1: Handle = cast.bit v0 -> Handle
    return v1
}"#;

    parse_ok(source);
}

/// Reject address space casts that change pointee semantics.
#[test]
fn test_reject_address_space_cast_with_mismatched_pointee() {
    let source = r#"type A {
    x: int32;
}

type B {
    y: int32;
}
function bad(): void {
b0:
    v0: ref<A, raw> = stack.alloc A
    v1: ref<B, raw, addressSpace(global)> = intrinsic.addressSpace.cast(v0)
    return
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: addressSpace.cast requires matching reference kind, mutability, and pointee"
    );
}

/// Reject struct layout metadata when field types disagree with the struct type.
#[test]
fn test_reject_struct_layout_field_type_mismatch() {
    let mut tree = NodeTree::new();
    let strings = StringPool::new();
    let field_name = strings.intern("x");

    let int32 = tree.insert_type(Type::INT32);
    let float32 = tree.insert_type(Type::FLOAT32);
    let field_id = tree.insert(Field {
        name: Some(field_name),
        ty: int32,
    });
    let struct_type = tree.insert_type(Type::Struct {
        fields: vec![field_id],
        copyability: Copyability::Trivial,
    });

    let layout_id = tree.type_table.layout_table.insert(Layout {
        layout_type: LayoutType::Struct,
        size: 4,
        alignment: 4,
        fields: vec![LayoutField {
            name: field_name,
            ty: float32,
            offset: 0,
            size: 4,
            alignment: 4,
            source_index: Some(0),
        }],
    });
    tree.type_table.set_layout_id(struct_type, layout_id);

    let validator = Validator::new(&tree);
    let error = validator
        .validate()
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: struct field type does not match layout field type"
    );
}

/// Dynamic call declared targets are metadata only.
#[test]
fn test_dynamic_call_declared_target_is_metadata_only() {
    let instruction = Instruction::CallVirtual {
        destination: None,
        receiver: Value::new(0),
        arguments: ArgumentSlice::new(0, 0),
        declaring_type: LocalNodeId::new(0),
        slot_id: VtableSlotId::new(0),
        signature: LocalNodeId::new(0),
        effects: None,
    };

    assert_eq!(instruction.call_declared_target(), None);
}

/// Reject managed allocation instructions when noManaged is required.
#[test]
fn test_reject_managed_alloc_with_no_managed_mode() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("noManaged");

    let void_ty = tree.insert_type(Type::Void);
    let layout_ty = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let result_ty = tree.insert_type(Type::Reference {
        kind: ReferenceKind::Managed,
        address_space: AddressSpace::Generic,
        mutability: Mutability::Mutable,
        pointee: layout_ty,
        is_nullable: false,
    });
    let destination = Value::new(0);
    let instruction = tree.insert(Instruction::ManagedAlloc {
        destination,
        layout: layout_ty,
        result_type: result_ty,
    });
    let block = Block {
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), void_ty, block_id);
    function.set_value_type(destination, result_ty);
    function.allocation = AllocationMode::NoManaged;
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: allocation mode violation: 'managed.alloc' is invalid because noManaged forbids managed allocations"
    );
}

/// Reject raw heap allocations when stackOnly is required.
#[test]
fn test_reject_raw_alloc_with_stack_only_mode() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("stackOnly");

    let void_ty = tree.insert_type(Type::Void);
    let layout_ty = tree.insert_type(Type::Int {
        width: 64,
        is_signed: true,
    });
    let result_ty = tree.insert_type(Type::Reference {
        kind: ReferenceKind::Raw,
        address_space: AddressSpace::Generic,
        mutability: Mutability::Mutable,
        pointee: layout_ty,
        is_nullable: false,
    });
    let destination = Value::new(0);
    let instruction = tree.insert(Instruction::RawAlloc {
        destination,
        layout: layout_ty,
        result_type: result_ty,
    });
    let block = Block {
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: Terminator::Return { value: None },
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), void_ty, block_id);
    function.set_value_type(destination, result_ty);
    function.allocation = AllocationMode::StackOnly;
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: allocation mode violation: 'raw.alloc' is invalid because stackOnly forbids non-stack allocations"
    );
}
