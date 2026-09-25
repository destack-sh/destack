use crate::tests::{DirRows, TestSession};

/// A const type parameter keeps the precision of a scalar literal argument.
#[test]
fn test_const_type_parameter_preserves_scalar_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<const T>(value: T): T;

const value = id("ready");
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<const T>(value: T): T;

const value: "ready" = id<"ready">("ready");

=== dir ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id("ready");
/// @type.symbol symbol=value source=value type="ready"
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=id target=id
/// @resolution.call source="id(\"ready\")" parameters=("ready") arguments=(provided("ready") as "ready") return="ready" kind=symbol target=id instance="id<\"ready\">"
/// @generic.instantiation id="id<\"ready\">" template=id arguments=("ready")
/// @generic.instance id="id<\"ready\">" template=id arguments=("ready")
"#,
    );
}

/// A const type parameter keeps the precision of an array literal argument.
#[test]
fn test_const_type_parameter_preserves_array_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<const T>(value: T): T;

const values = id([1, 2]);
const first = values[0];
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<const T>(value: T): T;

const values: readonly (1 | 2)[] = id<readonly (1 | 2)[]>([1, 2]);
const first: 1 | 2 = values[0];

=== dir ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const values = id([1, 2]);
/// @type.symbol symbol=values source=values type=readonly 1 | 2[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<1 | 2>" template=Array arguments=(1 | 2)
/// @generic.instance id="sliceAssumeInit<MaybeUninit<1 | 2>>" template=sliceAssumeInit arguments=(MaybeUninit<1 | 2>)
/// @generic.instance id="sliceUninit<MaybeUninit<1 | 2>>" template=sliceUninit arguments=(MaybeUninit<1 | 2>)
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(readonly 1 | 2[]) arguments=(provided([1, 2]) as readonly 1 | 2[]) return=readonly 1 | 2[] kind=symbol target=id instance="id<readonly 1 | 2[]>"
/// @generic.instantiation id="id<readonly 1 | 2[]>" template=id arguments=(readonly 1 | 2[])
/// @generic.instance id="id<readonly 1 | 2[]>" template=id arguments=(readonly 1 | 2[])
/// @resolution.call source=[1, 2] parameters=(^Slice<1 | 2>) arguments=(rest(provided(1) as 1 | 2, provided(2) as 1 | 2) as 1 | 2) return=1 | 2[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<1 | 2>"
/// @generic.instantiation id="arrayFromOwnedSlice<1 | 2>" template=arrayFromOwnedSlice arguments=(1 | 2)
/// @generic.instance id="arrayFromOwnedSlice<1 | 2>" template=arrayFromOwnedSlice arguments=(1 | 2)

const first = values[0];
/// @type.symbol symbol=first source=first type=1 | 2
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type=1 | 2 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=1 | 2, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<1 | 2, \"managed\" & \"local\">" template=index#2 arguments=(1 | 2, "managed" & "local")
/// @generic.instance id="index#2<1 | 2, \"bound0\" & \"local\">" template=index#2 arguments=(1 | 2, "bound0" & "local")
"#,
    );
}

/// A plain type parameter widens an array literal argument.
#[test]
fn test_plain_type_parameter_widens_array_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;

const values = id([1, 2]);
const first = values[0];
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const values: int64[] = id<int64[]>([1, 2]);
const first: int64 = values[0];

=== dir ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const values = id([1, 2]);
/// @type.symbol symbol=values source=values type=int64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @resolution.name source=id target=id
/// @resolution.call source="id([1, 2])" parameters=(int64[]) arguments=(provided([1, 2]) as int64[]) return=int64[] kind=symbol target=id instance=id<int64[]>
/// @generic.instantiation id=id<int64[]> template=id arguments=(int64[])
/// @generic.instance id=id<int64[]> template=id arguments=(int64[])
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)

const first = values[0];
/// @type.symbol symbol=first source=first type=int64
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type=int64 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int64, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<int64, \"managed\" & \"local\">" template=index#2 arguments=(int64, "managed" & "local")
/// @generic.instance id="index#2<int64, \"bound0\" & \"local\">" template=index#2 arguments=(int64, "bound0" & "local")
"#,
    );
}

/// A mutable array argument keeps its element type exact at a widening parameter.
#[test]
fn test_mutable_array_alias_does_not_widen_element_type() {
    let session = TestSession::single(
        r#"
declare function take(values: float64[]): void;
declare const values: (1 | 2)[];

take(values);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(values: float64[]): void;
declare const values: (1 | 2)[];

take(values);

=== dir ===
declare function take(values: float64[]): void;
/// @type.symbol symbol=take source="declare function take(values: float64[]): void" type=(float64[]) => void

declare const values: (1 | 2)[];
/// @type.symbol symbol=values source=values type=1 | 2[]
/// @resolution.pattern source=values kind=binding target=values

take(values);
/// @resolution.name source=take target=take
/// @resolution.call source=take(values) parameters=(float64[]) arguments=(provided(values) as float64[]) return=void kind=symbol target=take
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '1 | 2[]' is not assignable to parameter of type 'float64[]'"
/// @diagnostic.label line=5 column=6 span="values" line_source="take(values);"
/// @diagnostic.related line=5 column=1 span="take(values)" line_source="take(values);" message="in this call"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Array': expected 'float64', found '1 | 2'"
"#,
    );
}

