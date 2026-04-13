use super::{assert_format, assert_format_eq};

/// Formats declarations with dotted metadata names canonically.
#[test]
fn test_format_dotted_symbol_names() {
    assert_format_eq(
        r#"
type Status = newtype<int32>

global Status.Default: Status, readonly = 1int32

function Status.isActive(v0: Status): boolean {
b0(v0: Status):
    v1: int32 = cast.bit v0 -> int32
    v2: int32 = 1int32
    v3: boolean = int.eq v1, v2
    return v3
}

function checkDefault(): boolean {
b0:
    v0: Status = global.const Status.Default
    v1: boolean = call Status.isActive(v0): (Status) -> boolean
    return v1
}
"#,
        r#"
type Status = newtype<int32>;

global Status.Default: Status, readonly = 1int32

function Status.isActive(value0: Status): boolean {
entry0(value0: Status):
    value1: int32 = cast.bit value0 -> int32
    value2: int32 = 1int32
    value3: boolean = int.eq value1, value2
    return value3
}

function checkDefault(): boolean {
entry0:
    value0: Status = global.const Status.Default
    value1: boolean = call Status.isActive(value0): (Status) -> boolean
    return value1
}
"#,
    );
}

/// Formats callable typed declarations canonically.
#[test]
fn test_format_callable_type_declaration() {
    assert_format_eq(
        r#"
type Callable = (int32) => int32

function use(v0: Callable): Callable {
b0(v0: Callable):
    return v0
}
"#,
        r#"
type Callable = (int32) => int32;

function use(value0: Callable): Callable {
entry0(value0: Callable):
    return value0
}
"#,
    );
}

/// Formats imports and exports across declaration kinds canonically.
#[test]
fn test_format_import_export_declarations() {
    assert_format(
        r#"
extern global Imported: int32, readonly

export global Exported: int32 = 7int32

extern function imported(int32): int32

export function exported(value0: int32): int32 {
entry0(value0: int32):
    value1: int32 = call imported(value0): (int32) -> int32
    return value1
}
"#,
    );
}
