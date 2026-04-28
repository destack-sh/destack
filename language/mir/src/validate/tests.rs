use destack_core::StringPool;
use destack_source::FileId;

use crate::parse::{ParseError, ParseOptions, Parser};
use crate::{
    AddressSpace, AllocationMode, ArgumentAttribute, ArgumentSlice, Attribute, AttributeArgs,
    AttributeIdentifier, AttributeValue, Block, Call, CallBehavior, Constant, Copy,
    DebugBindingKind, DebugRangeStart, DebugScopeKind, DebugValueLocation, EffectClass, Field,
    Function, FunctionReference, Instruction, Layout, LayoutField, LayoutKind, Local, LocalNodeId,
    LocalReference, Mutability, Ownership, ProvenanceAnchor, ProvenanceKey, ReferenceKind,
    ReferenceMap, SuspendBehavior, Terminator, Tree, Type, TypeReference, UnwindBehavior, Value,
    ValueReference, VtableSlotId,
};

use super::Validator;

/// Parse and validate MIR source, then return the validation error.
fn assert_validate_error(source: &str) -> ParseError {
    Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect_err("expected parse failure")
}

/// Parse and validate MIR source, then assert success.
fn assert_validate_ok(source: &str) {
    Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("expected parse success");
}

/// Wrap one type id as a recoverable type reference.
fn type_reference(ty: LocalNodeId<Type>) -> TypeReference {
    TypeReference::Type(ty)
}

/// Wrap one value as a recoverable value reference.
fn value_reference(value: Value) -> ValueReference {
    ValueReference::Value(value)
}

/// Wrap one local id as a recoverable local reference.
fn local_reference(local: LocalNodeId<Local>) -> LocalReference {
    LocalReference::Local(local)
}

/// Wrap one function id as a recoverable function reference.
fn function_reference(function: LocalNodeId<Function>) -> FunctionReference {
    FunctionReference::Function(function)
}

/// Local references must resolve to locals declared on the function.
#[test]
fn test_validate_rejects_local_not_in_function() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("test");

    let ty = tree.insert_type(Type::Void);
    let local_ty = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let local = tree.insert(Local::new(
        type_reference(local_ty),
        Mutability::Immutable,
        Ownership::Owned,
    ));

    let instruction = tree.insert(Instruction::LocalGet {
        destination: value_reference(Value::new(0)),
        local: local_reference(local),
    });
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(ty), block_id);
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

/// Recovered MIR syntax markers are not valid MIR.
#[test]
fn test_reject_recovered_parse_nodes() {
    let source = r#"function broken(): void {
b0:
    value0: int32 = int.add

b1:
    return
}"#;

    let parsed = Parser::parse(FileId::new(0), source, ParseOptions::default());
    let (tree, _, diagnostics) = parsed.into_parts();

    assert_eq!(diagnostics.len(), 1);

    let validator = Validator::new(&tree);
    let error = validator
        .validate()
        .expect_err("expected validation failure");
    assert_eq!(error.to_string(), "recovered MIR instruction is not valid");
}

/// Reject recoverable attribute syntax after validation.
#[test]
fn test_reject_recovered_attribute_values() {
    let mut tree = Tree::new();
    let strings = StringPool::new();
    let type_id = tree.insert_type(Type::Void);
    let attribute_name = strings.intern("broken");

    tree.set_attributes(
        type_id,
        vec![Attribute {
            name: AttributeIdentifier::identifier(attribute_name),
            args: AttributeArgs::Value(AttributeValue::Missing),
        }],
    );

    let validator = Validator::new(&tree);
    let error = validator
        .validate()
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: attribute value is missing"
    );
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

    let error = assert_validate_error(source);
    assert_eq!(error.message, "duplicate value definition v0");
}

/// Reject duplicate final SSA value names.
#[test]
fn test_reject_duplicate_value_names() {
    let source = r#"function collide(value1: int32): int32 {
b0(value1: int32):
    v1: int32 = 0int32
    return value1
}"#;

    let error = assert_validate_error(source);
    assert_eq!(error.message, "duplicate MIR value name");
}

/// Reject duplicate final block names.
#[test]
fn test_reject_duplicate_block_names() {
    let source = r#"function collide(): void {
block1:
    jump b1

b1:
    return
}"#;

    let error = assert_validate_error(source);
    assert_eq!(error.message, "duplicate MIR block name");
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

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
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
    v0: ref<int32, managed> = callable.environment
    v1: int32 = load v0
    return v1
}