/// An array literal argument materializes as a slice parameter.
#[test]
fn test_array_literal_materializes_as_slice_parameter() {
    let session = TestSession::single(
        r#"
declare function take(values: Slice<float64>): void;

take([1, 2]);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(values: Slice<float64>): void;

take([1, 2]);

=== dir ===
declare function take(values: Slice<float64>): void;
/// @type.symbol symbol=take source="declare function take(values: Slice<float64>): void" type=(Slice<float64>) => void
/// @resolution.name source=Slice target=Slice

take([1, 2]);
/// @resolution.name source=take target=take
/// @resolution.call source="take([1, 2])" parameters=(Slice<float64>) arguments=(provided([1, 2]) as Slice<float64>) return=void kind=symbol target=take
/// @resolution.call source=[1, 2] parameters=(^Slice<float64>) arguments=(rest(provided(1) as float64, provided(2) as float64) as float64) return=float64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<float64>
/// @generic.instantiation id=arrayFromOwnedSlice<float64> template=arrayFromOwnedSlice arguments=(float64)
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=arrayFromOwnedSlice<float64> template=arrayFromOwnedSlice arguments=(float64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<float64>> template=sliceAssumeInit arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceUninit<MaybeUninit<float64>> template=sliceUninit arguments=(MaybeUninit<float64>)
"#,
    );
}

/// An array literal argument materializes as a fixed array parameter.
#[test]
fn test_array_literal_materializes_as_fixed_array_parameter() {
    let session = TestSession::single(
        r#"
declare function take(values: [float64; 2]): void;

take([1, 2]);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(values: [float64; 2]): void;

take([1, 2]);

=== dir ===
declare function take(values: [float64; 2]): void;
/// @type.symbol symbol=take source="declare function take(values: [float64; 2]): void" type=(FixedArray<float64, 2>) => void

take([1, 2]);
/// @resolution.name source=take target=take
/// @resolution.call source="take([1, 2])" parameters=(FixedArray<float64, 2>) arguments=(provided([1, 2]) as FixedArray<float64, 2>) return=void kind=symbol target=take
"#,
    );
}

/// An array literal of the wrong length reports a diagnostic at a fixed array parameter.
#[test]
fn test_array_literal_rejects_mismatched_fixed_array_parameter_length() {
    let session = TestSession::single(
        r#"
declare function take(values: [float64; 2]): void;

take([1, 2, 3]);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(values: [float64; 2]): void;

take([1, 2, 3]);

=== dir ===
declare function take(values: [float64; 2]): void;
/// @type.symbol symbol=take source="declare function take(values: [float64; 2]): void" type=(FixedArray<float64, 2>) => void

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

/// A plain type parameter widens a tuple literal argument.
#[test]
fn test_plain_type_parameter_widens_tuple_literal_precision() {
    let session = TestSession::single(
        r#"
declare function id<T>(value: T): T;

const value = id((1, "x"));
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const value: (int64, string) = id<(int64, string)>((1, "x"));

=== dir ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id((1, "x"));
/// @type.symbol symbol=value source=value type=(int64, string)
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=id target=id
/// @resolution.call source="id((1, \"x\"))" parameters=((int64, string)) arguments=(provided((1, "x")) as (int64, string)) return=(int64, string) kind=symbol target=id instance="id<(int64, string)>"
/// @generic.instantiation id="id<(int64, string)>" template=id arguments=((int64, string))
/// @generic.instance id="id<(int64, string)>" template=id arguments=((int64, string))
"#,
    );
}

/// A const type parameter keeps the precision of a nested object literal argument.
#[test]
fn test_const_type_parameter_preserves_nested_object_literal_precision() {
    let session = TestSession::single(
        r#"
declare function collect<const T>(values: T[]): T[];

const values = collect([{ kind: "ready" }]);
const kind = values[0].kind;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function collect<const T>(values: T[]): T[];

const values: { readonly kind: "ready" }[] = collect<{ readonly kind: "ready" }>([
    { kind: "ready" },
]);
const kind: "ready" = values[0].kind;

=== dir ===
declare function collect<const T>(values: T[]): T[];
/// @generic.template symbol=collect parameters=(const T)
/// @type.symbol symbol=collect source="declare function collect<const T>(values: T[]): T[]" type=<const T>(T[]) => T[]
/// @generic.instance id=Array<T> template=Array arguments=(T)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<T>> template=sliceAssumeInit arguments=(MaybeUninit<T>)
/// @generic.instance id=sliceUninit<MaybeUninit<T>> template=sliceUninit arguments=(MaybeUninit<T>)
/// @type.symbol symbol=collect.T source="const T" type=T
/// @resolution.name source=T target=collect.T
/// @resolution.name source=T target=collect.T

const values = collect([{ kind: "ready" }]);
/// @type.symbol symbol=values source=values type={ readonly kind: "ready" }[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<{ readonly kind: \"ready\" }>" template=Array arguments=({ readonly kind: "ready" })
/// @generic.instance id="sliceAssumeInit<MaybeUninit<{ readonly kind: \"ready\" }>>" template=sliceAssumeInit arguments=(MaybeUninit<{ readonly kind: "ready" }>)
/// @generic.instance id="sliceUninit<MaybeUninit<{ readonly kind: \"ready\" }>>" template=sliceUninit arguments=(MaybeUninit<{ readonly kind: "ready" }>)
/// @resolution.name source=collect target=collect
/// @resolution.call source="collect([{ kind: \"ready\" }])" parameters=({ readonly kind: "ready" }[]) arguments=(provided([{ kind: "ready" }]) as { readonly kind: "ready" }[]) return={ readonly kind: "ready" }[] kind=symbol target=collect instance="collect<{ readonly kind: \"ready\" }>"
/// @generic.instantiation id="collect<{ readonly kind: \"ready\" }>" template=collect arguments=({ readonly kind: "ready" })
/// @generic.instance id="collect<{ readonly kind: \"ready\" }>" template=collect arguments=({ readonly kind: "ready" })
/// @resolution.call source=[{ kind: "ready" }] parameters=(^Slice<{ readonly kind: "ready" }>) arguments=(rest(provided({ kind: "ready" }) as { readonly kind: "ready" }) as { readonly kind: "ready" }) return={ readonly kind: "ready" }[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<{ readonly kind: \"ready\" }>"
/// @generic.instantiation id="arrayFromOwnedSlice<{ readonly kind: \"ready\" }>" template=arrayFromOwnedSlice arguments=({ readonly kind: "ready" })
/// @generic.instance id="arrayFromOwnedSlice<{ readonly kind: \"ready\" }>" template=arrayFromOwnedSlice arguments=({ readonly kind: "ready" })

const kind = values[0].kind;
/// @type.symbol symbol=kind source=kind type="ready"
/// @resolution.pattern source=kind kind=binding target=kind
/// @resolution.name source=values target=values
/// @resolution.member source=values[0].kind receiver={ readonly kind: "ready" } type="ready" kind=field target_receiver={ readonly kind: "ready" } key=kind target_type="ready"
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type={ readonly kind: "ready" } kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return={ readonly kind: \"ready\" }, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<{ readonly kind: \"ready\" }, \"managed\" & \"local\">" template=index#2 arguments=({ readonly kind: "ready" }, "managed" & "local")
/// @generic.instance id="index#2<{ readonly kind: \"ready\" }, \"bound0\" & \"local\">" template=index#2 arguments=({ readonly kind: "ready" }, "bound0" & "local")
"#,
    );
}

/// A const type parameter keeps literal precision through a union parameter.
#[test]
fn test_const_type_parameter_preserves_literal_precision_through_union() {
    let session = TestSession::single(
        r#"
declare function maybe<const T>(value: T | undefined): T | undefined;

const value = maybe({ kind: "ready" });
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
declare function maybe<const T>(value: T | undefined): T | undefined;

const value: { readonly kind: "ready" } | undefined = maybe<{ readonly kind: "ready" }>({
    kind: "ready",
} as { readonly kind: "ready" } | undefined);

=== dir ===
declare function maybe<const T>(value: T | undefined): T | undefined;
/// @generic.template symbol=maybe parameters=(const T)
/// @type.symbol symbol=maybe source="declare function maybe<const T>(value: T | undefined): T | undefined" type=<const T>(T | undefined) => T | undefined
/// @type.symbol symbol=maybe.T source="const T" type=T
/// @resolution.name source=T target=maybe.T
/// @resolution.name source=T target=maybe.T

const value = maybe({ kind: "ready" });
/// @type.symbol symbol=value source=value type={ readonly kind: "ready" } | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=maybe target=maybe
/// @resolution.call source="maybe({ kind: \"ready\" })" parameters=({ readonly kind: "ready" } | undefined) arguments=(provided({ kind: "ready" }) as { readonly kind: "ready" } | undefined) return={ readonly kind: "ready" } | undefined kind=symbol target=maybe instance="maybe<{ readonly kind: \"ready\" }>"
/// @generic.instantiation id="maybe<{ readonly kind: \"ready\" }>" template=maybe arguments=({ readonly kind: "ready" })
/// @generic.instance id="maybe<{ readonly kind: \"ready\" }>" template=maybe arguments=({ readonly kind: "ready" })
"#);
}

/// A const type parameter keeps the precision of an object literal argument.
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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
declare function id<const T>(value: T): T;
/// @generic.template symbol=id parameters=(const T)
/// @type.symbol symbol=id source="declare function id<const T>(value: T): T" type=<const T>(T) => T
/// @type.symbol symbol=id.T source="const T" type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value source=value type={ readonly kind: "ready"; readonly level: 1 }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=({ readonly kind: "ready"; readonly level: 1 }) arguments=(provided({ kind: "ready", level: 1 }) as { readonly kind: "ready"; readonly level: 1 }) return={ readonly kind: "ready"; readonly level: 1 } kind=symbol target=id instance="id<{ readonly kind: \"ready\"; readonly level: 1 }>"
/// @generic.instantiation id="id<{ readonly kind: \"ready\"; readonly level: 1 }>" template=id arguments=({ readonly kind: "ready"; readonly level: 1 })
/// @generic.instance id="id<{ readonly kind: \"ready\"; readonly level: 1 }>" template=id arguments=({ readonly kind: "ready"; readonly level: 1 })

const kind = value.kind;
/// @type.symbol symbol=kind source=kind type="ready"
/// @resolution.pattern source=kind kind=binding target=kind
/// @resolution.name source=value target=value
/// @resolution.member source=value.kind receiver={ readonly kind: "ready"; readonly level: 1 } type="ready" kind=field target_receiver={ readonly kind: "ready"; readonly level: 1 } key=kind target_type="ready"
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.access source=value.kind root=value keys=[kind]

const level = value.level;
/// @type.symbol symbol=level source=level type=1
/// @resolution.pattern source=level kind=binding target=level
/// @resolution.name source=value target=value
/// @resolution.member source=value.level receiver={ readonly kind: "ready"; readonly level: 1 } type=1 kind=field target_receiver={ readonly kind: "ready"; readonly level: 1 } key=level target_type=1
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.access source=value.level root=value keys=[level]
"#,
    );
}

/// A plain type parameter widens an object literal argument.
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function id<T>(value: T): T;

const value: { kind: string; level: int64 } = id<{ kind: string; level: int64 }>({
    kind: "ready",
    level: 1,
});
const kind: string = value.kind;
const level: int64 = value.level;

=== dir ===
declare function id<T>(value: T): T;
/// @generic.template symbol=id parameters=(T)
/// @type.symbol symbol=id source="declare function id<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=id.T source=T type=T
/// @resolution.name source=T target=id.T
/// @resolution.name source=T target=id.T

const value = id({ kind: "ready", level: 1 });
/// @type.symbol symbol=value source=value type={ kind: string; level: int64 }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=id target=id
/// @resolution.call source="id({ kind: \"ready\", level: 1 })" parameters=({ kind: string; level: int64 }) arguments=(provided({ kind: "ready", level: 1 }) as { kind: string; level: int64 }) return={ kind: string; level: int64 } kind=symbol target=id instance="id<{ kind: string; level: int64 }>"
/// @generic.instantiation id="id<{ kind: string; level: int64 }>" template=id arguments=({ kind: string; level: int64 })
/// @generic.instance id="id<{ kind: string; level: int64 }>" template=id arguments=({ kind: string; level: int64 })

const kind = value.kind;
/// @type.symbol symbol=kind source=kind type=string
/// @resolution.pattern source=kind kind=binding target=kind
/// @resolution.name source=value target=value
/// @resolution.member source=value.kind receiver={ kind: string; level: int64 } type=string kind=field target_receiver={ kind: string; level: int64 } key=kind target_type=string
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.access source=value.kind root=value keys=[kind]

const level = value.level;
/// @type.symbol symbol=level source=level type=int64
/// @resolution.pattern source=level kind=binding target=level
/// @resolution.name source=value target=value
/// @resolution.member source=value.level receiver={ kind: string; level: int64 } type=int64 kind=field target_receiver={ kind: string; level: int64 } key=level target_type=int64
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.access source=value.level root=value keys=[level]
"#,
    );
}

/// A member lookup on a self-recursive bound reports the missing member.
#[test]
fn test_recursive_constraint_member_lookup_reports_missing_member() {
    let session = TestSession::single(
        r#"
function read<T: T | { name: string }>(value: T): string {
    return value.name;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function read<T: T | { name: string }>(value: T): string {
    return value.name;
}

=== dir ===
function read<T: T | { name: string }>(value: T): string {
/// @generic.template symbol=read parameters=(T: T | { name: string })
/// @type.symbol symbol=read type=<T: T | { name: string }>(T) => string
/// @type.symbol symbol=read.T source="T: T | { name: string }" type=T
/// @resolution.name source=T target=read.T
/// @type.symbol symbol=read.name source="name: string" type=string
/// @type.symbol symbol=read.value source="value: T" type=T
/// @resolution.name source=T target=read.T

    return value.name;
    /// @resolution.name source=value target=read.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=read.value
    /// @resolution.rejected source=value.name

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'name' does not exist on type 'T'"
/// @diagnostic.label line=3 column=18 span="name" line_source="return value.name;"
"#,
    );
}

/// A type parameter satisfies the bound its own declaration states.
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Equal<in T> {
    equals(other: T): boolean;
}

class Bucket<in out K: Equal<K>> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }

    pair(): Bucket<K> {
        return new Bucket<K>(this.key);
    }
}

=== dir ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(in T, this: Equal<T>)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(in T, this: Equal<T>)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(T) => boolean
/// @type.symbol symbol=Equal.T source=T type=T

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(T) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T
    /// @resolution.name source=T target=Equal.T

}

