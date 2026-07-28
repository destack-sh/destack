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
/// @type.symbol symbol=id.value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id("ready");
/// @type.symbol symbol=value source=value type="ready"
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=id target=id
/// @resolution.call source="id(\"ready\")" parameters=("ready") arguments=(provided("ready") as "ready") return="ready" kind=symbol target=id instance="id<\"ready\">"
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

const values: [1, 2] = id<[1, 2]>([1, 2]);
const first: 1 = values[0];

=== checked ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @type.symbol symbol=id.value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const values = id([1, 2]);
/// @type.symbol symbol=values source=values type=[1, 2]
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=([1, 2]) arguments=(provided([1, 2]) as [1, 2]) return=[1, 2] kind=symbol target=id instance="id<[1, 2]>"
/// @generic.instance source="id([1, 2])" id="id<[1, 2]>"

const first = values[0];
/// @type.symbol symbol=first source=first type=1
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=1 kind=member target="receiver=[1, 2], target=field(receiver=[1, 2], target=0, type=1), type=1"

/// @generic.instance id="id<[1, 2]>" template=id arguments=([1, 2])
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
/// @type.symbol symbol=id.value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const values = id([1, 2]);
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(Array<float64>) arguments=(provided([1, 2]) as Array<float64>) return=Array<float64> kind=symbol target=id instance=id<Array<float64>>
/// @generic.instance source="id([1, 2])" id=id<Array<float64>>

const first = values[0];
/// @type.symbol symbol=first source=first type=float64
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=float64 kind=call target="collections.array.index#3(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'static float64, \"exclusive\">)"
/// @generic.instance source=values[0] id="Array<float64>.<extension#6>.index#3<\"exclusive\">"

/// @generic.instance id="Array<float64>.<extension#6>.index#3<\"exclusive\">" template=collections.array.index#3 arguments=(float64, "exclusive")
/// @generic.instance id=id<Array<float64>> template=id arguments=(Array<float64>)
"#,
    );
}

#[test]
fn test_mutable_array_alias_does_not_widen_element_type() {
    let session = TestSession::single(
        r#"
declare function take(values: float64[]): void;
declare const values: (1 | 2)[];

take(values);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(values: float64[]): void;
declare const values: (1 | 2)[];

take(values);

=== checked ===
declare function take(values: float64[]): void;
/// @type.symbol symbol=take source="declare function take(values: float64[]): void" type=(Array<float64>) => void
/// @type.symbol symbol=take.values source="values: float64[]" type=Array<float64>

declare const values: (1 | 2)[];
/// @type.symbol symbol=values source=values type=Array<1 | 2>
/// @resolution.pattern source=values kind=binding target=values

take(values);
/// @resolution.name source=take target=take
/// @resolution.call source=take(values) parameters=(Array<float64>) arguments=(provided(values) as Array<float64>) return=void kind=symbol target=take
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'Array<1 | 2>' is not assignable to parameter of type 'Array<float64>'"
/// @diagnostic.label line=5 column=6 span="values" line_source="take(values);"
/// @diagnostic.related line=5 column=1 span="take(values)" line_source="take(values);" message="in this call"
/// @diagnostic.note message="the mismatch is in the element type: expected 'float64', found '1 | 2'"
"#,
    );
}

#[test]
fn test_array_literal_materializes_as_slice_parameter() {
    let session = TestSession::single(
        r#"
declare function take(values: Slice<float64>): void;

take([1, 2]);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(values: Slice<float64>): void;

take([1, 2] as [float64]);

=== checked ===
declare function take(values: Slice<float64>): void;
/// @type.symbol symbol=take source="declare function take(values: Slice<float64>): void" type=(Slice<float64>) => void
/// @type.symbol symbol=take.values source="values: Slice<float64>" type=Slice<float64>
/// @resolution.name source=Slice target=collections.slice.Slice

take([1, 2]);
/// @resolution.name source=take target=take
/// @resolution.call source="take([1, 2])" parameters=(Slice<float64>) arguments=(provided([1, 2]) as Slice<float64>) return=void kind=symbol target=take

/// @generic.instance id=Slice<float64> template=collections.slice.Slice arguments=(float64)
"#,
    );
}

#[test]
fn test_array_literal_materializes_as_fixed_array_parameter() {
    let session = TestSession::single(
        r#"
declare function take(values: [float64; 2]): void;

take([1, 2]);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(values: [float64; 2]): void;

take([1, 2]);

=== checked ===
declare function take(values: [float64; 2]): void;
/// @type.symbol symbol=take source="declare function take(values: [float64; 2]): void" type=(FixedArray<float64, 2>) => void
/// @type.symbol symbol=take.values source="values: [float64; 2]" type=FixedArray<float64, 2>

take([1, 2]);
/// @resolution.name source=take target=take
/// @resolution.call source="take([1, 2])" parameters=(FixedArray<float64, 2>) arguments=(provided([1, 2]) as FixedArray<float64, 2>) return=void kind=symbol target=take
"#,
    );
}

#[test]
fn test_array_literal_rejects_mismatched_fixed_array_parameter_length() {
    let session = TestSession::single(
        r#"
declare function take(values: [float64; 2]): void;

take([1, 2, 3]);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(values: [float64; 2]): void;

take([1, 2, 3]);

=== checked ===
declare function take(values: [float64; 2]): void;
/// @type.symbol symbol=take source="declare function take(values: [float64; 2]): void" type=(FixedArray<float64, 2>) => void
/// @type.symbol symbol=take.values source="values: [float64; 2]" type=FixedArray<float64, 2>

take([1, 2, 3]);
/// @resolution.name source=take target=take
/// @resolution.call source="take([1, 2, 3])" parameters=(FixedArray<float64, 2>) arguments=(provided([1, 2, 3]) as FixedArray<float64, 2>) return=void kind=symbol target=take
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'FixedArray<float64, 3>' is not assignable to parameter of type 'FixedArray<float64, 2>'"
/// @diagnostic.label line=4 column=6 span="[1, 2, 3]" line_source="take([1, 2, 3]);"
/// @diagnostic.related line=4 column=1 span="take([1, 2, 3])" line_source="take([1, 2, 3]);" message="in this call"
/// @diagnostic.note message="the mismatch is in the length: expected '2', found '3'"
"#,
    );
}

#[test]
fn test_plain_type_parameter_widens_tuple_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;

const value = id((1, "x"));
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const value: (float64, string) = id<(float64, string)>((1, "x"));

=== checked ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @type.symbol symbol=id.value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id((1, "x"));
/// @type.symbol symbol=value source=value type=(float64, string)
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=id target=id
/// @resolution.call source="id((1, \"x\"))" parameters=((float64, string)) arguments=(provided((1, "x")) as (float64, string)) return=(float64, string) kind=symbol target=id instance="id<(float64, string)>"
/// @generic.instance source="id((1, \"x\"))" id="id<(float64, string)>"

/// @generic.instance id="id<(float64, string)>" template=id arguments=((float64, string))
"#,
    );
}

