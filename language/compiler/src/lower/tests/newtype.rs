use destack_engine::Value;

use crate::TestProgram;

/// Lower newtype declarations into MIR newtype wrappers.
#[test]
fn test_lower_newtype_user_id_signature() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype UserId = int32;

function loadUser(id: UserId): UserId {
    return id;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type UserId = newtype<int32>;

function loadUser(value0: UserId): UserId {
entry0(value0: UserId):
    return value0
}
"#,
    );
}

/// Lower newtypes distinctly from type aliases.
#[test]
fn test_lower_newtype_distinct_from_alias() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
type RepositoryId = int32;
newtype UserId = int32;

function lookupRepository(id: RepositoryId): RepositoryId {
    return id;
}

function lookupUser(id: UserId): UserId {
    return id;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type UserId = newtype<int32>;

function lookupRepository(value0: int32): int32 {
entry0(value0: int32):
    return value0
}

function lookupUser(value0: UserId): UserId {
entry0(value0: UserId):
    return value0
}
"#,
    );
}

/// Lower tuple newtypes to MIR newtype wrappers.
#[test]
fn test_lower_newtype_tuple_payload() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype Range = (int32, int32);

function normalizeRange(value: Range): Range {
    return value;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type Range#1 = (int32, int32);
type Range = newtype<Range#1>;

function normalizeRange(value0: Range): Range {
entry0(value0: Range):
    return value0
}
"#,
    );
}

/// Lower struct-backed newtypes to MIR newtype wrappers.
#[test]
fn test_lower_newtype_struct_payload() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point { x: int32; y: int32 }
newtype Location = Point;

function markLocation(value: Location): Location {
    return value;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type Point {
    x: int32;
    y: int32;
}
type Location = newtype<Point>;

function markLocation(value0: Location): Location {
entry0(value0: Location):
    return value0
}
"#,
    );
}

/// Lower scalar newtype constructors into a cast.bit.
#[test]
fn test_lower_newtype_user_id_constructor() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype UserId = int32;

function makeUserId(value: int32): UserId {
    return UserId(value);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type UserId = newtype<int32>;

function makeUserId(value0: int32): UserId {
entry0(value0: int32):
    value1: UserId = cast.bit value0 -> UserId
    return value1
}
"#,
    );
}

/// Lower tuple newtype constructors into tuple payloads.
#[test]
fn test_lower_newtype_range_constructor() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype Range = (int32, int32);

function makeRange(start: int32, end: int32): Range {
    return Range(start, end);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type Range#1 = (int32, int32);
type Range = newtype<Range#1>;

function makeRange(value0: int32, value1: int32): Range {
entry0(value0: int32, value1: int32):
    value2: Range#1 = tuple Range#1 (value0, value1)
    value3: Range = cast.bit value2 -> Range
    return value3
}
"#,
    );
}

/// Execute scalar newtype matches with literal patterns.
#[test]
fn test_lower_newtype_match_status_code() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype StatusCode = int32;

function classifyStatus(value: int32): int32 {
    let status: StatusCode = StatusCode(value);
    let result: int32 = 0;
    match (status) {
        StatusCode(200) => {
            result = 1;
        }
        _ => {
            result = 0;
        }
    }
    return result;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "classifyStatus",
        &[Value::int32(200)],
        Value::int32(1),
    );
    test.assert_mir_function_output(
        module_id,
        "native",
        "classifyStatus",
        &[Value::int32(404)],
        Value::int32(0),
    );
}

/// Execute scalar newtype matches that bind inner values.
#[test]
fn test_lower_newtype_match_port_binding() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype Port = int32;

function readPort(value: int32): int32 {
    let port: Port = Port(value);
    let result: int32 = 0;
    match (port) {
        Port(inner) => {
            result = inner + 1;
        }
        _ => {
            result = 0;
        }
    }
    return result;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "readPort",
        &[Value::int32(3000)],
        Value::int32(3001),
    );
}

/// Execute tuple newtype matches with bound fields.
#[test]
fn test_lower_newtype_match_range_tuple() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype Range = (int32, int32);

function rangeWidth(start: int32, end: int32): int32 {
    let range: Range = Range(start, end);
    let result: int32 = 0;
    match (range) {
        Range(lower, upper) => {
            result = upper - lower;
        }
        _ => {
            result = 0;
        }
    }
    return result;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "rangeWidth",
        &[Value::int32(10), Value::int32(18)],
        Value::int32(8),
    );
}

/// Lower newtypes embedded in struct fields.
#[test]
fn test_lower_newtype_struct_field() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype UserId = int32;
struct User {
    id: UserId;
    flags: int32
}

function readUserId(user: User): UserId {
    return user.id;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type UserId = newtype<int32>;
type User {
    id: UserId;
    flags: int32;
}

function readUserId(value0: User): UserId {
entry0(value0: User):
    value1: UserId = field.get value0, 0
    return value1
}
"#,
    );
}