class Bucket<K: Equal<K>> {
/// @generic.template symbol=Bucket parameters=(in out K: Equal<K>)
/// @type.symbol symbol=Bucket type=typeof Bucket
/// @definition.class symbol=Bucket template=(in out K: Equal<K>)
/// @definition.field symbol=Bucket.key source="key: K" key=key type=K
/// @definition.method symbol=Bucket.constructor slot=constructor role=constructor type=(this: &'managed Bucket<K>, K) => Bucket<K>
/// @definition.method symbol=Bucket.pair slot=pair type=(this: Bucket<K>) => Bucket<K>
/// @type.symbol symbol=Bucket.K source="K: Equal<K>" type=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=Bucket.K

    key: K;
    /// @type.symbol symbol=Bucket.key source="key: K" type=K
    /// @resolution.name source=K target=Bucket.K

    constructor(key: K) {
    /// @type.symbol symbol=Bucket.constructor type=(this: &'managed Bucket<K>, K) => Bucket<K>
    /// @type.symbol symbol=Bucket.constructor.this type=&'managed Bucket<K>
    /// @type.symbol symbol=Bucket.constructor.key source="key: K" type=K
    /// @resolution.name source=K target=Bucket.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Bucket type=&'managed Bucket<K>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]
        /// @resolution.assignment source=this.key write="receiver=&'managed Bucket<K>, target=field(receiver=&'managed Bucket<K>, target=Bucket.key, type=K), type=K" type=K
        /// @resolution.name source=key target=Bucket.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Bucket.constructor.key

    }

    pair(): Bucket<K> {
    /// @type.symbol symbol=Bucket.pair type=(this: Bucket<K>) => Bucket<K>
    /// @type.symbol symbol=Bucket.pair.this type=Bucket<K>
    /// @resolution.name source=Bucket target=Bucket
    /// @resolution.name source=K target=Bucket.K

        return new Bucket<K>(this.key);
        /// @resolution.construct source="new Bucket<K>(this.key)" parameters=(K) arguments=(provided(this.key) as K) return=Bucket<K> kind=class target=Bucket constructor=Bucket.constructor instance=Bucket<K>
        /// @generic.instantiation id=Bucket.constructor<K> template=Bucket.constructor arguments=(K) owner=Bucket.pair
        /// @generic.instantiation id=Bucket<K> template=Bucket arguments=(K) owner=Bucket.pair
        /// @resolution.name source=Bucket target=Bucket
        /// @resolution.name source=K target=Bucket.K
        /// @resolution.member source=this.key receiver=Bucket<K> type=K kind=field target_receiver=Bucket<K> key=key target=Bucket.key target_type=K
        /// @resolution.receiver source=this kind=this declaration=Bucket type=Bucket<K>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}
"#, r#"
"#);
}

/// An extension where clause satisfies the bound a call inside it requires.
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Equal<in T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<in out K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }
}

extension<K> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== dir ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(in T#1, this: Equal<T#1>)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(in T#1, this: Equal<T#1>)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(in out K#1)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(this: &'managed Box<K#1>, K#1) => Box<K#1>
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(this: &'managed Box<K#1>, K#1) => Box<K#1>
    /// @type.symbol symbol=Box.constructor.this type=&'managed Box<K#1>
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box<K#1>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]
        /// @resolution.assignment source=this.key write="receiver=&'managed Box<K#1>, target=field(receiver=&'managed Box<K#1>, target=Box.key, type=K#1), type=K#1" type=K#1
        /// @resolution.name source=key target=Box.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Box.constructor.key

    }
}

extension<K> of Box<K> where K: Equal<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=Equal<K#2>
/// @definition.method symbol=check slot=check type=(this: Box<K#2>) => boolean
/// @type.symbol symbol=K source=K type=K#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: Box<K#2>) => boolean
    /// @type.symbol symbol=check.this type=Box<K#2>

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instantiation id=probe<K#2> template=probe arguments=(K#2) owner=check
        /// @resolution.member source=this.key receiver=Box<K#2> type=K#2 kind=field target_receiver=Box<K#2> key=key target=Box.key target_type=K#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}
"#, r#"
"#);
}

/// A method where clause satisfies the bound a call inside it requires.
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Equal<in T> {
    equals(other: T): boolean;
}

declare function probe<T: Equal<T>>(value: T): boolean;

class Box<in out K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }

    check(): boolean where K: Equal<K> {
        return probe<K>(this.key);
    }
}

=== dir ===
interface Equal<T> {
/// @generic.template symbol=Equal parameters=(in T#1, this: Equal<T#1>)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(in T#1, this: Equal<T#1>)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(in out K)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out K)
/// @definition.field symbol=Box.key source="key: K" key=key type=K
/// @definition.method symbol=Box.check slot=check type=(this: Box<K>) => boolean
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(this: &'managed Box<K>, K) => Box<K>
/// @type.symbol symbol=Box.K source=K type=K

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(this: &'managed Box<K>, K) => Box<K>
    /// @type.symbol symbol=Box.constructor.this type=&'managed Box<K>
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box<K>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]
        /// @resolution.assignment source=this.key write="receiver=&'managed Box<K>, target=field(receiver=&'managed Box<K>, target=Box.key, type=K), type=K" type=K
        /// @resolution.name source=key target=Box.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Box.constructor.key

    }