#[test]
fn test_const_type_parameter_preserves_nested_object_literal_precision() {
    let session = TestSession::single(
        r#"
declare function collect<const T>(values: T[]): T[];

const values = collect([{ kind: "ready" }]);
const kind = values[0].kind;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function collect<const T>(values: T[]): T[];

const values: { readonly kind: "ready" }[] = collect<{ readonly kind: "ready" }>([
    { kind: "ready" },
]);
const kind: "ready" = values[0].kind;

=== checked ===
declare function collect<const T>(values: T[]): T[];
/// @generic.template symbol=collect parameters=(const T)
/// @type.symbol symbol=collect source="declare function collect<const T>(values: T[]): T[]" type=<const T>(Array<T>) => Array<T>
/// @type.symbol symbol=collect.T source="const T" type=T
/// @type.symbol symbol=collect.values source="values: T[]" type=Array<T>
/// @resolution.name source=T target=collect.T
/// @resolution.name source=T target=collect.T

const values = collect([{ kind: "ready" }]);
/// @type.symbol symbol=values source=values type=Array<{ readonly kind: "ready" }>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=collect target=collect
/// @resolution.call source="collect([{ kind: \"ready\" }])" parameters=(Array<{ readonly kind: "ready" }>) arguments=(provided([{ kind: "ready" }]) as Array<{ readonly kind: "ready" }>) return=Array<{ readonly kind: "ready" }> kind=symbol target=collect instance="collect<{ readonly kind: \"ready\" }>"
/// @generic.instance source="collect([{ kind: \"ready\" }])" id="collect<{ readonly kind: \"ready\" }>"

const kind = values[0].kind;
/// @type.symbol symbol=kind source=kind type="ready"
/// @resolution.pattern source=kind kind=binding target=kind
/// @resolution.name source=values target=values
/// @resolution.member source=values[0].kind receiver={ readonly kind: "ready" } type="ready" kind=field target_receiver={ readonly kind: "ready" } key=kind target_type="ready"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.place source=values[0] placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type={ readonly kind: "ready" } kind=call target="collections.array.index#3(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'static { readonly kind: \"ready\" }, \"exclusive\">)"
/// @resolution.access source=values[0].kind root=values keys=[0, kind]
/// @generic.instance source=values[0] id="Array<{ readonly kind: \"ready\" }>.<extension#6>.index#3<\"exclusive\">"

/// @generic.instance id="Array<{ readonly kind: \"ready\" }>.<extension#6>.index#3<\"exclusive\">" template=collections.array.index#3 arguments=({ readonly kind: "ready" }, "exclusive")
/// @generic.instance id="collect<{ readonly kind: \"ready\" }>" template=collect arguments=({ readonly kind: "ready" })
"#,
    );
}

#[test]
fn test_const_type_parameter_preserves_literal_precision_through_union() {
    let session = TestSession::single(
        r#"
declare function maybe<const T>(value: T | undefined): T | undefined;

const value = maybe({ kind: "ready" });
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function maybe<const T>(value: T | undefined): T | undefined;

const value: { readonly kind: "ready" } | undefined = maybe<{ readonly kind: "ready" }>({
    kind: "ready",
} as { readonly kind: "ready" } | undefined);

=== checked ===
declare function maybe<const T>(value: T | undefined): T | undefined;
/// @generic.template symbol=maybe parameters=(const T)
/// @type.symbol symbol=maybe source="declare function maybe<const T>(value: T | undefined): T | undefined" type=<const T>(T | undefined) => T | undefined
/// @type.symbol symbol=maybe.T source="const T" type=T
/// @type.symbol symbol=maybe.value source="value: T | undefined" type=T | undefined
/// @resolution.name source=T target=maybe.T
/// @resolution.name source=T target=maybe.T

const value = maybe({ kind: "ready" });
/// @type.symbol symbol=value source=value type={ readonly kind: "ready" } | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=maybe target=maybe
/// @resolution.call source="maybe({ kind: \"ready\" })" parameters=({ readonly kind: "ready" } | undefined) arguments=(provided({ kind: "ready" }) as { readonly kind: "ready" } | undefined) return={ readonly kind: "ready" } | undefined kind=symbol target=maybe instance="maybe<{ readonly kind: \"ready\" }>"
/// @generic.instance source="maybe({ kind: \"ready\" })" id="maybe<{ readonly kind: \"ready\" }>"

/// @generic.instance id="maybe<{ readonly kind: \"ready\" }>" template=maybe arguments=({ readonly kind: "ready" })
"#);
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
/// @type.symbol symbol=id.value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value source=value type={ readonly kind: "ready"; readonly level: 1 }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=({ readonly kind: "ready"; readonly level: 1 }) arguments=(provided({ kind: "ready", level: 1 }) as { readonly kind: "ready"; readonly level: 1 }) return={ readonly kind: "ready"; readonly level: 1 } kind=symbol target=id instance="id<{ readonly kind: \"ready\"; readonly level: 1 }>"
/// @generic.instance source="id({ kind: \"ready\", level: 1 })" id="id<{ readonly kind: \"ready\"; readonly level: 1 }>"

const kind = value.kind;
/// @type.symbol symbol=kind source=kind type="ready"
/// @resolution.pattern source=kind kind=binding target=kind
/// @resolution.name source=value target=value
/// @resolution.member source=value.kind receiver={ readonly kind: "ready"; readonly level: 1 } type="ready" kind=field target_receiver={ readonly kind: "ready"; readonly level: 1 } key=kind target_type="ready"
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.access source=value.kind root=value keys=[kind]

const level = value.level;
/// @type.symbol symbol=level source=level type=1
/// @resolution.pattern source=level kind=binding target=level
/// @resolution.name source=value target=value
/// @resolution.member source=value.level receiver={ readonly kind: "ready"; readonly level: 1 } type=1 kind=field target_receiver={ readonly kind: "ready"; readonly level: 1 } key=level target_type=1
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.access source=value.level root=value keys=[level]

/// @generic.instance id="id<{ readonly kind: \"ready\"; readonly level: 1 }>" template=id arguments=({ readonly kind: "ready"; readonly level: 1 })
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
/// @type.symbol symbol=id.value source="value: T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value source=value type={ kind: string; level: float64 }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=({ kind: string; level: float64 }) arguments=(provided({ kind: "ready", level: 1 }) as { kind: string; level: float64 }) return={ kind: string; level: float64 } kind=symbol target=id instance="id<{ kind: string; level: float64 }>"
/// @generic.instance source="id({ kind: \"ready\", level: 1 })" id="id<{ kind: string; level: float64 }>"

const kind = value.kind;
/// @type.symbol symbol=kind source=kind type=string
/// @resolution.pattern source=kind kind=binding target=kind
/// @resolution.name source=value target=value
/// @resolution.member source=value.kind receiver={ kind: string; level: float64 } type=string kind=field target_receiver={ kind: string; level: float64 } key=kind target_type=string
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.access source=value.kind root=value keys=[kind]

const level = value.level;
/// @type.symbol symbol=level source=level type=float64
/// @resolution.pattern source=level kind=binding target=level
/// @resolution.name source=value target=value
/// @resolution.member source=value.level receiver={ kind: string; level: float64 } type=float64 kind=field target_receiver={ kind: string; level: float64 } key=level target_type=float64
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.access source=value.level root=value keys=[level]

/// @generic.instance id="id<{ kind: string; level: float64 }>" template=id arguments=({ kind: string; level: float64 })
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
/// @type.symbol symbol=read.value source="value: T" type=T
/// @resolution.name source=T target=read.T

    return value.name;
    /// @resolution.name source=value target=read.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=read.value

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'name' does not exist on type 'T'"
/// @diagnostic.label line=3 column=18 span="name" line_source="return value.name;"
"#,
    );
}