function caller(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#;

    let error = assert_validate_error(source);
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
    v0: ref<int32, managed> = callable.environment
    v1: int32 = load v0
    return v1
}

function caller(): int32 {
b0:
    tailCall callee(): () -> int32
}"#;

    let error = assert_validate_error(source);
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
    v0: ref<int32, managed> = callable.environment
    v1: int32 = load v0
    return v1
}

function caller(v0: ref<int32, managed>): int32 {
b0(v0: ref<int32, managed>):
    v1: () -> int32 = function.address callee
    return v0
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: function.address cannot target a function with an environment"
    );
}

/// Reject callable.bind when the environment operand type mismatches.
#[test]
fn test_reject_callable_environment_type_mismatch() {
    let source = r#"@environment(ref<int32, managed>)
function callee(): int32 {
b0:
    v0: ref<int32, managed> = callable.environment
    v1: int32 = load v0
    return v1
}

function caller(v0: ref<int64, managed>): int32 {
b0(v0: ref<int64, managed>):
    v1: () => int32 = callable.bind callee, v0
    return v0
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: callable.bind environment type mismatch"
    );
}

/// Reject field projection on opaque callable values.
#[test]
fn test_reject_field_get_on_callable() {
    let source = r#"@environment(ref<int32, managed>)
function callee(): int32 {
b0:
    v0: ref<int32, managed> = callable.environment
    v1: int32 = load v0
    return v1
}

function caller(v0: ref<int32, managed>): () -> int32    {
b0(v0: ref<int32, managed>):
    v1: () => int32 = callable.bind callee, v0
    v2: () -> int32 = field.get v1, 0
    return v2
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: field.get does not support callable"
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

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
    assert_eq!(error.message, "expected value reference, got CloseBrace");
}

/// Reject panic traps with mutable managed payloads.
#[test]
fn test_reject_trap_panic_with_mutable_payload() {
    let source = r#"type PanicMessage { }
function trapper(v0: ref<PanicMessage, managed>): void {
b0(v0: ref<PanicMessage, managed>):
    trap.panic v0
}"#;

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
    assert_eq!(error.message, "use of undefined value v0");
}

/// Reject missing return values for non void functions.
#[test]
fn test_reject_missing_return_value() {
    let source = r#"function bad(): int32 {
b0:
    return
}"#;

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
    assert_eq!(error.message, "duplicate switch case value 0");
}

/// Reject duplicate instruction ids within a block.
#[test]
fn test_reject_duplicate_instruction_id() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("dup_inst");

    let ty = tree.insert_type(Type::Void);
    let value_ty = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let value = Value::new(0);
    let instruction = tree.insert(Instruction::Const {
        destination: value_reference(value),
        value: Constant::int32(1),
    });
    let assume = tree.insert(Instruction::Assume {
        condition: value_reference(value),
    });
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: vec![instruction, assume, assume],
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(ty), block_id);
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

/// Reject duplicate terminator ids within a function.
#[test]
fn test_reject_duplicate_terminator_id() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("dup_terminator");

    let void_type = tree.insert_type(Type::Void);
    let shared_terminator = tree.insert(Terminator::Return { value: None });
    let entry_block = tree.insert(Block {
        name: None,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: shared_terminator,
    });
    let second_block = tree.insert(Block {
        name: None,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: shared_terminator,
    });

    let mut function = Function::local(name, Vec::new(), type_reference(void_type), entry_block);
    function.blocks = vec![entry_block, second_block];
    function.entry = Some(entry_block);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    let expected = format!("duplicate terminator id term{}", shared_terminator.id);
    assert_eq!(error.to_string(), expected);
}

