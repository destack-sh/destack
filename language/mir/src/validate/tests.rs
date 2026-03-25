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
    let source = r#"function @dup() -> i32 {
block0:
    v0: i32 = iconst 0i32
    jump block1(v0)
block1(v0: i32):
    return v0
}"#;

    let error = parse_error(source);
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

    let error = parse_error(source);
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
    v0: i32 = iconst 1i32
    v1: i32 = iconst 2i32
    v2: i32 = call @callee(v0, v1) -> fn(i32) -> i32
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
    let source = r#"extern function @noop() -> void
function @caller() -> void {
block0:
    v0: void = call @noop() -> fn() -> void
    return
}"#;

    let error = parse_error(source);
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
    v0: i32 = local.get local0
    return
}"#;

    let error = parse_error(source);
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

    let error = parse_error(source);
    assert_eq!(error.message, "tail call return type mismatch");
}

/// Reject exceptional calls whose normal continuation omits the result parameter.
#[test]
fn test_reject_call_terminator_missing_normal_result_parameter() {
    let source = r#"extern function @callee(i32) -> i32
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    call @callee(v0) normal block1 unwind block2
block1:
    v1: i32 = iconst 0i32
    return v1
block2(v2: ref<managed readonly i32>):
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
    let source = r#"function @trapper() -> void {
block0:
    trap panic
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "expected value, got CloseBrace");
}

/// Reject panic traps with mutable managed payloads.
#[test]
fn test_reject_trap_panic_with_mutable_payload() {
    let source = r#"type @PanicMessage = {}
function @trapper(v0: ref<managed @PanicMessage>) -> void {
block0(v0: ref<managed @PanicMessage>):
    trap panic v0
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: trap panic requires a non null readonly managed reference payload"
    );
}

/// Reject exceptional calls whose unwind continuation does not accept a managed exception.
#[test]
fn test_reject_call_terminator_with_non_managed_unwind_parameter() {
    let source = r#"extern function @callee(i32) -> i32
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    call @callee(v0) normal block1 unwind block2
block1(v1: i32):
    return v1
block2(v2: i32):
    return v2
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: call unwind continuation requires a managed exception parameter"
    );
}

/// Reject use of undefined values.
#[test]
fn test_reject_use_of_undefined_value() {
    let source = r#"function @bad() -> i32 {
block0:
    return v0
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "use of undefined value v0");
}

/// Reject missing return values for non void functions.
#[test]
fn test_reject_missing_return_value() {
    let source = r#"function @bad() -> i32 {
block0:
    return
}"#;

    let error = parse_error(source);
    assert_eq!(error.message, "return value required for non void function");
}

/// Reject duplicate switch case values.
#[test]
fn test_reject_duplicate_switch_case_value() {
    let source = r#"function @dispatch(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block1, 0 => block1, 0 => block2
block1:
    v1: i32 = iconst 1i32
    return v1
block2:
    v2: i32 = iconst 2i32
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

/// Reject malformed closure env type ids.
#[test]
fn test_reject_function_closure_env_type_wrong_node_kind() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("bad_closure_env_type");

    let void_type = tree.insert_type(Type::Void);
    let block_id = tree.insert(Block {
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: Terminator::Return { value: None },
    });

    let mut function = Function::local(name, Vec::new(), void_type, block_id);
    function.closure_env_type = Some(LocalNodeId::new(block_id.id));
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
    let source = r#"function @bad() -> void {
block0:
    v0: ref<raw i32> = stack.alloc i64
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
    let source = r#"function @bad(v0: [i32; 4], v1: f32) -> i32 {
block0(v0: [i32; 4], v1: f32):
    v2: i32 = element.get v0, v1
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
    let source = r#"type @Pair = { i32, i32 }
function @bad(v0: @Pair) -> i32 {
block0(v0: @Pair):
    v1: i32 = field.get v0, 3
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
    let source = r#"type @Box = { x: i32 }
function @bad() -> i64 {
block0:
    v0: ref<managed @Box> = managed.alloc @Box
    v1: i64 = ptr_to_int v0 -> i64
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
    let source = r#"function @bad(v0: i32) -> i64 {
block0(v0: i32):
    v1: i64 = bitcast v0 -> i64
    return v1
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: bitcast requires equal storage size, got 4 and 8 bytes"
    );
}

/// Allow bitcasts through transparent newtype wrappers.
#[test]
fn test_allow_bitcast_with_transparent_newtype() {
    let source = r#"type @Handle = newtype<ref<raw i32>>
function @ok(v0: ref<raw i32>) -> @Handle {
block0(v0: ref<raw i32>):
    v1: @Handle = bitcast v0 -> @Handle
    return v1
}"#;

    parse_ok(source);
}

/// Reject address space casts that change pointee semantics.
#[test]
fn test_reject_addrspace_cast_with_mismatched_pointee() {
    let source = r#"type @A = { x: i32 }
type @B = { y: i32 }
function @bad() -> void {
block0:
    v0: ref<raw @A> = stack.alloc @A
    v1: ref<raw addrspace(global) @B> = intrinsic.addrspace.cast(v0)
    return
}"#;

    let error = parse_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: addrspace.cast requires matching reference kind, mutability, and pointee"
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

/// Reject managed allocation instructions when no_managed is required.
#[test]
fn test_reject_managed_alloc_with_no_managed_mode() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("no_managed");

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
        "metadata invariant violation: allocation mode violation: 'managed.alloc' is invalid because no_managed forbids managed allocations"
    );
}

/// Reject raw heap allocations when stack_only is required.
#[test]
fn test_reject_raw_alloc_with_stack_only_mode() {
    let mut tree = NodeTree::new();
    let pool = StringPool::new();
    let name = pool.intern("stack_only");

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
        "metadata invariant violation: allocation mode violation: 'raw.alloc' is invalid because stack_only forbids non-stack allocations"
    );
}