    check(): boolean where K: Equal<K> {
    /// @type.symbol symbol=Box.check type=(this: Box<K>) => boolean
    /// @type.symbol symbol=Box.check.this type=Box<K>
    /// @resolution.name source=K target=Box.K
    /// @resolution.name source=Equal target=Equal
    /// @resolution.name source=K target=Box.K

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K) arguments=(provided(this.key) as K) return=boolean kind=symbol target=probe instance=probe<K>
        /// @generic.instantiation id=probe<K> template=probe arguments=(K) owner=Box.check
        /// @resolution.member source=this.key receiver=Box<K> type=K kind=field target_receiver=Box<K> key=key target=Box.key target_type=K
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<K>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}
"#, r#"
"#);
}

/// An extension method calls an unbounded generic function of the module.
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
declare function probe<T>(value: T): boolean;

class Box<in out K> {
    key: K;

    constructor(key: K) {
        this.key = key;
    }
}

extension<K> of Box<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== dir ===
declare function probe<T>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T)
/// @type.symbol symbol=probe source="declare function probe<T>(value: T): boolean" type=<T>(T) => boolean
/// @type.symbol symbol=probe.T source=T type=T
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(in out K#1)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(this: &'managed Box<K#1>, K#1) => Box<K#1>
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(this: &'managed Box<K#1>, K#1) => Box<K#1>
    /// @type.symbol symbol=Box.constructor.this type=&'managed Box<K#1>
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box<K#1>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]
        /// @resolution.assignment source=this.key write="receiver=&'managed Box<K#1>, target=field(receiver=&'managed Box<K#1>, target=Box.key, type=K#1), type=K#1" type=K#1
        /// @resolution.name source=key target=Box.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Box.constructor.key

    }
}

extension<K> of Box<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.method symbol=check slot=check type=(this: Box<K#2>) => boolean
/// @type.symbol symbol=K source=K type=K#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: Box<K#2>) => boolean
    /// @type.symbol symbol=check.this type=Box<K#2>

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instantiation id=probe<K#2> template=probe arguments=(K#2) owner=check
        /// @resolution.member source=this.key receiver=Box<K#2> type=K#2 kind=field target_receiver=Box<K#2> key=key target=Box.key target_type=K#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}
"#, r#"
"#);
}

/// A bounded extension parameter also takes the bounds of the where clause.
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
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

    constructor(key: K) {
        this.key = key;
    }
}

extension<K: Hash> of Box<K> where K: Equal<K> {
    check(): boolean {
        return probe<K>(this.key);
    }
}

=== dir ===
interface Hash {
/// @generic.template symbol=Hash parameters=(this: Hash)
/// @type.symbol symbol=Hash type=Hash
/// @definition.interface symbol=Hash template=(this: Hash)
/// @definition.where symbol=Hash relation=satisfies left=this right=Hash
/// @definition.method symbol=Hash.hash source="hash(): float64" slot=hash type=() => float64

    hash(): float64;
    /// @type.symbol symbol=Hash.hash source="hash(): float64" type=() => float64

}

interface Equal<T> {
/// @generic.template symbol=Equal parameters=(in T#1, this: Equal<T#1>)
/// @type.symbol symbol=Equal type=Equal
/// @definition.interface symbol=Equal template=(in T#1, this: Equal<T#1>)
/// @definition.where symbol=Equal relation=satisfies left=this right=Equal<T#1>
/// @definition.method symbol=Equal.equals source="equals(other: T): boolean" slot=equals type=(T#1) => boolean
/// @type.symbol symbol=Equal.T source=T type=T#1

    equals(other: T): boolean;
    /// @type.symbol symbol=Equal.equals source="equals(other: T): boolean" type=(T#1) => boolean
    /// @type.symbol symbol=Equal.equals.other source="other: T" type=T#1
    /// @resolution.name source=T target=Equal.T

}

declare function probe<T: Equal<T>>(value: T): boolean;
/// @generic.template symbol=probe parameters=(T#2: Equal<T#2>)
/// @type.symbol symbol=probe source="declare function probe<T: Equal<T>>(value: T): boolean" type=<T#2: Equal<T#2>>(T#2) => boolean
/// @type.symbol symbol=probe.T source="T: Equal<T>" type=T#2
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=probe.T
/// @resolution.name source=T target=probe.T

class Box<K> {
/// @generic.template symbol=Box parameters=(in out K#1)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out K#1)
/// @definition.field symbol=Box.key source="key: K" key=key type=K#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(this: &'managed Box<K#1>, K#1) => Box<K#1>
/// @type.symbol symbol=Box.K source=K type=K#1

    key: K;
    /// @type.symbol symbol=Box.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

    constructor(key: K) {
    /// @type.symbol symbol=Box.constructor type=(this: &'managed Box<K#1>, K#1) => Box<K#1>
    /// @type.symbol symbol=Box.constructor.this type=&'managed Box<K#1>
    /// @type.symbol symbol=Box.constructor.key source="key: K" type=K#1
    /// @resolution.name source=K target=Box.K

        this.key = key;
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box<K#1>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.key kind=place
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]
        /// @resolution.assignment source=this.key write="receiver=&'managed Box<K#1>, target=field(receiver=&'managed Box<K#1>, target=Box.key, type=K#1), type=K#1" type=K#1
        /// @resolution.name source=key target=Box.constructor.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=Box.constructor.key

    }
}

extension<K: Hash> of Box<K> where K: Equal<K> {
/// @generic.template symbol=<module>#2 parameters=(K#2: Hash)
/// @definition.extension symbol=<module>#2 form=local target=Box<K#2>
/// @definition.where symbol=<module>#2 source="K: Equal<K>" relation=satisfies left=K#2 right=Equal<K#2>
/// @definition.method symbol=check slot=check type=(this: Box<K#2>) => boolean
/// @type.symbol symbol=K source="K: Hash" type=K#2
/// @resolution.name source=Hash target=Hash
/// @resolution.name source=Box target=Box
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=K target=K

    check(): boolean {
    /// @type.symbol symbol=check type=(this: Box<K#2>) => boolean
    /// @type.symbol symbol=check.this type=Box<K#2>

        return probe(this.key);
        /// @resolution.name source=probe target=probe
        /// @resolution.call source=probe(this.key) parameters=(K#2) arguments=(provided(this.key) as K#2) return=boolean kind=symbol target=probe instance=probe<K#2>
        /// @generic.instantiation id=probe<K#2> template=probe arguments=(K#2) owner=check
        /// @resolution.member source=this.key receiver=Box<K#2> type=K#2 kind=field target_receiver=Box<K#2> key=key target=Box.key target_type=K#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Box<K#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.key placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.key root=this keys=[key]

    }
}
"#, r#"
"#);
}

/// A declared bound and a where bound intersect to constrain the return value.
#[test]
fn test_intersected_parameter_bounds_constrain_return_values() {
    let session = TestSession::single(
        r#"
function active<T: boolean | string>(value: T): boolean where T: boolean {
    return value;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function active<T: boolean | string>(value: T): boolean where T: boolean {
    return value;
}

=== dir ===
function active<T: boolean | string>(value: T): boolean where T: boolean {
/// @generic.template symbol=active parameters=(T: boolean | string)
/// @type.symbol symbol=active type=<T: boolean | string>(T) => boolean
/// @type.symbol symbol=active.T source="T: boolean | string" type=T
/// @type.symbol symbol=active.value source="value: T" type=T
/// @resolution.name source=T target=active.T
/// @resolution.name source=T target=active.T

    return value;
    /// @resolution.name source=value target=active.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=active.value

}
"#,
    );
}

/// A member selects through the interface a where clause bounds the receiver with.
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Doubling {
    double(): int32;
}

function twice<T>(value: T): int32 where T: Doubling {
    return value.double();
}

=== dir ===
interface Doubling {
/// @generic.template symbol=Doubling parameters=(this: Doubling)
/// @type.symbol symbol=Doubling type=Doubling
/// @definition.interface symbol=Doubling template=(this: Doubling)
/// @definition.where symbol=Doubling relation=satisfies left=this right=Doubling
/// @definition.method symbol=Doubling.double source="double(): int32" slot=double type=() => int32

    double(): int32;
    /// @type.symbol symbol=Doubling.double source="double(): int32" type=() => int32

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
    /// @resolution.member source=value.double receiver=T type=() => int32 kind=symbol target_receiver=T target=Doubling.double
    /// @resolution.call source=value.double() parameters=() return=int32 kind=symbol target=Doubling.double receiver=T
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=twice.value

}
"#,
    );
}