/// Reject one empty debug binding location range.
#[test]
fn test_reject_debug_binding_empty_range() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("debug_range");

    let void_type = tree.insert_type(Type::Void);
    let value_type = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let instruction = tree.insert(Instruction::Const {
        destination: value_reference(Value::new(0)),
        value: Constant::int32(1),
    });
    let terminator_id = tree.insert(Terminator::Return { value: None });
    let block_id = tree.insert(Block {
        name: Some(pool.intern("entry0")),
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: terminator_id,
    });

    let mut function = Function::local(name, Vec::new(), type_reference(void_type), block_id);
    function.set_value_type(Value::new(0), value_type);
    function.value_names[0] = Some(pool.intern("value0"));
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let scope_id =
        tree.metadata
            .debug
            .create_scope(DebugScopeKind::Function, Some(name), None, None);
    tree.metadata
        .debug
        .set_function_scope(function_id, scope_id);

    let binding_id = tree.metadata.debug.create_binding(
        name,
        value_type,
        scope_id,
        None,
        DebugBindingKind::Local,
    );
    tree.metadata.debug.add_binding_location_range(
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

/// Reject one cyclic origin graph.
#[test]
fn test_reject_origin_cycle() {
    let mut tree = Tree::new();
    let first =
        tree.metadata
            .provenance
            .create(ProvenanceAnchor::Synthetic, None, Vec::new(), None);
    let second = tree.metadata.provenance.create(
        ProvenanceAnchor::Mir(first),
        None,
        vec![ProvenanceKey::Mir(first)],
        None,
    );
    tree.metadata.provenance.record_by_id[first.index()].contributors =
        vec![ProvenanceKey::Mir(second)];

    let validator = Validator::new(&tree);
    let error = validator
        .validate()
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: origin graph must be acyclic"
    );
}

/// Reject duplicate local ids on a function.
#[test]
fn test_reject_duplicate_local_id() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("dup_local");

    let ty = tree.insert_type(Type::Void);
    let local_ty = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let local = tree.insert(Local::new(
        type_reference(local_ty),
        Mutability::Immutable,
        Ownership::Owned,
    ));
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(ty), block_id);
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
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("bad_return_type");

    let void_type = tree.insert_type(Type::Void);
    let terminator_id = tree.insert(Terminator::Return { value: None });
    let block_id = tree.insert(Block {
        name: None,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: terminator_id,
    });

    let mut function = Function::local(name, Vec::new(), type_reference(void_type), block_id);
    function.return_type = type_reference(LocalNodeId::new(block_id.id));
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

/// Reject malformed callable environment type ids.
#[test]
fn test_reject_callable_environment_type_wrong_node_kind() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("bad_environment_type");

    let void_type = tree.insert_type(Type::Void);
    let terminator_id = tree.insert(Terminator::Return { value: None });
    let block_id = tree.insert(Block {
        name: None,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: terminator_id,
    });

    let mut function = Function::local(name, Vec::new(), type_reference(void_type), block_id);
    function.environment = Some(type_reference(LocalNodeId::new(block_id.id)));
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
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("arg_slice");

    let void_ty = tree.insert_type(Type::Void);
    let signature = tree.insert_type(Type::FunctionSignature {
        parameters: Vec::new(),
        result: type_reference(void_ty),
    });
    let callee = Function::import(name, Vec::new(), type_reference(void_ty));
    let callee_id = tree.insert(callee);

    let instruction = tree.insert(Instruction::Call {
        destination: None,
        function: function_reference(callee_id),
        call: Call::new(ArgumentSlice::new(0, 1), type_reference(signature)),
    });
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(void_ty), block_id);
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

/// Reject call metadata with mismatched argument attribute lengths.
#[test]
fn test_reject_call_effect_argument_count_mismatch() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("bad_effects");

    let void_ty = tree.insert_type(Type::Void);
    let signature = tree.insert_type(Type::FunctionSignature {
        parameters: Vec::new(),
        result: type_reference(void_ty),
    });
    let callee = Function::import(name, Vec::new(), type_reference(void_ty));
    let callee_id = tree.insert(callee);

    let instruction = tree.insert(Instruction::Call {
        destination: None,
        function: function_reference(callee_id),
        call: Call {
            argument_attributes: vec![ArgumentAttribute::default()],
            ..Call::new(ArgumentSlice::new(0, 0), type_reference(signature))
        },
    });
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(void_ty), block_id);
    function.blocks = vec![block_id];
    function.entry = Some(block_id);
    let function_id = tree.insert(function);

    let validator = Validator::new(&tree);
    let error = validator
        .validate_function(function_id)
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: call argument attribute count mismatch expected 0 got 1"
    );
}

