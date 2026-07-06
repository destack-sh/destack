use super::{assert_format, assert_format_eq};

/// Formats declarations with dotted names canonically.
#[test]
fn test_format_dotted_symbol_names() {
    assert_format_eq(
        r#"
type Status = newtype<int32>;

readonly global Status.Default: Status = 1

function Status.isActive(v0: Status): boolean {
entry(v0: Status):
    v1: int32 = cast.bit v0 -> int32
    v2: int32 = 1
    v3: boolean = int.eq v1, v2
    return v3
}

function checkDefault(): boolean {
entry:
    v0: ref<Status, raw, readonly> = global.address Status.Default
    v1: Status = load v0
    v2: boolean = call Status.isActive(v1)
    return v2
}
"#,
        r#"
type Status = newtype<int32>;

readonly global Status.Default: Status = 1

function Status.isActive(v0: Status): boolean {
entry(v0: Status):
    v1: int32 = cast.bit v0 -> int32
    v2: int32 = 1
    v3: boolean = int.eq v1, v2
    return v3
}

function checkDefault(): boolean {
entry:
    v0: ref<Status, raw, readonly> = global.address Status.Default
    v1: Status = load v0
    v2: boolean = call Status.isActive(v1)
    return v2
}
"#,
    );
}

/// Formats callable typed declarations canonically.
#[test]
fn test_format_callable_type_declaration() {
    assert_format_eq(
        r#"
type Callable = (int32) => int32;

function use(v0: Callable): Callable {
entry(v0: Callable):
    return v0
}
"#,
        r#"
type Callable = (int32) => int32;

function use(v0: Callable): Callable {
entry(v0: Callable):
    return v0
}
"#,
    );
}

/// Formats imports and exports across declaration kinds canonically.
#[test]
fn test_format_import_export_declarations() {
    assert_format(
        r#"
external readonly global Imported: int32

export global Exported: int32 = 7

external function imported(int32): int32

@binding("runtime.touch")
external function touch(): void

export function exported(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call imported(v0)
    return v1
}
"#,
    );
}