/// A static member call infers the parameters of an exported extension.
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

    session.assert_dir("main.tspp", DirRows::checked(), r#"
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

=== dir ===
declare function todo(message: string): never;
/// @type.symbol symbol=todo source="declare function todo(message: string): never" type=(string) => never

newtype Inner<T> = intrinsic;
/// @generic.template symbol=Inner parameters=(in out T#1)
/// @type.symbol symbol=Inner source="newtype Inner<T> = intrinsic" type=Inner
/// @definition.newtype symbol=Inner source="newtype Inner<T> = intrinsic" template=(in out T#1) backing=intrinsic constructors=[<T#1>(intrinsic) => Inner<T#1>]
/// @type.symbol symbol=Inner.T source=T type=T#1

extension<T> of Inner<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @generic.instance id=Inner<T#2> template=Inner arguments=(T#2)
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
    /// @generic.instance id=Inner<T#3> template=Inner arguments=(T#3)
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=Cell.T

}

export extension<T> of Cell<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @generic.instance id=Cell<T#4> template=Cell arguments=(T#4)
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
        /// @resolution.call source=Inner.new(value) parameters=(T#4) arguments=(provided(value) as T#4) return=Inner<T#4> kind=symbol target=new#1 instance=Inner<T#4>.<extension#1>.new#1
        /// @generic.instantiation id=new#1<T#4> template=new#1 arguments=(T#4) owner=new#2
        /// @generic.instance id=Inner<T#4> template=Inner arguments=(T#4)
        /// @generic.instance id=new#1<T#4> template=new#1 arguments=(T#4)
        /// @resolution.name source=value target=new.value#2
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=new.value#2

    }
}
"#);
}

/// A where equality rejects arguments that are only assignable one way.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function requireEqual<T, U>(left: T, right: U): void where T == U {}

declare const wider: { x: int32; y: string };
declare const narrower: { x: int32 };

requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower);

=== dir ===
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
/// @type.symbol symbol=x#1 source="x: int32" type=int32
/// @type.symbol symbol=y#1 source="y: string" type=string

declare const narrower: { x: int32 };
/// @type.symbol symbol=narrower source=narrower type={ x: int32 }
/// @resolution.pattern source=narrower kind=binding target=narrower
/// @type.symbol symbol=x#2 source="x: int32" type=int32

requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower)" parameters=({ x: int32; y: string }, { x: int32 }) arguments=(provided(wider) as { x: int32; y: string }, provided(narrower) as { x: int32 }) return=void kind=symbol target=requireEqual instance="requireEqual<{ x: int32; y: string }, { x: int32 }>"
/// @generic.instantiation id="requireEqual<{ x: int32; y: string }, { x: int32 }>" template=requireEqual arguments=({ x: int32; y: string }, { x: int32 })
/// @type.symbol symbol=x#3 source="x: int32" type=int32
/// @type.symbol symbol=y#2 source="y: string" type=string
/// @type.symbol symbol=x#4 source="x: int32" type=int32
/// @resolution.name source=wider target=wider
/// @resolution.place source=wider placement="local" lifetime="static" access="immutable"
/// @resolution.access source=wider root=wider
/// @resolution.name source=narrower target=narrower
/// @resolution.place source=narrower placement="local" lifetime="static" access="immutable"
/// @resolution.access source=narrower root=narrower
"#,
        r#"
/// @diagnostic.error id=equality-requirement-not-satisfied message="equality requirement '{ x: int32; y: string } == { x: int32 }' is not satisfied"
/// @diagnostic.label line=7 column=1 span="requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower)" line_source="requireEqual<{ x: int32; y: string }, { x: int32 }>(wider, narrower);"
"#,
    );
}

/// A where equality infers one common type for both parameters.
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function requireEqual<T, U>(left: T, right: U): void where T == U {}

declare const wider: { x: int32; y: string };
declare const narrower: { x: int32 };

requireEqual<{ x: int32; y: string } | { x: int32 }, { x: int32; y: string } | { x: int32 }>(
    wider as { x: int32; y: string } | { x: int32 },
    narrower as { x: int32; y: string } | { x: int32 },
);

=== dir ===
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
/// @type.symbol symbol=x#1 source="x: int32" type=int32
/// @type.symbol symbol=y source="y: string" type=string

declare const narrower: { x: int32 };
/// @type.symbol symbol=narrower source=narrower type={ x: int32 }
/// @resolution.pattern source=narrower kind=binding target=narrower
/// @type.symbol symbol=x#2 source="x: int32" type=int32

requireEqual(wider, narrower);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(wider, narrower)" parameters=({ x: int32; y: string } | { x: int32 }, { x: int32; y: string } | { x: int32 }) arguments=(provided(wider) as { x: int32; y: string } | { x: int32 }, provided(narrower) as { x: int32; y: string } | { x: int32 }) return=void kind=symbol target=requireEqual instance="requireEqual<{ x: int32; y: string } | { x: int32 }, { x: int32; y: string } | { x: int32 }>"
/// @generic.instantiation id="requireEqual<{ x: int32; y: string } | { x: int32 }, { x: int32; y: string } | { x: int32 }>" template=requireEqual arguments=({ x: int32; y: string } | { x: int32 }, { x: int32; y: string } | { x: int32 })
/// @generic.instance id="requireEqual<{ x: int32; y: string } | { x: int32 }, { x: int32; y: string } | { x: int32 }>" template=requireEqual arguments=({ x: int32; y: string } | { x: int32 }, { x: int32; y: string } | { x: int32 })
/// @resolution.name source=wider target=wider
/// @resolution.place source=wider placement="local" lifetime="static" access="immutable"
/// @resolution.access source=wider root=wider
/// @resolution.name source=narrower target=narrower
/// @resolution.place source=narrower placement="local" lifetime="static" access="immutable"
/// @resolution.access source=narrower root=narrower
"#,
    );
}

/// A static member selects through the interface a parameter bound names.
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

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Makeable {
    static make(): this;
}

function build<T: Makeable>(): T {
    T.make()
}

=== dir ===
interface Makeable {
/// @generic.template symbol=Makeable parameters=(this: Makeable)
/// @type.symbol symbol=Makeable type=Makeable
/// @definition.interface symbol=Makeable template=(this: Makeable)
/// @definition.where symbol=Makeable relation=satisfies left=this right=Makeable
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
    /// @resolution.call source=T.make() parameters=() return=T kind=symbol target=Makeable.make
    /// @generic.instantiation id=Makeable.make<T> template=Makeable.make arguments=() owner=build
    /// @generic.instance id=Makeable.make<T> template=Makeable.make arguments=()

}
"#);
}

/// A static member selects through every arm of a union parameter bound.
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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
interface Zero {
/// @generic.template symbol=Zero parameters=(this: Zero)
/// @type.symbol symbol=Zero type=Zero
/// @definition.interface symbol=Zero template=(this: Zero)
/// @definition.where symbol=Zero relation=satisfies left=this right=Zero
/// @definition.method symbol=Zero.zero source="static zero(): this" slot=zero static=true type=() => this

    static zero(): this;
    /// @type.symbol symbol=Zero.zero source="static zero(): this" type=() => this

}

interface Integer extends Zero {}
/// @generic.template symbol=Integer parameters=(this: Integer)
/// @type.symbol symbol=Integer source="interface Integer extends Zero {}" type=Integer
/// @definition.interface symbol=Integer source="interface Integer extends Zero {}" template=(this: Integer)
/// @definition.where symbol=Integer source="interface Integer extends Zero {}" relation=satisfies left=this right=Integer
/// @definition.extends symbol=Integer source=Zero target=Zero
/// @resolution.name source=Zero target=Zero

interface Float extends Zero {}
/// @generic.template symbol=Float parameters=(this: Float)
/// @type.symbol symbol=Float source="interface Float extends Zero {}" type=Float
/// @definition.interface symbol=Float source="interface Float extends Zero {}" template=(this: Float)
/// @definition.where symbol=Float source="interface Float extends Zero {}" relation=satisfies left=this right=Float
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
    /// @resolution.call source=T.zero() parameters=() return=T kind=symbol target=Zero.zero
    /// @generic.instantiation id=Zero.zero<T> template=Zero.zero arguments=() owner=zero
    /// @generic.instance id=Zero.zero<T> template=Zero.zero arguments=()

}
"#,
    );
}