/// Pure effects must not suspend execution.
#[test]
fn test_reject_pure_effect_with_suspend() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("pure_suspend");

    let void_ty = tree.insert_type(Type::Void);
    let signature = tree.insert_type(Type::FunctionSignature {
        parameters: Vec::new(),
        result: type_reference(void_ty),
    });
    let callee = Function::import(name, Vec::new(), type_reference(void_ty));
    let callee_id = tree.insert(callee);

    let behavior = CallBehavior {
        effect_class: EffectClass::Pure,
        suspend: SuspendBehavior::MaySuspend,
        ..CallBehavior::none()
    };
    let instruction = tree.insert(Instruction::Call {
        destination: None,
        function: function_reference(callee_id),
        call: Call {
            behavior: Some(behavior),
            ..Call::new(ArgumentSlice::new(0, 0), type_reference(signature))
        },
    });
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(void_ty), block_id);
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
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("pure_unwind");

    let void_ty = tree.insert_type(Type::Void);
    let signature = tree.insert_type(Type::FunctionSignature {
        parameters: Vec::new(),
        result: type_reference(void_ty),
    });
    let callee = Function::import(name, Vec::new(), type_reference(void_ty));
    let callee_id = tree.insert(callee);

    let behavior = CallBehavior {
        effect_class: EffectClass::Pure,
        unwind: UnwindBehavior::MayUnwind,
        ..CallBehavior::none()
    };
    let instruction = tree.insert(Instruction::Call {
        destination: None,
        function: function_reference(callee_id),
        call: Call {
            behavior: Some(behavior),
            ..Call::new(ArgumentSlice::new(0, 0), type_reference(signature))
        },
    });
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(void_ty), block_id);
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

/// Reject pointer-producing instructions with mismatched pointee type.
#[test]
fn test_reject_pointer_result_pointee_mismatch() {
    let source = r#"function bad(): void {
b0:
    v0: ref<int32, raw> = stack.alloc int64
    return
}"#;

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: element.get index must be an integer type"
    );
}

/// Reject element gets on slices.
#[test]
fn test_reject_element_get_with_slice() {
    let source = r#"function bad(v0: slice<int32>, v1: int64): int32 {
b0(v0: slice<int32>, v1: int64):
    v2: int32 = element.get v0, v1
    return v2
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: element.get expects a fixed array aggregate"
    );
}

/// Reject element sets on slices.
#[test]
fn test_reject_element_set_with_slice() {
    let source = r#"function bad(v0: slice<int32>, v1: int64, v2: int32): slice<int32> {
b0(v0: slice<int32>, v1: int64, v2: int32):
    v3: slice<int32> = element.set v0, v1, v2
    return v3
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: element.set expects a fixed array aggregate"
    );
}

/// Accept element addresses on slices.
#[test]
fn test_accept_element_address_with_slice() {
    let source = r#"function good(v0: slice<int32>, v1: int64): ref<int32, managed> {
b0(v0: slice<int32>, v1: int64):
    v2: ref<int32, managed> = element.address v0, v1
    return v2
}"#;

    assert_validate_ok(source);
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

    let error = assert_validate_error(source);
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
    v0: ref<Box, managed> = new Box
    v1: int64 = cast.pointerToInt v0 -> int64
    return v1
}"#;

    let error = assert_validate_error(source);
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

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: cast.bit requires equal storage size, got 4 and 8 bytes"
    );
}

/// Allow bitcasts through transparent newtype wrappers.
#[test]
fn test_allow_bitcast_with_transparent_newtype() {
    let source = r#"
type Handle = newtype<ref<int32, raw>>;

function ok(v0: ref<int32, raw>): Handle {
b0(v0: ref<int32, raw>):
    v1: Handle = cast.bit v0 -> Handle
    return v1
}"#;

    assert_validate_ok(source);
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
    v1: ref<B, raw, space(local)> = intrinsic.space.cast(v0)
    return
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: space.cast requires matching reference kind, mutability, and pointee"
    );
}

/// Reject struct layout metadata when field types disagree with the struct type.
#[test]
fn test_reject_struct_layout_field_type_mismatch() {
    let mut tree = Tree::new();
    let strings = StringPool::new();
    let field_name = strings.intern("x");

    let int32 = tree.insert_type(Type::INT32);
    let float32 = tree.insert_type(Type::FLOAT32);
    let field_id = tree.insert(Field {
        name: Some(field_name),
        ty: type_reference(int32),
    });
    let struct_type = tree.insert_type(Type::Struct {
        fields: vec![field_id],
        copy: Copy::Yes,
    });

    let layout_id = tree.metadata.layout.layout_table.insert(Layout {
        kind: LayoutKind::Struct,
        size: 4,
        alignment: 4,
        reference_map: ReferenceMap::empty(),
        fields: vec![LayoutField {
            name: Some(field_name),
            ty: float32,
            offset: 0,
            size: 4,
            alignment: 4,
            source_index: Some(0),
        }],
    });
    tree.metadata.layout.set_layout_id(struct_type, layout_id);

    let validator = Validator::new(&tree);
    let error = validator
        .validate()
        .expect_err("expected validation failure");
    assert_eq!(
        error.to_string(),
        "metadata invariant violation: struct field type does not match layout field type"
    );
}

