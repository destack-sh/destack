use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_type_parameter_preserves_scalar_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<const T>(value: T): T;

const value = id("ready");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<const T>(value: T): T;

const value: "ready" = id<"ready">("ready");

=== checked ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=[const T]
/// @type.symbol symbol=id type=<const T>(T) => T
/// @type.symbol symbol=value type=T

const value = id("ready");
/// @type.symbol symbol=value type="ready"
/// @resolution.name source=id target=id
/// @resolution.call source="id(\"ready\")" parameters=("ready") return="ready" kind=symbol target=id instance="id<\"ready\">"
/// @generic.instance source="id(\"ready\")" id="id<\"ready\">"
/// @generic.instance id="id<\"ready\">" symbol=id arguments=["ready"]
"#,
    );
}

#[test]
fn test_const_type_parameter_preserves_array_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<const T>(value: T): T;

const values = id([1, 2]);
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<const T>(value: T): T;

const values: readonly [1, 2] = id<readonly [1, 2]>([1, 2]);
const first: 1 = values[0];

=== checked ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=[const T]
/// @type.symbol symbol=id type=<const T>(T) => T
/// @type.symbol symbol=value type=T

const values = id([1, 2]);
/// @type.symbol symbol=values type=readonly [1, 2]
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(readonly [1, 2]) return=readonly [1, 2] kind=symbol target=id instance="id<readonly [1, 2]>"
/// @generic.instance source="id([1, 2])" id="id<readonly [1, 2]>"

const first = values[0];
/// @type.symbol symbol=first type=1
/// @resolution.name source=values target=values
/// @resolution.member source=values[0] receiver=readonly [1, 2] kind=builtin builtin=subscript.index
/// @generic.instance id="id<readonly [1, 2]>" symbol=id arguments=[readonly [1, 2]]
"#,
    );
}

#[test]
fn test_plain_type_parameter_widens_array_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;

const values = id([1, 2]);
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const values: float64[] = id<float64[]>([1, 2]);
const first: float64 = values[0];

=== checked ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=[T]
/// @type.symbol symbol=id type=<T>(T) => T
/// @type.symbol symbol=value type=T

const values = id([1, 2]);
/// @type.symbol symbol=values type=float64[]
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(float64[]) return=float64[] kind=symbol target=id instance="id<float64[]>"
/// @generic.instance source="id([1, 2])" id="id<float64[]>"

const first = values[0];
/// @type.symbol symbol=first type=float64
/// @resolution.name source=values target=values
/// @resolution.member source=values[0] receiver=float64[] kind=builtin builtin=subscript.index
/// @generic.instance id="id<float64[]>" symbol=id arguments=[float64[]]
"#,
    );
}

#[test]
fn test_const_type_parameter_preserves_object_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<const T>(value: T): T;

const value = id({ kind: "ready", level: 1 });
const kind = value.kind;
const level = value.level;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<const T>(value: T): T;

const value: { kind: "ready"; level: 1 } = id<{ kind: "ready"; level: 1 }>({ kind: "ready", level: 1 });
const kind: "ready" = value.kind;
const level: 1 = value.level;

=== checked ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=[const T]
/// @type.symbol symbol=id type=<const T>(T) => T
/// @type.symbol symbol=value type=T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value type={ kind: "ready"; level: 1 }
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=({ kind: "ready"; level: 1 }) return={ kind: "ready"; level: 1 } kind=symbol target=id instance="id<{ kind: \"ready\"; level: 1 }>"
/// @generic.instance source="id({ kind: \"ready\", level: 1 })" id="id<{ kind: \"ready\"; level: 1 }>"

const kind = value.kind;
/// @type.symbol symbol=kind type="ready"
/// @resolution.name source=value target=value
/// @resolution.member source=value.kind receiver={ kind: "ready"; level: 1 } kind=field key=kind

const level = value.level;
/// @type.symbol symbol=level type=1
/// @resolution.name source=value target=value
/// @resolution.member source=value.level receiver={ kind: "ready"; level: 1 } kind=field key=level
/// @generic.instance id="id<{ kind: \"ready\"; level: 1 }>" symbol=id arguments=[{ kind: "ready"; level: 1 }]
"#,
    );
}

#[test]
fn test_plain_type_parameter_widens_object_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;

const value = id({ kind: "ready", level: 1 });
const kind = value.kind;
const level = value.level;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const value: { kind: string; level: float64 } = id<{ kind: string; level: float64 }>({
    kind: "ready",
    level: 1,
});
const kind: string = value.kind;
const level: float64 = value.level;

=== checked ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=[T]
/// @type.symbol symbol=id type=<T>(T) => T
/// @type.symbol symbol=value type=T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value type={ kind: string; level: float64 }
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=({ kind: string; level: float64 }) return={ kind: string; level: float64 } kind=symbol target=id instance="id<{ kind: string; level: float64 }>"
/// @generic.instance source="id({ kind: \"ready\", level: 1 })" id="id<{ kind: string; level: float64 }>"

const kind = value.kind;
/// @type.symbol symbol=kind type=string
/// @resolution.name source=value target=value
/// @resolution.member source=value.kind receiver={ kind: string; level: float64 } kind=field key=kind

const level = value.level;
/// @type.symbol symbol=level type=float64
/// @resolution.name source=value target=value
/// @resolution.member source=value.level receiver={ kind: string; level: float64 } kind=field key=level
/// @generic.instance id="id<{ kind: string; level: float64 }>" symbol=id arguments=[{ kind: string; level: float64 }]
"#,
    );
}