/// A static member call infers the parameters of the extension declaring it.
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

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
class Box<in out T> {
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
        Box.make<T>(value)
    }
}

=== dir ===
class Box<T> {
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(this: &'managed Box<T#1>, T#1) => Box<T#1>
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

    constructor(value: T) {
    /// @type.symbol symbol=Box.constructor type=(this: &'managed Box<T#1>, T#1) => Box<T#1>
    /// @type.symbol symbol=Box.constructor.this type=&'managed Box<T#1>
    /// @type.symbol symbol=Box.constructor.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box<T#1>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Box<T#1>, target=field(receiver=&'managed Box<T#1>, target=Box.value, type=T#1), type=T#1" type=T#1
        /// @resolution.name source=value target=Box.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value

    }
}

extension<T> of Box<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
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
        /// @generic.instantiation id=Box.constructor<T#2> template=Box.constructor arguments=(T#2) owner=make
        /// @generic.instantiation id=Box<T#2> template=Box arguments=(T#2) owner=make
        /// @generic.instance id=Box.constructor<T#2> template=Box.constructor arguments=(T#2)
        /// @resolution.name source=Box target=Box
        /// @resolution.name source=T target=T#1
        /// @resolution.name source=value target=make.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=make.value

    }
}

extension<T> of Box<T> {
/// @generic.template symbol=<module>#3 parameters=(T#3)
/// @generic.instance id=Box<T#3> template=Box arguments=(T#3)
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
        /// @resolution.member source=Box.make receiver=typeof Box type=(T#2) => Box<T#2> kind=symbol target_receiver=typeof Box target=make
        /// @resolution.call source=Box.make(value) parameters=(T#3) arguments=(provided(value) as T#3) return=Box<T#3> kind=symbol target=make instance=Box<T#3>.<extension#1>.make
        /// @generic.instantiation id=make<T#3> template=make arguments=(T#3) owner=wrap
        /// @generic.instance id=Box.constructor<T#3> template=Box.constructor arguments=(T#3)
        /// @generic.instance id=make<T#3> template=make arguments=(T#3)
        /// @resolution.name source=value target=wrap.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=wrap.value

    }
}
"#);
}

/// An expected field type infers the arguments of a static member call.
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

    session.assert_dir("main.tspp", DirRows::checked(), r#"
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

=== dir ===
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
/// @generic.instance id=Inner<T#2> template=Inner arguments=(T#2)
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
    /// @generic.instance id=Inner<T#3> template=Inner arguments=(T#3)
    /// @resolution.name source=Inner target=Inner
    /// @resolution.name source=T target=Outer.T

}

extension<T> of Outer<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @generic.instance id=Outer<T#4> template=Outer arguments=(T#4)
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
        /// @resolution.call source=Inner.new(value) parameters=(T#4) arguments=(provided(value) as T#4) return=Inner<T#4> kind=symbol target=new#1 instance=Inner<T#4>.<extension#1>.new#1
        /// @generic.instantiation id=new#1<T#4> template=new#1 arguments=(T#4) owner=new#2
        /// @generic.instance id=Inner<T#4> template=Inner arguments=(T#4)
        /// @generic.instance id=new#1<T#4> template=new#1 arguments=(T#4)
        /// @resolution.name source=value target=new.value#2
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=new.value#2

    }
}
"#);
}

/// A generic call result assigns into a union return type.
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

    session.assert_dir("main.tspp", DirRows::checked(), r#"
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

=== dir ===
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
    /// @generic.instantiation id=pair<T#2> template=pair arguments=(T#2) owner=check
    /// @generic.instance id=pair<T#2> template=pair arguments=(T#2)
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
"#);
}

/// A static member on a bound infers the method's own type argument.
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

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Iterator<out T, out R = void> {}

interface FromIterator<in T> {
    static fromIterator<R>(values: Iterator<T, R>): this;
}

extension<T, R, I: Iterator<T, R>> of I {
    collect<C>(): C where C: FromIterator<T> {
        C.fromIterator<T, R>(this as Iterator<T, R>)
    }
}

=== dir ===
interface Iterator<out T, out R = void> {}
/// @generic.template symbol=Iterator parameters=(out T#1, out R#1 = void, this: Iterator<T#1, R#1>)
/// @type.symbol symbol=Iterator source="interface Iterator<out T, out R = void> {}" type=Iterator
/// @definition.interface symbol=Iterator source="interface Iterator<out T, out R = void> {}" template=(out T#1, out R#1 = void, this: Iterator<T#1, R#1>)
/// @definition.where symbol=Iterator source="interface Iterator<out T, out R = void> {}" relation=satisfies left=this right=Iterator<T#1, R#1>
/// @type.symbol symbol=Iterator.T source="out T" type=T#1
/// @type.symbol symbol=Iterator.R source="out R = void" type=R#1

interface FromIterator<T> {
/// @generic.template symbol=FromIterator parameters=(in T#2, this: FromIterator<T#2>)
/// @type.symbol symbol=FromIterator type=FromIterator
/// @definition.interface symbol=FromIterator template=(in T#2, this: FromIterator<T#2>)
/// @definition.where symbol=FromIterator relation=satisfies left=this right=FromIterator<T#2>
/// @definition.method symbol=FromIterator.fromIterator source="static fromIterator<R>(values: Iterator<T, R>): this" slot=fromIterator static=true type=<R#2>(Iterator<T#2, R#2>) => this
/// @type.symbol symbol=FromIterator.T source=T type=T#2

    static fromIterator<R>(values: Iterator<T, R>): this;
    /// @generic.template symbol=FromIterator.fromIterator parent=template#1 parameters=(R#2)
    /// @type.symbol symbol=FromIterator.fromIterator source="static fromIterator<R>(values: Iterator<T, R>): this" type=<R#2>(Iterator<T#2, R#2>) => this
    /// @generic.instance id="Iterator<T#2, R#2>" template=Iterator arguments=(T#2, R#2)
    /// @type.symbol symbol=FromIterator.fromIterator.R source=R type=R#2
    /// @type.symbol symbol=FromIterator.fromIterator.values source="values: Iterator<T, R>" type=Iterator<T#2, R#2>
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=FromIterator.T
    /// @resolution.name source=R target=FromIterator.fromIterator.R

}

extension<T, R, I: Iterator<T, R>> of I {
/// @generic.template symbol=<module>#2 parameters=(T#3, R#3, I: Iterator<T#3, R#3>)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.method symbol=collect slot=collect type=<C>(this: I) => C
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=R source=R type=R#3
/// @type.symbol symbol=I source="I: Iterator<T, R>" type=I
/// @resolution.name source=Iterator target=Iterator
/// @generic.instance id="Iterator<T#3, R#3>" template=Iterator arguments=(T#3, R#3)
/// @resolution.name source=T target=T
/// @resolution.name source=R target=R
/// @resolution.name source=I target=I

    collect<C>(): C where C: FromIterator<T> {
    /// @generic.template symbol=collect parent=template#2 parameters=(C)
    /// @type.symbol symbol=collect type=<C>(this: I) => C
    /// @type.symbol symbol=collect.this type=I
    /// @type.symbol symbol=collect.C source=C type=C
    /// @resolution.name source=C target=collect.C
    /// @resolution.name source=C target=collect.C
    /// @resolution.name source=FromIterator target=FromIterator
    /// @generic.instance id=FromIterator<T#3> template=FromIterator arguments=(T#3)
    /// @resolution.name source=T target=T

        C.fromIterator(this)
        /// @resolution.name source=C target=collect.C
        /// @resolution.member source=C.fromIterator receiver=C type=<R#2>(Iterator<T#3, R#2>) => C kind=symbol target_receiver=C target=FromIterator.fromIterator
        /// @resolution.call source=C.fromIterator(this) parameters=(Iterator<T#3, R#3>) arguments=(provided(this) as Iterator<T#3, R#3>) return=C kind=symbol target=FromIterator.fromIterator instance=FromIterator<T#3>.fromIterator<R#3>
        /// @generic.instantiation id="FromIterator.fromIterator<C, T#3, R#3>" template=FromIterator.fromIterator arguments=(T#3, R#3) owner=collect
        /// @generic.instantiation id=FromIterator.fromIterator<T#3> template=FromIterator.fromIterator arguments=(T#3) owner=collect
        /// @generic.instance id="FromIterator.fromIterator<C, T#3, R#3>" template=FromIterator.fromIterator arguments=(T#3, R#3)
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

    }
}
"#);
}

