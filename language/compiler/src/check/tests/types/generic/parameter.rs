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
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @type.symbol symbol=value#1 source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id("ready");
/// @type.symbol symbol=value#2 source=value type="ready"
/// @resolution.name source=id target=id
/// @resolution.call source="id(\"ready\")" parameters=("ready") return="ready" kind=symbol target=id instance="id<\"ready\">"
/// @generic.instance source="id(\"ready\")" id="id<\"ready\">"
/// @generic.instance id="id<\"ready\">" template=id arguments=("ready")
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
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const values = id([1, 2]);
/// @type.symbol symbol=values source=values type=readonly [1, 2]
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(readonly [1, 2]) return=readonly [1, 2] kind=symbol target=id instance="id<readonly [1, 2]>"
/// @generic.instance source="id([1, 2])" id="id<readonly [1, 2]>"

const first = values[0];
/// @type.symbol symbol=first source=first type=1
/// @resolution.name source=values target=values
/// @resolution.member source=values[0] receiver=readonly [1, 2] kind=element index=0
/// @generic.instance id="id<readonly [1, 2]>" template=id arguments=(readonly [1, 2])
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
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const values = id([1, 2]);
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(Array<float64>) return=Array<float64> kind=symbol target=id instance="id<Array<float64>>"
/// @generic.instance source="id([1, 2])" id="id<Array<float64>>"

const first = values[0];
/// @type.symbol symbol=first source=first type=float64
/// @resolution.name source=values target=values
/// @resolution.call source=values[0] parameters=(usize) return=float64 kind=symbol target=collections.array.index#8 receiver=Array<float64>
/// @generic.instance id="id<Array<float64>>" template=id arguments=(Array<float64>)
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

const value: { readonly kind: "ready"; readonly level: 1 } = id<{
    readonly kind: "ready";
    readonly level: 1;
}>({ kind: "ready", level: 1 });
const kind: "ready" = value.kind;
const level: 1 = value.level;

=== checked ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @type.symbol symbol=value#1 source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value#2 source=value type=Managed<{ readonly kind: "ready"; readonly level: 1 }>
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=(Managed<{ readonly kind: "ready"; readonly level: 1 }>) return=Managed<{ readonly kind: "ready"; readonly level: 1 }> kind=symbol target=id instance="id<Managed<{ readonly kind: \"ready\"; readonly level: 1 }>>"
/// @generic.instance source="id({ kind: \"ready\", level: 1 })" id="id<Managed<{ readonly kind: \"ready\"; readonly level: 1 }>>"

const kind = value.kind;
/// @type.symbol symbol=kind source=kind type="ready"
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.kind receiver=Managed<{ readonly kind: "ready"; readonly level: 1 }> kind=field key=kind

const level = value.level;
/// @type.symbol symbol=level source=level type=1
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.level receiver=Managed<{ readonly kind: "ready"; readonly level: 1 }> kind=field key=level
/// @generic.instance id="id<Managed<{ readonly kind: \"ready\"; readonly level: 1 }>>" template=id arguments=(Managed<{ readonly kind: "ready"; readonly level: 1 }>)
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
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @type.symbol symbol=value#1 source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value#2 source=value type=Managed<{ kind: string; level: float64 }>
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=(Managed<{ kind: string; level: float64 }>) return=Managed<{ kind: string; level: float64 }> kind=symbol target=id instance="id<Managed<{ kind: string; level: float64 }>>"
/// @generic.instance source="id({ kind: \"ready\", level: 1 })" id="id<Managed<{ kind: string; level: float64 }>>"

const kind = value.kind;
/// @type.symbol symbol=kind source=kind type=string
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.kind receiver=Managed<{ kind: string; level: float64 }> kind=field key=kind

const level = value.level;
/// @type.symbol symbol=level source=level type=float64
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.level receiver=Managed<{ kind: string; level: float64 }> kind=field key=level
/// @generic.instance id="id<Managed<{ kind: string; level: float64 }>>" template=id arguments=(Managed<{ kind: string; level: float64 }>)
"#,
    );
}

#[test]
fn test_recursive_constraint_member_lookup_reports_missing_member() {
    let session = TestSession::single(
        r#"
function read<T: T | { name: string }>(value: T): string {
    return value.name;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function read<T: T | { name: string }>(value: T): string {
    return value.name;
}

=== checked ===
function read<T: T | { name: string }>(value: T): string {
/// @generic.template symbol=read parameters=(T: T | { name: string })
/// @type.symbol symbol=read type=<T: T | { name: string }>(T) => string
/// @type.symbol symbol=read.T source="T: T | { name: string }" type=T
/// @resolution.name source=T target=read.T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=read.T

    return value.name;
    /// @resolution.name source=value target=value

}
"#,
        r#"
/// @diagnostic.error code=EC300 message="member 'name' does not exist on type 'T'"
/// @diagnostic.label line=3 column=12 source="return value.name;"
"#,
    );
}