#[test]
fn test_parameter_satisfies_its_own_declared_bound() {
    let session = TestSession::single(
        r#"
interface Equal<T> {
    equals(other: T): boolean;
}

class Bucket<K: Equal<K>> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }

    pair(): Bucket<K> {
        return new Bucket<K>(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Equal<in T> {
    equals(other: T): boolean;
}

class Bucket<in out K: Equal<K>> {
    key: K;

    constructor(key: K): this {
        this.key = key;
    }

    pair(): Bucket<K> {
        return new Bucket<K>(this.key);
    }
}

=== checked ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(in T)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(in T)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(this: this, T) => boolean
/// @type.symbol symbol=Equal.T source=T type=T

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(this: this, T) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T
    /// @resolution.name source=T target=Equal.T

}

class Bucket<K: Equal<K>> {
/// @generic.template symbol=Bucket parameters=(in out K: Equal<K>)
/// @type.symbol symbol=Bucket type=Bucket
/// @definition.class symbol=Bucket template=(in out K: Equal<K>)
/// @definition.field symbol=Bucket.key source="key: K" key=key type=K
/// @definition.method symbol=Bucket.constructor slot=constructor role=constructor type=(K) => this
/// @definition.method symbol=Bucket.pair slot=pair type=(this: this) => Bucket<K>
/// @type.symbol symbol=Bucket.K source="K: Equal<K>" type=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=Bucket.K

    key: K;
    /// @type.symbol symbol=Bucket.key source="key: K" type=K
    /// @resolution.name source=K target=Bucket.K

    constructor(key: K) {
    /// @type.symbol symbol=Bucket.constructor type=(K) => this
    /// @type.symbol symbol=Bucket.constructor.key source="key: K" type=K
    /// @resolution.name source=K target=Bucket.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Bucket type=Bucket<K>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.assignment source=this.key write="receiver=Bucket<K>, target=field(receiver=Bucket<K>, target=Bucket.key, type=K), type=K" type=K
        /// @resolution.name source=key target=Bucket.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Bucket.constructor.key

    }

    pair(): Bucket<K> {
    /// @type.symbol symbol=Bucket.pair type=(this: this) => Bucket<K>
    /// @resolution.name source=Bucket target=Bucket
    /// @resolution.name source=K target=Bucket.K

        return new Bucket<K>(this.key);
        /// @resolution.construct source="new Bucket<K>(this.key)" parameters=(K) arguments=(provided(this.key) as K) return=Bucket<K> kind=class target=Bucket constructor=Bucket.constructor instance=Bucket<K>
        /// @generic.instance source="new Bucket<K>(this.key)" id=Bucket<K>
        /// @resolution.name source=Bucket target=Bucket
        /// @resolution.name source=K target=Bucket.K
        /// @resolution.member source=this.key receiver=Bucket<K> type=K kind=field target_receiver=Bucket<K> key=key target=Bucket.key target_type=K
        /// @resolution.receiver source=this kind=this declaration=Bucket type=Bucket<K>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}

/// @generic.instance id=Bucket<K> template=Bucket arguments=(K)
"#, "");
}

#[test]
fn test_extension_where_clause_satisfies_call_bound() {
    let session = TestSession::single(
        r#"
interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }
}

extension<K> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Equal<in T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<in out K> {
    key: K;

    constructor(key: K): this {
        this.key = key;
    }
}

extension<K> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== checked ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(in T#1)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(in T#1)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(this: this, T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(this: this, T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @type.symbol symbol=probe.value source="value: T" type=T#2
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(in out K#1)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(K#1) => this
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(K#1) => this
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.assignment source=this.key write="receiver=Box<K#1>, target=field(receiver=Box<K#1>, target=Box.key, type=K#1), type=K#1" type=K#1
        /// @resolution.name source=key target=Box.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Box.constructor.key

    }
}

extension<K> of Box<K> where K: Equal<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=Equal<K#2>
/// @definition.method symbol=check slot=check type=(this: this) => boolean
/// @type.symbol symbol=K source=K type=K#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: this) => boolean

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instance source=probe(this.key) id=probe<K#2>
        /// @resolution.member source=this.key receiver=Box<K#2> type=K#2 kind=field target_receiver=Box<K#2> key=key target=Box.key target_type=K#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}

/// @generic.instance id=probe<K#2> template=probe arguments=(K#2)
"#, "");
}

#[test]
fn test_method_where_clause_satisfies_call_bound() {
    let session = TestSession::single(
        r#"
interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }

    check(): boolean where K: Equal<K> {
        return probe(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Equal<in T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<in out K> {
    key: K;

    constructor(key: K): this {
        this.key = key;
    }

    check(): boolean where K: Equal<K> {
        return probe<K>(this.key);
    }
}

=== checked ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(in T#1)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(in T#1)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(this: this, T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(this: this, T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @type.symbol symbol=probe.value source="value: T" type=T#2
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(in out K)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out K)
/// @definition.field symbol=Box.key source="key: K" key=key type=K
/// @definition.method symbol=Box.check slot=check type=(this: this) => boolean
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(K) => this
/// @type.symbol symbol=Box.K source=K type=K

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(K) => this
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.assignment source=this.key write="receiver=Box<K>, target=field(receiver=Box<K>, target=Box.key, type=K), type=K" type=K
        /// @resolution.name source=key target=Box.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Box.constructor.key

    }

    check(): boolean where K: Equal<K> {
    /// @type.symbol symbol=Box.check type=(this: this) => boolean
    /// @resolution.name source=K target=Box.K
    /// @resolution.name source=Equal target=Equal
    /// @resolution.name source=K target=Box.K

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K) arguments=(provided(this.key) as K) return=boolean kind=symbol target=probe instance=probe<K>
        /// @generic.instance source=probe(this.key) id=probe<K>
        /// @resolution.member source=this.key receiver=Box<K> type=K kind=field target_receiver=Box<K> key=key target=Box.key target_type=K
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}

/// @generic.instance id=probe<K> template=probe arguments=(K)
"#, "");
}

#[test]
fn test_extension_method_calls_module_function() {
    let session = TestSession::single(
        r#"
declare function probe<T>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }
}

extension<K> of Box<K> {
    check(): boolean {
        return probe(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function probe<T>(value: T): boolean;

class Box<in out K> {
    key: K;

    constructor(key: K): this {
        this.key = key;
    }
}

extension<K> of Box<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== checked ===
declare function probe<T>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T)
/// @type.symbol symbol=probe source="declare function probe<T>(value: T): boolean" type=<T>(T) => boolean
/// @type.symbol symbol=probe.T source=T type=T
/// @type.symbol symbol=probe.value source="value: T" type=T
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(in out K#1)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(K#1) => this
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(K#1) => this
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.assignment source=this.key write="receiver=Box<K#1>, target=field(receiver=Box<K#1>, target=Box.key, type=K#1), type=K#1" type=K#1
        /// @resolution.name source=key target=Box.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Box.constructor.key

    }
}

extension<K> of Box<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.method symbol=check slot=check type=(this: this) => boolean
/// @type.symbol symbol=K source=K type=K#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: this) => boolean

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instance source=probe(this.key) id=probe<K#2>
        /// @resolution.member source=this.key receiver=Box<K#2> type=K#2 kind=field target_receiver=Box<K#2> key=key target=Box.key target_type=K#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}

/// @generic.instance id=probe<K#2> template=probe arguments=(K#2)
"#, "");
}

#[test]
fn test_constrained_parameter_takes_where_clause_bounds() {
    let session = TestSession::single(
        r#"
interface Hash {
    hash(): float64;
}

interface Equal<T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }
}

extension<K: Hash> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe(this.key);
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Hash {
    hash(): float64;
}

interface Equal<in T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<in out K> {
    key: K;

    constructor(key: K): this {
        this.key = key;
    }
}

extension<K: Hash> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== checked ===
interface Hash {
/// @type.symbol symbol=Hash type=Hash
/// @definition.interface symbol=Hash
/// @definition.method symbol=Hash.hash source="hash(): float64" slot=hash type=(this: this) => float64

    hash(): float64;
    /// @type.symbol symbol=Hash.hash source="hash(): float64" type=(this: this) => float64

}

interface Equal<T> {
/// @generic.template symbol=Equal parameters=(in T#1)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(in T#1)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(this: this, T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(this: this, T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @type.symbol symbol=probe.value source="value: T" type=T#2
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(in out K#1)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(K#1) => this
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(K#1) => this
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.assignment source=this.key write="receiver=Box<K#1>, target=field(receiver=Box<K#1>, target=Box.key, type=K#1), type=K#1" type=K#1
        /// @resolution.name source=key target=Box.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Box.constructor.key

    }
}

extension<K: Hash> of Box<K> where K: Equal<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2: Hash)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=Equal<K#2>
/// @definition.method symbol=check slot=check type=(this: this) => boolean
/// @type.symbol symbol=K source="K: Hash" type=K#2
/// @resolution.name source=Hash target=Hash
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: this) => boolean

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instance source=probe(this.key) id=probe<K#2>
        /// @resolution.member source=this.key receiver=Box<K#2> type=K#2 kind=field target_receiver=Box<K#2> key=key target=Box.key target_type=K#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}

/// @generic.instance id=probe<K#2> template=probe arguments=(K#2)
"#, "");
}

#[test]
fn test_member_selects_through_where_bound() {
    let session = TestSession::single(
        r#"
interface Doubling {
    double(): int32;
}

function twice<T>(value: T): int32 where T: Doubling {
    return value.double();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Doubling {
    double(): int32;
}

function twice<T>(value: T): int32 where T: Doubling {
    return value.double();
}

=== checked ===
interface Doubling {
/// @type.symbol symbol=Doubling type=Doubling
/// @definition.interface symbol=Doubling
/// @definition.method symbol=Doubling.double source="double(): int32" slot=double type=(this: this) => int32

    double(): int32;
    /// @type.symbol symbol=Doubling.double source="double(): int32" type=(this: this) => int32

}

function twice<T>(value: T): int32 where T: Doubling {
/// @generic.template symbol=twice parameters=(T)
/// @type.symbol symbol=twice type=<T>(T) => int32
/// @type.symbol symbol=twice.T source=T type=T
/// @type.symbol symbol=twice.value source="value: T" type=T
/// @resolution.name source=T target=twice.T
/// @resolution.name source=T target=twice.T
/// @resolution.name source=Doubling target=Doubling

    return value.double();
    /// @resolution.name source=value target=twice.value
    /// @resolution.member source=value.double receiver=T type=(this: T) => int32 kind=symbol target_receiver=T target=Doubling.double
    /// @resolution.call source=value.double() parameters=() return=int32 kind=symbol target=Doubling.double receiver=T
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=twice.value

}
"#,
    );
}

#[test]
fn test_exported_static_member_infers_extension_parameters() {
    let session = TestSession::single(
        r#"
declare function todo(message: string): never;

newtype Inner<T> = intrinsic;

extension<T> of Inner<T> {
    static new(value: T): Inner<T> {
        todo("Inner.new")
    }
}

export struct Cell<T> {
    storage: Inner<T>;
}

export extension<T> of Cell<T> {
    static new(value: T): Cell<T> {
        Cell { storage: Inner.new(value) }
    }
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function todo(message: string): never;

newtype Inner<in out T> = intrinsic;

extension<T> of Inner<T> {
    static new(value: T): Inner<T> {
        todo("Inner.new")
    }
}

export struct Cell<in out T> {
    storage: Inner<T>;
}

export extension<T> of Cell<T> {
    static new(value: T): Cell<T> {
        Cell<T> { storage: Inner.new<T>(value) }
    }
}

=== checked ===
declare function todo(message: string): never;
/// @type.symbol symbol=todo source="declare function todo(message: string): never" type=(string) => never
/// @type.symbol symbol=todo.message source="message: string" type=string

newtype Inner<T> = intrinsic;
/// @generic.template symbol=Inner parameters=(in out T#1)
/// @type.symbol symbol=Inner source="newtype Inner<T> = intrinsic" type=Inner
/// @definition.newtype symbol=Inner source="newtype Inner<T> = intrinsic" template=(in out T#1) backing=intrinsic
/// @type.symbol symbol=Inner.T source=T type=T#1

extension<T> of Inner<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Inner<T#2>
/// @definition.method symbol=new#1 slot=new static=true type=(T#2) => Inner<T#2>
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Inner target=Inner
/// @resolution.name source=T target=T#1

    static new(value: T): Inner<T> {
    /// @type.symbol symbol=new#1 type=(T#2) => Inner<T#2>
    /// @type.symbol symbol=new.value#1 source="value: T" type=T#2
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=T#1

        todo("Inner.new")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Inner.new\")" parameters=(string) arguments=(provided("Inner.new") as string) return=never kind=symbol target=todo

    }
}

export struct Cell<T> {
/// @generic.template symbol=Cell parameters=(in out T#3)
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell template=(in out T#3)
/// @definition.field symbol=Cell.storage source="storage: Inner<T>" key=storage type=Inner<T#3>
/// @type.symbol symbol=Cell.T source=T type=T#3

    storage: Inner<T>;
    /// @type.symbol symbol=Cell.storage source="storage: Inner<T>" type=Inner<T#3>
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=Cell.T

}

export extension<T> of Cell<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @definition.extension symbol=<module>#3 form=exported target=Cell<T#4>
/// @definition.method symbol=new#2 slot=new static=true type=(T#4) => Cell<T#4>
/// @type.symbol symbol=T#2 source=T type=T#4
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=T target=T#2

    static new(value: T): Cell<T> {
    /// @type.symbol symbol=new#2 type=(T#4) => Cell<T#4>
    /// @type.symbol symbol=new.value#2 source="value: T" type=T#4
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=T target=T#2

        Cell { storage: Inner.new(value) }
        /// @resolution.name source=Cell target=Cell
        /// @resolution.name source=Inner target=Inner
        /// @resolution.member source=Inner.new receiver=Inner type=(T#2) => Inner<T#2> kind=symbol target_receiver=Inner target=new#1
        /// @resolution.call source=Inner.new(value) parameters=(T#4) arguments=(provided(value) as T#4) return=Inner<T#4> kind=symbol target=new#1 receiver=Inner instance=Inner<T#4>.<extension#1>.new#1
        /// @generic.instance source=Inner.new(value) id=Inner<T#4>.<extension#1>.new#1
        /// @resolution.name source=value target=new.value#2
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=new.value#2

    }
}

/// @generic.instance id=Cell<T#4> template=Cell arguments=(T#4)
/// @generic.instance id=Inner<T#2> template=Inner arguments=(T#2)
/// @generic.instance id=Inner<T#3> template=Inner arguments=(T#3)
/// @generic.instance id=Inner<T#4>.<extension#1>.new#1 template=new#1 arguments=(T#4)
"#);
}

#[test]
fn test_where_equality_rejects_one_way_assignability() {
    let session = TestSession::single(
        r#"
function requireEqual<T, U>(left: T, right: U): void where T == U {}

declare const wider: { x: int32; y: string };
declare const narrower: { x: int32 };

requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function requireEqual<T, U>(left: T, right: U): void where T == U {}

declare const wider: { x: int32; y: string };
declare const narrower: { x: int32 };

requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower);

=== checked ===
function requireEqual<T, U>(left: T, right: U): void where T == U {}
/// @generic.template symbol=requireEqual parameters=(T, U)
/// @type.symbol symbol=requireEqual source="function requireEqual<T, U>(left: T, right: U): void where T == U {}" type=<T, U>(T, U) => void
/// @type.symbol symbol=requireEqual.T source=T type=T
/// @type.symbol symbol=requireEqual.U source=U type=U
/// @type.symbol symbol=requireEqual.left source="left: T" type=T
/// @resolution.name source=T target=requireEqual.T
/// @type.symbol symbol=requireEqual.right source="right: U" type=U
/// @resolution.name source=U target=requireEqual.U
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=U target=requireEqual.U

declare const wider: { x: int32; y: string };
/// @type.symbol symbol=wider source=wider type={ x: int32; y: string }
/// @resolution.pattern source=wider kind=binding target=wider

declare const narrower: { x: int32 };
/// @type.symbol symbol=narrower source=narrower type={ x: int32 }
/// @resolution.pattern source=narrower kind=binding target=narrower

requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower)" parameters=({ x: int32; y: string }, { x: int32 }) arguments=(provided(wider) as { x: int32; y: string }, provided(narrower) as { x: int32 }) return=void kind=symbol target=requireEqual instance="requireEqual<{ x: int32; y: string }, { x: int32 }>"
/// @generic.instance source="requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower)" id="requireEqual<{ x: int32; y: string }, { x: int32 }>"
/// @resolution.name source=wider target=wider
/// @resolution.name source=narrower target=narrower

/// @generic.instance id="requireEqual<{ x: int32; y: string }, { x: int32 }>" template=requireEqual arguments=({ x: int32; y: string }, { x: int32 })
"#,
        r#"
/// @diagnostic.error id=equality-requirement-not-satisfied message="equality requirement '{ x: int32; y: string } == { x: int32 }' is not satisfied"
/// @diagnostic.label line=7 column=1 span="requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower)" line_source="requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower);"
"#,
    );
}

#[test]
fn test_where_equality_infers_common_type() {
    let session = TestSession::single(
        r#"
function requireEqual<T, U>(left: T, right: U): void where T == U {}

declare const wider: { x: int32; y: string };
declare const narrower: { x: int32 };

requireEqual(wider, narrower);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function requireEqual<T, U>(left: T, right: U): void where T == U {}

declare const wider: { x: int32; y: string };
declare const narrower: { x: int32 };

requireEqual<{ x: int32 }, { x: int32 }>(wider, narrower);

=== checked ===
function requireEqual<T, U>(left: T, right: U): void where T == U {}
/// @generic.template symbol=requireEqual parameters=(T, U)
/// @type.symbol symbol=requireEqual source="function requireEqual<T, U>(left: T, right: U): void where T == U {}" type=<T, U>(T, U) => void
/// @type.symbol symbol=requireEqual.T source=T type=T
/// @type.symbol symbol=requireEqual.U source=U type=U
/// @type.symbol symbol=requireEqual.left source="left: T" type=T
/// @resolution.name source=T target=requireEqual.T
/// @type.symbol symbol=requireEqual.right source="right: U" type=U
/// @resolution.name source=U target=requireEqual.U
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=U target=requireEqual.U

declare const wider: { x: int32; y: string };
/// @type.symbol symbol=wider source=wider type={ x: int32; y: string }
/// @resolution.pattern source=wider kind=binding target=wider

declare const narrower: { x: int32 };
/// @type.symbol symbol=narrower source=narrower type={ x: int32 }
/// @resolution.pattern source=narrower kind=binding target=narrower

requireEqual(wider, narrower);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(wider, narrower)" parameters=({ x: int32 }, { x: int32 }) arguments=(provided(wider) as { x: int32 }, provided(narrower) as { x: int32 }) return=void kind=symbol target=requireEqual instance="requireEqual<{ x: int32 }, { x: int32 }>"
/// @generic.instance source="requireEqual(wider, narrower)" id="requireEqual<{ x: int32 }, { x: int32 }>"
/// @resolution.name source=wider target=wider
/// @resolution.place source=wider placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=wider root=wider
/// @resolution.name source=narrower target=narrower
/// @resolution.place source=narrower placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=narrower root=narrower

/// @generic.instance id="requireEqual<{ x: int32 }, { x: int32 }>" template=requireEqual arguments=({ x: int32 }, { x: int32 })
"#,
    );
}

#[test]
fn test_static_member_selects_through_a_parameter_bound() {
    let session = TestSession::single(
        r#"
interface Makeable {
    static make(): this;
}

function build<T: Makeable>(): T {
    T.make()
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Makeable {
    static make(): this;
}

function build<T: Makeable>(): T {
    T.make()
}

=== checked ===
interface Makeable {
/// @type.symbol symbol=Makeable type=Makeable
/// @definition.interface symbol=Makeable
/// @definition.method symbol=Makeable.make source="static make(): this" slot=make static=true type=() => this

    static make(): this;
    /// @type.symbol symbol=Makeable.make source="static make(): this" type=() => this

}

function build<T: Makeable>(): T {
/// @generic.template symbol=build parameters=(T: Makeable)
/// @type.symbol symbol=build type=<T: Makeable>() => T
/// @type.symbol symbol=build.T source="T: Makeable" type=T
/// @resolution.name source=Makeable target=Makeable
/// @resolution.name source=T target=build.T

    T.make()
    /// @resolution.name source=T target=build.T
    /// @resolution.member source=T.make receiver=T type=() => T kind=symbol target_receiver=T target=Makeable.make
    /// @resolution.call source=T.make() parameters=() return=T kind=symbol target=Makeable.make receiver=T

}
"#);
}

#[test]
fn test_static_member_selects_through_a_union_parameter_bound() {
    let session = TestSession::single(
        r#"
interface Zero {
    static zero(): this;
}

interface Integer extends Zero {}
interface Float extends Zero {}
type Numeric = Integer | Float;

function zero<T: Numeric>(): T {
    T.zero()
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Zero {
    static zero(): this;
}

interface Integer extends Zero {}
interface Float extends Zero {}
type Numeric = Integer | Float;

function zero<T: Numeric>(): T {
    T.zero()
}

=== checked ===
interface Zero {
/// @type.symbol symbol=Zero type=Zero
/// @definition.interface symbol=Zero
/// @definition.method symbol=Zero.zero source="static zero(): this" slot=zero static=true type=() => this

    static zero(): this;
    /// @type.symbol symbol=Zero.zero source="static zero(): this" type=() => this

}

interface Integer extends Zero {}
/// @type.symbol symbol=Integer source="interface Integer extends Zero {}" type=Integer
/// @definition.interface symbol=Integer source="interface Integer extends Zero {}"
/// @definition.extends symbol=Integer source=Zero target=Zero
/// @resolution.name source=Zero target=Zero

interface Float extends Zero {}
/// @type.symbol symbol=Float source="interface Float extends Zero {}" type=Float
/// @definition.interface symbol=Float source="interface Float extends Zero {}"
/// @definition.extends symbol=Float source=Zero target=Zero
/// @resolution.name source=Zero target=Zero

type Numeric = Integer | Float;
/// @type.symbol symbol=Numeric source="type Numeric = Integer | Float" type=Integer | Float
/// @definition.type symbol=Numeric source="type Numeric = Integer | Float" value=Integer | Float
/// @resolution.name source=Integer target=Integer
/// @resolution.name source=Float target=Float

function zero<T: Numeric>(): T {
/// @generic.template symbol=zero parameters=(T: Numeric)
/// @type.symbol symbol=zero type=<T: Numeric>() => T
/// @type.symbol symbol=zero.T source="T: Numeric" type=T
/// @resolution.name source=Numeric target=Numeric
/// @resolution.name source=T target=zero.T

    T.zero()
    /// @resolution.name source=T target=zero.T
    /// @resolution.member source=T.zero type=() => T kind=union arms=[receiver=T, target=Zero.zero, type=() => T, receiver=T, target=Zero.zero, type=() => T]
    /// @resolution.call source=T.zero() return=T kind=union arms=[Zero.zero(parameters=(), arguments=(), return=T), Zero.zero(parameters=(), arguments=(), return=T)]

}
"#,
    );
}

#[test]
fn test_generic_float_arithmetic_accepts_scalar_literals() {
    let session = TestSession::single(
        r#"
import { Float } from "destack:math";

declare function log<T: Float>(value: T): T;
declare function sqrt<T: Float>(value: T): T;

function asinh<T: Float>(x: T): T {
    log(x + sqrt(x * x + 1))
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Float } from "destack:math";

declare function log<T: Float>(value: T): T;
declare function sqrt<T: Float>(value: T): T;

function asinh<T: Float>(x: T): T {
    log<T>(x + sqrt<T>(x * x + 1))
}

=== checked ===
import { Float } from "destack:math";

declare function log<T: Float>(value: T): T;
/// @generic.template symbol=log parameters=(T#1: math.float.Float)
/// @type.symbol symbol=log source="declare function log<T: Float>(value: T): T" type=<T#1: math.float.Float>(T#1) => T#1
/// @type.symbol symbol=log.T source="T: Float" type=T#1
/// @resolution.name source=Float target=math.float.Float
/// @type.symbol symbol=log.value source="value: T" type=T#1
/// @resolution.name source=T target=log.T
/// @resolution.name source=T target=log.T

declare function sqrt<T: Float>(value: T): T;
/// @generic.template symbol=sqrt parameters=(T#2: math.float.Float)
/// @type.symbol symbol=sqrt source="declare function sqrt<T: Float>(value: T): T" type=<T#2: math.float.Float>(T#2) => T#2
/// @type.symbol symbol=sqrt.T source="T: Float" type=T#2
/// @resolution.name source=Float target=math.float.Float
/// @type.symbol symbol=sqrt.value source="value: T" type=T#2
/// @resolution.name source=T target=sqrt.T
/// @resolution.name source=T target=sqrt.T

function asinh<T: Float>(x: T): T {
/// @generic.template symbol=asinh parameters=(T#3: math.float.Float)
/// @type.symbol symbol=asinh type=<T#3: math.float.Float>(T#3) => T#3
/// @type.symbol symbol=asinh.T source="T: Float" type=T#3
/// @resolution.name source=Float target=math.float.Float
/// @type.symbol symbol=asinh.x source="x: T" type=T#3
/// @resolution.name source=T target=asinh.T
/// @resolution.name source=T target=asinh.T

    log(x + sqrt(x * x + 1))
    /// @resolution.name source=log target=log
    /// @resolution.call source="log(x + sqrt(x * x + 1))" parameters=(T#3) arguments=(provided(x + sqrt(x * x + 1)) as T#3) return=T#3 kind=symbol target=log instance=log<T#3>
    /// @generic.instance source="log(x + sqrt(x * x + 1))" id=log<T#3>
    /// @resolution.name source=x target=asinh.x
    /// @resolution.operator source="x + sqrt(x * x + 1)" type=T#3 operator="+" kind=builtin operands=[x as T#3 families=(float), sqrt(x * x + 1) as T#3 families=(float)]
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=asinh.x
    /// @resolution.name source=sqrt target=sqrt
    /// @resolution.call source="sqrt(x * x + 1)" parameters=(T#3) arguments=(provided(x * x + 1) as T#3) return=T#3 kind=symbol target=sqrt instance=sqrt<T#3>
    /// @generic.instance source="sqrt(x * x + 1)" id=sqrt<T#3>
    /// @resolution.name source=x target=asinh.x
    /// @resolution.operator source="x * x + 1" type=T#3 operator="+" kind=builtin operands=[x * x as T#3 families=(float), 1 as T#3 families=(float)]
    /// @resolution.operator source="x * x" type=T#3 operator="*" kind=builtin operands=[x as T#3 families=(float), x as T#3 families=(float)]
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=asinh.x
    /// @resolution.name source=x target=asinh.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=asinh.x

}

/// @generic.instance id=log<T#3> template=log arguments=(T#3)
/// @generic.instance id=sqrt<T#3> template=sqrt arguments=(T#3)
"#,
    );
}

#[test]
fn test_static_member_infers_extension_parameters_at_calls() {
    let session = TestSession::single(
        r#"
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

extension<T> of Box<T> {
    static make(value: T): Box<T> {
        new Box<T>(value)
    }
}

extension<T> of Box<T> {
    static wrap(value: T): Box<T> {
        Box.make(value)
    }
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
class Box<in out T> {
    value: T;

    constructor(value: T): this {
        this.value = value;
    }
}

extension<T> of Box<T> {
    static make(value: T): Box<T> {
        new Box<T>(value)
    }
}

extension<T> of Box<T> {
    static wrap(value: T): Box<T> {
        Box.make<T>(value)
    }
}

=== checked ===
class Box<T> {
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(T#1) => this
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

    constructor(value: T) {
    /// @type.symbol symbol=Box.constructor type=(T#1) => this
    /// @type.symbol symbol=Box.constructor.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.assignment source=this.value write="receiver=Box<T#1>, target=field(receiver=Box<T#1>, target=Box.value, type=T#1), type=T#1" type=T#1
        /// @resolution.name source=value target=Box.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value

    }
}

extension<T> of Box<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.method symbol=make slot=make static=true type=(T#2) => Box<T#2>
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#1

    static make(value: T): Box<T> {
    /// @type.symbol symbol=make type=(T#2) => Box<T#2>
    /// @type.symbol symbol=make.value source="value: T" type=T#2
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=T#1

        new Box<T>(value)
        /// @resolution.construct source="new Box<T>(value)" parameters=(T#2) arguments=(provided(value) as T#2) return=Box<T#2> kind=class target=Box constructor=Box.constructor instance=Box<T#2>
        /// @generic.instance source="new Box<T>(value)" id=Box<T#2>
        /// @resolution.name source=Box target=Box
        /// @resolution.name source=T target=T#1
        /// @resolution.name source=value target=make.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=make.value

    }
}

extension<T> of Box<T> {
/// @generic.template symbol=<module>#3 parameters=(T#3)
/// @definition.extension symbol=<module>#3 form=local target=Box<T#3>
/// @definition.method symbol=wrap slot=wrap static=true type=(T#3) => Box<T#3>
/// @type.symbol symbol=T#2 source=T type=T#3
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T#2

    static wrap(value: T): Box<T> {
    /// @type.symbol symbol=wrap type=(T#3) => Box<T#3>
    /// @type.symbol symbol=wrap.value source="value: T" type=T#3
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=T#2

        Box.make(value)
        /// @resolution.name source=Box target=Box
        /// @resolution.member source=Box.make receiver=Box type=(T#2) => Box<T#2> kind=symbol target_receiver=Box target=make
        /// @resolution.call source=Box.make(value) parameters=(T#3) arguments=(provided(value) as T#3) return=Box<T#3> kind=symbol target=make receiver=Box instance=Box<T#3>.<extension#1>.make
        /// @generic.instance source=Box.make(value) id=Box<T#3>.<extension#1>.make
        /// @resolution.name source=value target=wrap.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=wrap.value

    }
}

/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @generic.instance id=Box<T#3> template=Box arguments=(T#3)
/// @generic.instance id=Box<T#3>.<extension#1>.make template=make arguments=(T#3)
"#);
}

#[test]
fn test_expected_field_type_drives_static_member_inference() {
    let session = TestSession::single(
        r#"
struct Inner<T> {
    value: T;
}

extension<T> of Inner<T> {
    static new(value: T): Inner<T> {
        Inner { value }
    }
}

struct Outer<T> {
    inner: Inner<T>;
}

extension<T> of Outer<T> {
    static new(value: T): Outer<T> {
        Outer { inner: Inner.new(value) }
    }
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Inner<out T> {
    value: T;
}

extension<T> of Inner<T> {
    static new(value: T): Inner<T> {
        Inner<T> { value }
    }
}

struct Outer<out T> {
    inner: Inner<T>;
}

extension<T> of Outer<T> {
    static new(value: T): Outer<T> {
        Outer<T> { inner: Inner.new<T>(value) }
    }
}

=== checked ===
struct Inner<T> {
/// @generic.template symbol=Inner parameters=(out T#1)
/// @type.symbol symbol=Inner type=Inner
/// @definition.struct symbol=Inner template=(out T#1)
/// @definition.field symbol=Inner.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Inner.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Inner.value source="value: T" type=T#1
    /// @resolution.name source=T target=Inner.T

}

extension<T> of Inner<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Inner<T#2>
/// @definition.method symbol=new#1 slot=new static=true type=(T#2) => Inner<T#2>
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Inner target=Inner
/// @resolution.name source=T target=T#1

    static new(value: T): Inner<T> {
    /// @type.symbol symbol=new#1 type=(T#2) => Inner<T#2>
    /// @type.symbol symbol=new.value#1 source="value: T" type=T#2
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=T#1

        Inner { value }
        /// @resolution.name source=Inner target=Inner
        /// @resolution.name source=value target=new.value#1
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=new.value#1

    }
}

struct Outer<T> {
/// @generic.template symbol=Outer parameters=(out T#3)
/// @type.symbol symbol=Outer type=Outer
/// @definition.struct symbol=Outer template=(out T#3)
/// @definition.field symbol=Outer.inner source="inner: Inner<T>" key=inner type=Inner<T#3>
/// @type.symbol symbol=Outer.T source=T type=T#3

    inner: Inner<T>;
    /// @type.symbol symbol=Outer.inner source="inner: Inner<T>" type=Inner<T#3>
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=Outer.T

}

extension<T> of Outer<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @definition.extension symbol=<module>#3 form=local target=Outer<T#4>
/// @definition.method symbol=new#2 slot=new static=true type=(T#4) => Outer<T#4>
/// @type.symbol symbol=T#2 source=T type=T#4
/// @resolution.name source=Outer target=Outer
/// @resolution.name source=T target=T#2

    static new(value: T): Outer<T> {
    /// @type.symbol symbol=new#2 type=(T#4) => Outer<T#4>
    /// @type.symbol symbol=new.value#2 source="value: T" type=T#4
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Outer target=Outer
    /// @resolution.name source=T target=T#2

        Outer { inner: Inner.new(value) }
        /// @resolution.name source=Outer target=Outer
        /// @resolution.name source=Inner target=Inner
        /// @resolution.member source=Inner.new receiver=Inner type=(T#2) => Inner<T#2> kind=symbol target_receiver=Inner target=new#1
        /// @resolution.call source=Inner.new(value) parameters=(T#4) arguments=(provided(value) as T#4) return=Inner<T#4> kind=symbol target=new#1 receiver=Inner instance=Inner<T#4>.<extension#1>.new#1
        /// @generic.instance source=Inner.new(value) id=Inner<T#4>.<extension#1>.new#1
        /// @resolution.name source=value target=new.value#2
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=new.value#2

    }
}

/// @generic.instance id=Inner<T#2> template=Inner arguments=(T#2)
/// @generic.instance id=Inner<T#3> template=Inner arguments=(T#3)
/// @generic.instance id=Inner<T#4>.<extension#1>.new#1 template=new#1 arguments=(T#4)
/// @generic.instance id=Outer<T#4> template=Outer arguments=(T#4)
"#);
}

#[test]
fn test_call_result_assigns_into_a_union_result() {
    let session = TestSession::single(
        r#"
function pair<T>(a: T): (T, boolean) {
    (a, true)
}

function check<T>(a: T): T | undefined {
    let (result, overflow) = pair(a);

    if (overflow) {
        return undefined;
    }

    result
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
function pair<T>(a: T): (T, boolean) {
    (a, true)
}

function check<T>(a: T): T | undefined {
    let (result, overflow) = pair<T>(a);

    if (overflow) {
        return undefined as T | undefined;
    }

    result as T | undefined
}

=== checked ===
function pair<T>(a: T): (T, boolean) {
/// @generic.template symbol=pair parameters=(T#1)
/// @type.symbol symbol=pair type=<T#1>(T#1) => (T#1, boolean)
/// @type.symbol symbol=pair.T source=T type=T#1
/// @type.symbol symbol=pair.a source="a: T" type=T#1
/// @resolution.name source=T target=pair.T
/// @resolution.name source=T target=pair.T

    (a, true)
    /// @resolution.name source=a target=pair.a
    /// @resolution.place source=a placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=a root=pair.a

}

function check<T>(a: T): T | undefined {
/// @generic.template symbol=check parameters=(T#2)
/// @type.symbol symbol=check type=<T#2>(T#2) => T#2 | undefined
/// @type.symbol symbol=check.T source=T type=T#2
/// @type.symbol symbol=check.a source="a: T" type=T#2
/// @resolution.name source=T target=check.T
/// @resolution.name source=T target=check.T

    let (result, overflow) = pair(a);
    /// @resolution.pattern source=(result, overflow) kind=tuple fields=(check.result, check.overflow)
    /// @type.symbol symbol=check.result source=result type=T#2
    /// @resolution.pattern source=result kind=binding target=check.result
    /// @type.symbol symbol=check.overflow source=overflow type=boolean
    /// @resolution.pattern source=overflow kind=binding target=check.overflow
    /// @resolution.name source=pair target=pair
    /// @resolution.call source=pair(a) parameters=(T#2) arguments=(provided(a) as T#2) return=(T#2, boolean) kind=symbol target=pair instance=pair<T#2>
    /// @generic.instance source=pair(a) id=pair<T#2>
    /// @resolution.name source=a target=check.a
    /// @resolution.place source=a placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=a root=check.a

    if (overflow) {
    /// @resolution.name source=overflow target=check.overflow
    /// @resolution.place source=overflow placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=overflow root=check.overflow

        return undefined;
    }

    result
    /// @resolution.name source=result target=check.result
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=check.result

}

/// @generic.instance id=pair<T#2> template=pair arguments=(T#2)
"#);
}
#[test]
fn test_static_bound_member_infers_method_type_argument() {
    let session = TestSession::single(
        r#"
interface Iterator<out T, out R = void> {}

interface FromIterator<T> {
    static fromIterator<R>(values: Iterator<T, R>): this;
}

extension<T, R, I: Iterator<T, R>> of I {
    collect<C>(): C where C: FromIterator<T> {
        C.fromIterator(this)
    }
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Iterator<out T, out R = void> {}

interface FromIterator<in out T> {
    static fromIterator<R>(values: Dynamic<Iterator<T, R>>): this;
}

extension<T, R, I: Iterator<T, R>> of I {
    collect<C>(): C where C: FromIterator<T> {
        C.fromIterator<T, R>(this as Dynamic<Iterator<T, R>>)
    }
}

=== checked ===
interface Iterator<out T, out R = void> {}
/// @generic.template symbol=Iterator parameters=(out T#1, out R#1 = void)
/// @type.symbol symbol=Iterator source="interface Iterator<out T, out R = void> {}" type=Iterator
/// @definition.interface symbol=Iterator source="interface Iterator<out T, out R = void> {}" template=(out T#1, out R#1 = void)
/// @definition.where symbol=Iterator source="interface Iterator<out T, out R = void> {}" relation=satisfies left=this right=Iterator<T#1, R#1>
/// @type.symbol symbol=Iterator.T source="out T" type=T#1
/// @type.symbol symbol=Iterator.R source="out R = void" type=R#1

interface FromIterator<T> {
/// @generic.template symbol=FromIterator parameters=(in out T#2)
/// @type.symbol symbol=FromIterator type=FromIterator
/// @definition.interface symbol=FromIterator template=(in out T#2)
/// @definition.where symbol=FromIterator relation=satisfies left=this right=FromIterator<T#2>
/// @definition.method symbol=FromIterator.fromIterator source="static fromIterator<R>(values: Iterator<T, R>): this" slot=fromIterator static=true type=<R#2>(Dynamic<Iterator<T#2, R#2>>) => this
/// @type.symbol symbol=FromIterator.T source=T type=T#2

    static fromIterator<R>(values: Iterator<T, R>): this;
    /// @generic.template symbol=FromIterator.fromIterator parent=template#1 parameters=(R#2)
    /// @type.symbol symbol=FromIterator.fromIterator source="static fromIterator<R>(values: Iterator<T, R>): this" type=<R#2>(Dynamic<Iterator<T#2, R#2>>) => this
    /// @type.symbol symbol=FromIterator.fromIterator.R source=R type=R#2
    /// @type.symbol symbol=FromIterator.fromIterator.values source="values: Iterator<T, R>" type=Dynamic<Iterator<T#2, R#2>>
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=FromIterator.T
    /// @resolution.name source=R target=FromIterator.fromIterator.R

}

extension<T, R, I: Iterator<T, R>> of I {
/// @generic.template symbol=<module>#2 parameters=(T#3, R#3, I: Iterator<T#3, R#3>)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.method symbol=collect slot=collect type=<C>(this: this) => C
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=R source=R type=R#3
/// @type.symbol symbol=I source="I: Iterator<T, R>" type=I
/// @resolution.name source=Iterator target=Iterator
/// @resolution.name source=T target=T
/// @resolution.name source=R target=R
/// @resolution.name source=I target=I

    collect<C>(): C where C: FromIterator<T> {
    /// @generic.template symbol=collect parent=template#2 parameters=(C)
    /// @type.symbol symbol=collect type=<C>(this: this) => C
    /// @type.symbol symbol=collect.C source=C type=C
    /// @resolution.name source=C target=collect.C
    /// @resolution.name source=C target=collect.C
    /// @resolution.name source=FromIterator target=FromIterator
    /// @resolution.name source=T target=T

        C.fromIterator(this)
        /// @resolution.name source=C target=collect.C
        /// @resolution.member source=C.fromIterator receiver=C type=<R#2>(Dynamic<Iterator<T#3, R#2>>) => C kind=symbol target_receiver=C target=FromIterator.fromIterator
        /// @resolution.call source=C.fromIterator(this) parameters=(Dynamic<Iterator<T#3, R#3>>) arguments=(provided(this) as Dynamic<Iterator<T#3, R#3>>) return=C kind=symbol target=FromIterator.fromIterator receiver=C instance=FromIterator.fromIterator<R#3>
        /// @generic.instance source=C.fromIterator(this) id=FromIterator.fromIterator<R#3>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

    }
}

/// @generic.instance id="Iterator<T#2, R#2>" template=Iterator arguments=(T#2, R#2)
/// @generic.instance id=FromIterator.fromIterator<R#3> template=FromIterator.fromIterator arguments=(T#3, R#3)
"#);
}