/// A declaration naming lifetimes keeps its written parameter arity.
#[test]
fn test_keep_written_arity_for_named_lifetimes() {
    let session = TestSession::single(
        r#"
struct Named<'a> {
    first: &'a string;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Named<'a> {
    first: &'a string;
}

=== dir ===
struct Named<'a> {
/// @generic.template symbol=Named parameters=('a)
/// @type.symbol symbol=Named type=Named
/// @definition.struct symbol=Named template=('a)
/// @definition.field symbol=Named.first source="first: &'a string" key=first type=&'a string
/// @type.symbol symbol=Named.'a source='a type='a

    first: &'a string;
    /// @type.symbol symbol=Named.first source="first: &'a string" type=&'a string
    /// @resolution.name source='a target=Named.'a

}
"#,
    );
}

/// An elided borrow inside a lifetime-naming declaration reports a diagnostic.
#[test]
fn test_reject_elided_borrows_in_lifetime_naming_declarations() {
    let session = TestSession::single(
        r#"
struct Mixed<'a> {
    first: &'a string;
    second: &string;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Mixed<'a> {
    first: &'a string;
    second: &string;
}

=== dir ===
struct Mixed<'a> {
/// @generic.template symbol=Mixed parameters=('a)
/// @type.symbol symbol=Mixed type=Mixed
/// @definition.struct symbol=Mixed template=('a)
/// @definition.field symbol=Mixed.first source="first: &'a string" key=first type=&'a string
/// @definition.field symbol=Mixed.second source="second: &string" key=second type=Borrowed<string, <error>, "mutable">
/// @type.symbol symbol=Mixed.'a source='a type='a

    first: &'a string;
    /// @type.symbol symbol=Mixed.first source="first: &'a string" type=&'a string
    /// @resolution.name source='a target=Mixed.'a

    second: &string;
    /// @type.symbol symbol=Mixed.second source="second: &string" type=Borrowed<string, <error>, "mutable">

}
"#,
        r#"
/// @diagnostic.error id=elided-declaration-lifetime message="type declaration 'Mixed' writes its lifetimes"
/// @diagnostic.label line=4 column=13 span="&" line_source="second: &string;"
/// @diagnostic.help message="declare the lifetime parameter and name it, like &'a"
"#,
    );
}

/// A where equality rejects a bounded parameter passed as its argument.
#[test]
fn test_where_equality_rejects_a_bounded_parameter_argument() {
    let session = TestSession::single(
        r#"
function requireExact<T>(value: T): void where T == int32 {}

function forward<U: int32>(value: U): void {
    requireExact<U>(value);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function requireExact<T>(value: T): void where T == int32 {}

function forward<U: int32>(value: U): void {
    requireExact<U>(value);
}

=== dir ===
function requireExact<T>(value: T): void where T == int32 {}
/// @generic.template symbol=requireExact parameters=(T)
/// @type.symbol symbol=requireExact source="function requireExact<T>(value: T): void where T == int32 {}" type=<T>(T) => void
/// @type.symbol symbol=requireExact.T source=T type=T
/// @type.symbol symbol=requireExact.value source="value: T" type=T
/// @resolution.name source=T target=requireExact.T
/// @resolution.name source=T target=requireExact.T

function forward<U: int32>(value: U): void {
/// @generic.template symbol=forward parameters=(U: int32)
/// @type.symbol symbol=forward type=<U: int32>(U) => void
/// @type.symbol symbol=forward.U source="U: int32" type=U
/// @type.symbol symbol=forward.value source="value: U" type=U
/// @resolution.name source=U target=forward.U

    requireExact<U>(value);
    /// @resolution.name source=requireExact target=requireExact
    /// @resolution.call source=requireExact<U>(value) parameters=(U) arguments=(provided(value) as U) return=void kind=symbol target=requireExact instance=requireExact<U>
    /// @generic.instantiation id=requireExact<U> template=requireExact arguments=(U) owner=forward
    /// @resolution.name source=U target=forward.U
    /// @resolution.name source=value target=forward.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=forward.value

}
"#,
        r#"
/// @diagnostic.error id=equality-requirement-not-satisfied message="equality requirement 'U == int32' is not satisfied"
/// @diagnostic.label line=5 column=5 span="requireExact<U>(value)" line_source="requireExact<U>(value);"
"#,
    );
}

/// A subtract expression projects the output of an imported extension.
#[test]
fn test_project_the_output_of_an_imported_subtract_extension() {
    let session = TestSession::builder()
        .module(
            "a.tspp",
            r#"
import { Subtract } from "tspp:ops";

export interface Numericish {}

export class Vec<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    length(): T {
        this.value
    }
}

export extension<T: Numericish> of Vec<T> implements Subtract<Vec<T>> {
    type Output = Vec<T>;

    subtract(other: Vec<T>): this.Output {
        this
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Vec, Numericish } from "./a.tspp";

function f<T: Numericish>(a: Vec<T>, b: Vec<T>): T {
    (a - b).length()
}
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Numericish, Vec } from "./a.tspp";

function f<T: Numericish>(a: Vec<T>, b: Vec<T>): T {
    (a - b).length<T>()
}

=== dir ===
import { Vec, Numericish } from "./a.tspp";

function f<T: Numericish>(a: Vec<T>, b: Vec<T>): T {
/// @generic.template symbol=f parameters=(T: a.Numericish)
/// @type.symbol symbol=f type=<T: a.Numericish>(a.Vec<T>, a.Vec<T>) => T
/// @generic.instance id=a.Vec<T> template=a.Vec arguments=(T)
/// @type.symbol symbol=f.T source="T: Numericish" type=T
/// @resolution.name source=Numericish target=a.Numericish
/// @type.symbol symbol=f.a source="a: Vec<T>" type=a.Vec<T>
/// @resolution.name source=Vec target=a.Vec
/// @resolution.name source=T target=f.T
/// @type.symbol symbol=f.b source="b: Vec<T>" type=a.Vec<T>
/// @resolution.name source=Vec target=a.Vec
/// @resolution.name source=T target=f.T
/// @resolution.name source=T target=f.T

    (a - b).length()
    /// @resolution.member source="(a - b).length" receiver=a.Vec<T> type=(this: a.Vec<T>) => T kind=symbol target_receiver=a.Vec<T> target=a.Vec.length
    /// @resolution.call source=(a - b).length() parameters=() return=T kind=symbol target=a.Vec.length receiver=a.Vec<T> instance=a.Vec<T>.length
    /// @generic.instantiation id=a.Vec.length<T> template=a.Vec.length arguments=(T) owner=f
    /// @generic.instance id=a.Vec.length<T> template=a.Vec.length arguments=(T)
    /// @resolution.name source=a target=f.a
    /// @resolution.operator source="a - b" type=a.Vec<T> operator="-" kind=call parameters=(a.Vec<T>) arguments=(provided(b) as a.Vec<T>) return=a.Vec<T> kind=symbol target=a.subtract receiver=a.Vec<T> instance=a.Vec<T>.<extension#1>.subtract
    /// @resolution.place source=a placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=a root=f.a
    /// @generic.instantiation id=a.subtract<T> template=a.subtract arguments=(T) owner=f
    /// @generic.instance id=a.subtract<T> template=a.subtract arguments=(T)
    /// @resolution.name source=b target=f.b
    /// @resolution.place source=b placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=b root=f.b

}
"#,
    );
}

