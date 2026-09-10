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
    v3: boolean = eq v1, v2
    return v3
}

function checkDefault(): boolean {
entry:
    v0: ref<Status, borrowed, 'static & local, readonly> = global.address Status.Default
    v1: Status = load v0
    v2: boolean = call Status.isActive(v1): (Status) => boolean
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
    v3: boolean = eq v1, v2
    return v3
}

function checkDefault(): boolean {
entry:
    v0: ref<Status, borrowed, 'static & local, readonly> = global.address Status.Default
    v1: Status = load v0
    v2: boolean = call Status.isActive(v1): (Status) => boolean
    return v2
}
"#,
    );
}

/// Formats function signature declarations canonically.
#[test]
fn test_format_callable_type_declaration() {
    assert_format(
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

@binding("runtime.touch", { provider: "runtime", effect: "pure", affinity: "worker" })
external function touch(): void

export function exported(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call imported(v0): (int32) => int32
    return v1
}
"#,
    );
}

/// Formats constrained binding metadata as one TS-style object.
#[test]
fn test_format_binding_declaration() {
    assert_format_eq(
        r#"
@binding("host.fs.open", { provider: "host", effect: "external", replay: "forbidden", affinity: "main", requires: ["host.fs.open"], platforms: ["linux"], families: ["unix"], hosts: ["native"] })
external function open(): void
"#,
        r#"
@binding("host.fs.open", {
    provider: "host",
    effect: "external",
    replay: "forbidden",
    affinity: "main",
    requires: ["host.fs.open"],
    platforms: ["linux"],
    families: ["unix"],
    hosts: ["native"]
})
external function open(): void
"#,
    );
}