/// Dynamic call declared targets are stored inline on the instruction.
#[test]
fn test_dynamic_call_declared_target_is_inline() {
    let instruction = Instruction::CallVirtual {
        destination: None,
        receiver: value_reference(Value::new(0)),
        call: Call::new(
            ArgumentSlice::new(0, 0),
            type_reference(LocalNodeId::new(0)),
        ),
        declaring_type: type_reference(LocalNodeId::new(0)),
        slot_id: VtableSlotId::new(0),
        declared_target: Some(function_reference(LocalNodeId::new(1))),
    };

    assert_eq!(
        instruction.call_declared_target(),
        Some(function_reference(LocalNodeId::new(1)))
    );
}

/// Reject managed allocation instructions when noManaged is required.
#[test]
fn test_reject_new_with_no_managed_mode() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("noManaged");

    let void_ty = tree.insert_type(Type::Void);
    let layout_ty = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let result_ty = tree.insert_type(Type::Reference {
        kind: ReferenceKind::Managed,
        address_space: AddressSpace::Local,
        mutability: Mutability::Mutable,
        pointee: type_reference(layout_ty),
        is_nullable: false,
    });
    let destination = Value::new(0);
    let instruction = tree.insert(Instruction::New {
        destination: value_reference(destination),
        layout: type_reference(layout_ty),
        result_type: type_reference(result_ty),
    });
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(void_ty), block_id);
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
        "metadata invariant violation: allocation mode violation: 'new' is invalid because noManaged forbids managed allocations"
    );
}

/// Reject raw heap allocations when stackOnly is required.
#[test]
fn test_reject_raw_alloc_with_stack_only_mode() {
    let mut tree = Tree::new();
    let pool = StringPool::new();
    let name = pool.intern("stackOnly");

    let void_ty = tree.insert_type(Type::Void);
    let layout_ty = tree.insert_type(Type::Int {
        width: 64,
        is_signed: true,
    });
    let result_ty = tree.insert_type(Type::Reference {
        kind: ReferenceKind::Raw,
        address_space: AddressSpace::Local,
        mutability: Mutability::Mutable,
        pointee: type_reference(layout_ty),
        is_nullable: false,
    });
    let destination = Value::new(0);
    let instruction = tree.insert(Instruction::RawAlloc {
        destination: value_reference(destination),
        layout: type_reference(layout_ty),
        result_type: type_reference(result_ty),
    });
    let block = Block {
        name: None,
        parameters: Vec::new(),
        instructions: vec![instruction],
        terminator: tree.insert(Terminator::Return { value: None }),
    };
    let block_id = tree.insert(block);

    let mut function = Function::local(name, Vec::new(), type_reference(void_ty), block_id);
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

/// Reject raw allocations that pretend to return owned references.
#[test]
fn test_reject_raw_alloc_with_owned_reference() {
    let source = r#"function bad(): void {
entry0():
    value0: ref<int32, owned> = raw.alloc int32
    return
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: pointer-producing instruction result type has wrong reference kind"
    );
}

/// Reject heap allocations that pretend to return raw references.
#[test]
fn test_reject_new_with_raw_reference() {
    let source = r#"function bad(): void {
entry0():
    value0: ref<int32, raw> = new int32
    return
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: new result type has wrong reference kind"
    );
}

/// Reject heap array allocations with non integer lengths.
#[test]
fn test_reject_new_slice_with_non_integer_length() {
    let source = r#"function bad(value0: float32): void {
entry0(value0: float32):
    value1: slice<int32> = new.slice int32, value0
    return
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: new.slice length must be an integer type"
    );
}

/// Reject pinning non-local or non-heap references.
#[test]
fn test_reject_pin_for_raw_reference() {
    let source = r#"function bad(value0: ref<int32, raw>): void {
entry0(value0: ref<int32, raw>):
    pin value0
    return
}"#;

    let error = assert_validate_error(source);
    assert_eq!(
        error.message,
        "metadata invariant violation: pin value must be one local heap reference"
    );
}

/// Accept pinning local owned references.
#[test]
fn test_accept_pin_for_owned_reference() {
    let source = r#"function good(value0: ref<int32, owned>): void {
entry0(value0: ref<int32, owned>):
    pin value0
    unpin value0
    return
}"#;

    assert_validate_ok(source);
}