/// Prove a readonly array bound through the parameter's own readonly array bound.
#[test]
fn test_prove_a_readonly_array_bound_through_the_parameter_bound() {
    let session = TestSession::single(
        r#"
import { Iterable } from "tspp:iter";

export newtype interface Parameterized<P: readonly unknown[]> {
    (name: string, body?: Function<P, void>): void;
    readonly skip: Parameterized<P>;
}

export newtype interface Suite {
    each<P: readonly unknown[]>(values: Iterable<P>): Parameterized<P>;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Iterable } from "tspp:iter";

export newtype interface Parameterized<P: readonly unknown[]> {
    (name: string, body?: Function<P, void>): void;
    readonly skip: Parameterized<P>;
}

export newtype interface Suite {
    each<P: readonly unknown[]>(values: Iterable<P>): Parameterized<P>;
}

=== dir ===
import { Iterable } from "tspp:iter";

export newtype interface Parameterized<P: readonly unknown[]> {
/// @generic.template symbol=Parameterized parameters=(P#1: readonly unknown[], this: Parameterized<P#1>)
/// @type.symbol symbol=Parameterized type=Parameterized
/// @definition.interface symbol=Parameterized template=(P#1: readonly unknown[], this: Parameterized<P#1>) nominal=true
/// @definition.where symbol=Parameterized relation=satisfies left=this right=Parameterized<P#1>
/// @definition.field symbol=Parameterized.skip source="readonly skip: Parameterized<P>" key=skip type=Parameterized<P#1>
/// @definition.signature kind=call source="(name: string, body?: Function<P, void>): void" type=(string, Function<P#1, void> | undefined?) => void
/// @type.symbol symbol=Parameterized.P source="P: readonly unknown[]" type=P#1

    (name: string, body?: Function<P, void>): void;
    /// @type.symbol symbol=Parameterized.name source="name: string" type=string
    /// @type.symbol symbol=Parameterized.body source="body?: Function<P, void>" type=Function<P#1, void, "mutable"> | undefined
    /// @resolution.name source=Function target=Function
    /// @resolution.name source=P target=Parameterized.P

    readonly skip: Parameterized<P>;
    /// @type.symbol symbol=Parameterized.skip source="readonly skip: Parameterized<P>" type=Parameterized<P#1>
    /// @resolution.name source=Parameterized target=Parameterized
    /// @resolution.name source=P target=Parameterized.P

}

export newtype interface Suite {
/// @generic.template symbol=Suite parameters=(this: Suite)
/// @type.symbol symbol=Suite type=Suite
/// @definition.interface symbol=Suite template=(this: Suite) nominal=true
/// @definition.where symbol=Suite relation=satisfies left=this right=Suite
/// @definition.method symbol=Suite.each source="each<P: readonly unknown[]>(values: Iterable<P>): Parameterized<P>" slot=each type=<P#2: readonly unknown[]>(Iterable<P#2>) => Parameterized<P#2>

    each<P: readonly unknown[]>(values: Iterable<P>): Parameterized<P>;
    /// @generic.template symbol=Suite.each parent=template#1 parameters=(P#2: readonly unknown[])
    /// @type.symbol symbol=Suite.each source="each<P: readonly unknown[]>(values: Iterable<P>): Parameterized<P>" type=<P#2: readonly unknown[]>(Iterable<P#2>) => Parameterized<P#2>
    /// @type.symbol symbol=Suite.each.P source="P: readonly unknown[]" type=P#2
    /// @type.symbol symbol=Suite.each.values source="values: Iterable<P>" type=Iterable<P#2>
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=P target=Suite.each.P
    /// @resolution.name source=Parameterized target=Parameterized
    /// @resolution.name source=P target=Suite.each.P

}
"#, r#"

"#);
}

/// Prove a function parameter list bound through one element of an intersection bound.
#[test]
fn test_prove_a_function_parameter_bound_through_an_intersection_bound() {
    let session = TestSession::single(
        r#"
import { Copy } from "tspp:memory";

export newtype interface Parameterized<P: readonly unknown[] & Copy> {
    (name: string, body?: Function<P, void>): void;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Copy } from "tspp:memory";

export newtype interface Parameterized<P: readonly unknown[] & Copy> {
    (name: string, body?: Function<P, void>): void;
}

=== dir ===
import { Copy } from "tspp:memory";

export newtype interface Parameterized<P: readonly unknown[] & Copy> {
/// @generic.template symbol=Parameterized parameters=(P: readonly unknown[] & Copy, this: Parameterized<P>)
/// @type.symbol symbol=Parameterized type=Parameterized
/// @definition.interface symbol=Parameterized template=(P: readonly unknown[] & Copy, this: Parameterized<P>) nominal=true
/// @definition.where symbol=Parameterized relation=satisfies left=this right=Parameterized<P>
/// @definition.signature kind=call source="(name: string, body?: Function<P, void>): void" type=(string, Function<P, void> | undefined?) => void
/// @type.symbol symbol=Parameterized.P source="P: readonly unknown[] & Copy" type=P
/// @resolution.name source=Copy target=Copy

    (name: string, body?: Function<P, void>): void;
    /// @type.symbol symbol=Parameterized.name source="name: string" type=string
    /// @type.symbol symbol=Parameterized.body source="body?: Function<P, void>" type=Function<P, void, "mutable"> | undefined
    /// @resolution.name source=Function target=Function
    /// @resolution.name source=P target=Parameterized.P

}
"#, r#"

"#);
}

/// Satisfy a property key bound with a parameter bounded by keyof.
#[test]
fn test_satisfy_a_property_key_bound_with_a_keyof_parameter() {
    let session = TestSession::single(
        r#"
export newtype interface Test<TestValues = {}> {
    override<const Name: keyof TestValues>(name: Name, value: TestValues[Name]): Omit<TestValues, Name>;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
export newtype interface Test<in out TestValues = {}> {
    override<const Name: keyof TestValues>(
        name: Name,
        value: TestValues[Name],
    ): Omit<TestValues, Name>;
}

=== dir ===
export newtype interface Test<TestValues = {}> {
/// @generic.template symbol=Test parameters=(in out TestValues = {}, this: Test<TestValues>)
/// @type.symbol symbol=Test type=Test
/// @definition.interface symbol=Test template=(in out TestValues = {}, this: Test<TestValues>) nominal=true
/// @definition.where symbol=Test relation=satisfies left=this right=Test<TestValues>
/// @definition.method symbol=Test.override slot=override type=<const Name: keyof TestValues>(Name, TestValues[Name]) => Omit<TestValues, Name>
/// @type.symbol symbol=Test.TestValues source="TestValues = {}" type=TestValues

    override<const Name: keyof TestValues>(name: Name, value: TestValues[Name]): Omit<TestValues, Name>;
    /// @generic.template symbol=Test.override parent=template#0 parameters=(const Name: keyof TestValues)
    /// @type.symbol symbol=Test.override type=<const Name: keyof TestValues>(Name, TestValues[Name]) => Omit<TestValues, Name>
    /// @type.symbol symbol=Test.override.Name source="const Name: keyof TestValues" type=Name
    /// @resolution.name source=TestValues target=Test.TestValues
    /// @type.symbol symbol=Test.override.name source="name: Name" type=Name
    /// @resolution.name source=Name target=Test.override.Name
    /// @type.symbol symbol=Test.override.value source="value: TestValues[Name]" type=TestValues[Name]
    /// @resolution.name source=TestValues target=Test.TestValues
    /// @resolution.name source=Name target=Test.override.Name
    /// @resolution.name source=Omit target=Omit
    /// @resolution.name source=TestValues target=Test.TestValues
    /// @resolution.name source=Name target=Test.override.Name

}
"#, r#"
"#);
}

/// Assign a keyof over a parameter into the union adding undefined to the same keyof.
#[test]
fn test_assign_a_keyof_into_its_optional_union() {
    let session = TestSession::single(
        r#"
function pick<T>(key: keyof T): keyof T | undefined {
    return key;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function pick<T>(key: keyof T): keyof T | undefined {
    return key as keyof T | undefined;
}

=== dir ===
function pick<T>(key: keyof T): keyof T | undefined {
/// @generic.template symbol=pick parameters=(T)
/// @type.symbol symbol=pick type=<T>(keyof T) => keyof T | undefined
/// @type.symbol symbol=pick.T source=T type=T
/// @type.symbol symbol=pick.key source="key: keyof T" type=keyof T
/// @resolution.name source=T target=pick.T
/// @resolution.name source=T target=pick.T

    return key;
    /// @resolution.name source=key target=pick.key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=pick.key

}
"#,
        r#"
"#,
    );
}

/// Materialize an integer literal at a rigid integer parameter.
#[test]
fn test_materialize_a_literal_at_a_rigid_integer_parameter() {
    let session = TestSession::single(
        r#"
import { Integer } from "tspp:math";

function unit<T: Integer>(): T {
    return 1;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Integer } from "tspp:math";

function unit<T: Integer>(): T {
    return 1;
}

=== dir ===
import { Integer } from "tspp:math";

function unit<T: Integer>(): T {
/// @generic.template symbol=unit parameters=(T: Integer)
/// @type.symbol symbol=unit type=<T: Integer>() => T
/// @type.symbol symbol=unit.T source="T: Integer" type=T
/// @resolution.name source=Integer target=Integer
/// @resolution.name source=T target=unit.T

    return 1;
}
"#,
        r#"
"#,
    );
}
