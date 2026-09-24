use crate::tests::{DirRows, TestSession};

#[test]
fn test_mapped_type_projects_each_source_key() {
    let session = TestSession::single(
        r#"
type Flags<T> = { [K in keyof T]: boolean };
type Actual = Flags<{ name: string; age: int32 }>;

declare const value: Actual;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags<T> = { [K in keyof T]: boolean };
type Actual = Flags<{ name: string; age: int32 }>;

declare const value: Actual;

=== dir ===
type Flags<T> = { [K in keyof T]: boolean };
/// @generic.template symbol=Flags parameters=(T)
/// @type.symbol symbol=Flags source="type Flags<T> = { [K in keyof T]: boolean }" type={ [K in keyof T]: boolean }
/// @definition.type symbol=Flags source="type Flags<T> = { [K in keyof T]: boolean }" template=(T) value={ [K in keyof T]: boolean }
/// @type.symbol symbol=Flags.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=Flags.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Flags.T

type Actual = Flags<{ name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Flags<{ name: string; age: int32 }>" type={ name: boolean; age: boolean }
/// @definition.type symbol=Actual source="type Actual = Flags<{ name: string; age: int32 }>" value=Flags<{ name: string; age: int32 }>
/// @resolution.name source=Flags target=Flags
/// @type.symbol symbol=Actual.name source="name: string" type=string
/// @type.symbol symbol=Actual.age source="age: int32" type=int32

declare const value: Actual;
/// @type.symbol symbol=value source=value type=Actual
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Actual target=Actual
"#,
    );
}

#[test]
fn test_mapped_type_carries_source_field_modifiers() {
    let session = TestSession::single(
        r#"
type Clone<T> = { [K in keyof T]: T[K] };
type Actual = Clone<{ readonly name: string; age?: int32 }>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Clone<T> = { [K in keyof T]: T[K] };
type Actual = Clone<{ readonly name: string; age?: int32 }>;

=== dir ===
type Clone<T> = { [K in keyof T]: T[K] };
/// @generic.template symbol=Clone parameters=(T)
/// @type.symbol symbol=Clone source="type Clone<T> = { [K in keyof T]: T[K] }" type={ [K in keyof T]: T[K] }
/// @definition.type symbol=Clone source="type Clone<T> = { [K in keyof T]: T[K] }" template=(T) value={ [K in keyof T]: T[K] }
/// @type.symbol symbol=Clone.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=Clone.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Clone.T
/// @resolution.name source=T target=Clone.T
/// @resolution.name source=K target=Clone.K

type Actual = Clone<{ readonly name: string; age?: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Clone<{ readonly name: string; age?: int32 }>" type={ readonly name: string; age?: int32 }
/// @definition.type symbol=Actual source="type Actual = Clone<{ readonly name: string; age?: int32 }>" value=Clone<{ readonly name: string; age?: int32 }>
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=Actual.name source="readonly name: string" type=string
/// @type.symbol symbol=Actual.age source="age?: int32" type=int32
"#,
    );
}

#[test]
fn test_mapped_type_adds_optional_modifier() {
    let session = TestSession::single(
        r#"
type Loose<T> = { [K in keyof T]?: T[K] };
type Actual = Loose<{ name: string; age: int32 }>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Loose<T> = { [K in keyof T]?: T[K] };
type Actual = Loose<{ name: string; age: int32 }>;

=== dir ===
type Loose<T> = { [K in keyof T]?: T[K] };
/// @generic.template symbol=Loose parameters=(T)
/// @type.symbol symbol=Loose source="type Loose<T> = { [K in keyof T]?: T[K] }" type={ [K in keyof T]?: T[K] }
/// @definition.type symbol=Loose source="type Loose<T> = { [K in keyof T]?: T[K] }" template=(T) value={ [K in keyof T]?: T[K] }
/// @type.symbol symbol=Loose.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=Loose.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Loose.T
/// @resolution.name source=T target=Loose.T
/// @resolution.name source=K target=Loose.K

type Actual = Loose<{ name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Loose<{ name: string; age: int32 }>" type={ name?: string; age?: int32 }
/// @definition.type symbol=Actual source="type Actual = Loose<{ name: string; age: int32 }>" value=Loose<{ name: string; age: int32 }>
/// @resolution.name source=Loose target=Loose
/// @type.symbol symbol=Actual.name source="name: string" type=string
/// @type.symbol symbol=Actual.age source="age: int32" type=int32
"#,
    );
}

#[test]
fn test_mapped_type_optional_read_includes_undefined() {
    let session = TestSession::single(
        r#"
type Optional<T> = { [K in keyof T]?: T[K] };
type Value = Optional<{ name: string }>["name"];

const missing: Value = undefined;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Optional<T> = { [K in keyof T]?: T[K] };
type Value = Optional<{ name: string }>["name"];

const missing: Value = undefined as Value;

=== dir ===
type Optional<T> = { [K in keyof T]?: T[K] };
/// @generic.template symbol=Optional parameters=(T)
/// @type.symbol symbol=Optional source="type Optional<T> = { [K in keyof T]?: T[K] }" type={ [K in keyof T]?: T[K] }
/// @definition.type symbol=Optional source="type Optional<T> = { [K in keyof T]?: T[K] }" template=(T) value={ [K in keyof T]?: T[K] }
/// @type.symbol symbol=Optional.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=Optional.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Optional.T
/// @resolution.name source=T target=Optional.T
/// @resolution.name source=K target=Optional.K

type Value = Optional<{ name: string }>["name"];
/// @type.symbol symbol=Value source="type Value = Optional<{ name: string }>[\"name\"]" type=string | undefined
/// @definition.type symbol=Value source="type Value = Optional<{ name: string }>[\"name\"]" value=Optional<{ name: string }>["name"]
/// @resolution.name source=Optional target=Optional
/// @type.symbol symbol=Value.name source="name: string" type=string

const missing: Value = undefined;
/// @type.symbol symbol=missing source=missing type=Value
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_mapped_type_optional_read_rejects_unrelated_value() {
    let session = TestSession::single(
        r#"
type Optional<T> = { [K in keyof T]?: T[K] };
type Value = Optional<{ name: string }>["name"];

const bad: Value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Optional<T> = { [K in keyof T]?: T[K] };
type Value = Optional<{ name: string }>["name"];

const bad: Value = 1;

=== dir ===
type Optional<T> = { [K in keyof T]?: T[K] };
/// @generic.template symbol=Optional parameters=(T)
/// @type.symbol symbol=Optional source="type Optional<T> = { [K in keyof T]?: T[K] }" type={ [K in keyof T]?: T[K] }
/// @definition.type symbol=Optional source="type Optional<T> = { [K in keyof T]?: T[K] }" template=(T) value={ [K in keyof T]?: T[K] }
/// @type.symbol symbol=Optional.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=Optional.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Optional.T
/// @resolution.name source=T target=Optional.T
/// @resolution.name source=K target=Optional.K

type Value = Optional<{ name: string }>["name"];
/// @type.symbol symbol=Value source="type Value = Optional<{ name: string }>[\"name\"]" type=string | undefined
/// @definition.type symbol=Value source="type Value = Optional<{ name: string }>[\"name\"]" value=Optional<{ name: string }>["name"]
/// @resolution.name source=Optional target=Optional
/// @type.symbol symbol=Value.name source="name: string" type=string

const bad: Value = 1;
/// @type.symbol symbol=bad source=bad type=Value
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '1' is not assignable to type 'Value'"
/// @diagnostic.label line=5 column=20 span="1" line_source="const bad: Value = 1;"
/// @diagnostic.related line=5 column=12 span="Value" line_source="const bad: Value = 1;" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to 'string | undefined'"
"#,
    );
}

#[test]
fn test_mapped_type_removes_source_modifiers() {
    let session = TestSession::single(
        r#"
type Strict<T> = { -readonly [K in keyof T]-?: T[K] };
type Actual = Strict<{ readonly name?: string }>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Strict<T> = { -readonly [K in keyof T]-?: T[K] };
type Actual = Strict<{ readonly name?: string }>;

=== dir ===
type Strict<T> = { -readonly [K in keyof T]-?: T[K] };
/// @generic.template symbol=Strict parameters=(T)
/// @type.symbol symbol=Strict source="type Strict<T> = { -readonly [K in keyof T]-?: T[K] }" type={ -readonly[K in keyof T]-?: T[K] }
/// @definition.type symbol=Strict source="type Strict<T> = { -readonly [K in keyof T]-?: T[K] }" template=(T) value={ -readonly[K in keyof T]-?: T[K] }
/// @type.symbol symbol=Strict.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=Strict.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Strict.T
/// @resolution.name source=T target=Strict.T
/// @resolution.name source=K target=Strict.K

type Actual = Strict<{ readonly name?: string }>;
/// @type.symbol symbol=Actual source="type Actual = Strict<{ readonly name?: string }>" type={ name: string }
/// @definition.type symbol=Actual source="type Actual = Strict<{ readonly name?: string }>" value=Strict<{ readonly name?: string }>
/// @resolution.name source=Strict target=Strict
/// @type.symbol symbol=Actual.name source="readonly name?: string" type=string
"#,
    );
}

#[test]
fn test_mapped_type_remaps_keys() {
    let session = TestSession::single(
        r#"
type Caps = { [K in "name" | "age" as Uppercase<K>]: boolean };
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Caps = { [K in "name" | "age" as Uppercase<K>]: boolean };

=== dir ===
type Caps = { [K in "name" | "age" as Uppercase<K>]: boolean };
/// @type.symbol symbol=Caps source="type Caps = { [K in \"name\" | \"age\" as Uppercase<K>]: boolean }" type={ NAME: boolean; AGE: boolean }
/// @definition.type symbol=Caps source="type Caps = { [K in \"name\" | \"age\" as Uppercase<K>]: boolean }" value={ [K in "name" | "age" as Uppercase<K>]: boolean }
/// @generic.template source=type_expression parameters=(K: "name" | "age")
/// @type.symbol symbol=Caps.K source=[K in "name" | "age" as Uppercase<K>] type=K
/// @resolution.name source=Uppercase target=Uppercase
/// @resolution.name source=K target=Caps.K
"#,
    );
}

#[test]
fn test_mapped_type_merges_remapped_key_collisions() {
    let session = TestSession::single(
        r#"
type Collide<T> = { [K in keyof T as "value"]: T[K] };
type Actual = Collide<{ name: string; age: int32 }>;

declare const actual: Actual;

actual.value satisfies string | int32;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Collide<T> = { [K in keyof T as "value"]: T[K] };
type Actual = Collide<{ name: string; age: int32 }>;

declare const actual: Actual;

actual.value satisfies string | int32;

=== dir ===
type Collide<T> = { [K in keyof T as "value"]: T[K] };
/// @generic.template symbol=Collide parameters=(T)
/// @type.symbol symbol=Collide source="type Collide<T> = { [K in keyof T as \"value\"]: T[K] }" type={ [K in keyof T as "value"]: T[K] }
/// @definition.type symbol=Collide source="type Collide<T> = { [K in keyof T as \"value\"]: T[K] }" template=(T) value={ [K in keyof T as "value"]: T[K] }
/// @type.symbol symbol=Collide.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=Collide.K source=[K in keyof T as "value"] type=K
/// @resolution.name source=T target=Collide.T
/// @resolution.name source=T target=Collide.T
/// @resolution.name source=K target=Collide.K

type Actual = Collide<{ name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Collide<{ name: string; age: int32 }>" type={ value: string; value: int32 }
/// @definition.type symbol=Actual source="type Actual = Collide<{ name: string; age: int32 }>" value=Collide<{ name: string; age: int32 }>
/// @resolution.name source=Collide target=Collide
/// @type.symbol symbol=Actual.name source="name: string" type=string
/// @type.symbol symbol=Actual.age source="age: int32" type=int32

declare const actual: Actual;
/// @type.symbol symbol=actual source=actual type=Actual
/// @resolution.pattern source=actual kind=binding target=actual
/// @resolution.name source=Actual target=Actual

actual.value satisfies string | int32;
/// @resolution.name source=actual target=actual
/// @resolution.member source=actual.value receiver=Actual type=string & int32 kind=field target_receiver=Actual key=value target_type=string & int32
/// @resolution.place source=actual placement="local" lifetime="static" access="immutable"
/// @resolution.access source=actual root=actual
/// @resolution.place source=actual.value placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=actual.value root=actual keys=[value]
"#,
    );
}

#[test]
fn test_mapped_type_drops_never_remapped_keys() {
    let session = TestSession::single(
        r#"
type WithoutSecret<T> = { [K in keyof T as K extends "secret" ? never : K]: T[K] };
type Actual = WithoutSecret<{ name: string; secret: string }>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type WithoutSecret<T> = { [K in keyof T as K extends "secret" ? never : K]: T[K] };
type Actual = WithoutSecret<{ name: string; secret: string }>;

=== dir ===
type WithoutSecret<T> = { [K in keyof T as K extends "secret" ? never : K]: T[K] };
/// @generic.template symbol=WithoutSecret parameters=(T)
/// @type.symbol symbol=WithoutSecret type={ [K in keyof T as K extends "secret" ? never : K]: T[K] }
/// @definition.type symbol=WithoutSecret template=(T) value={ [K in keyof T as K extends "secret" ? never : K]: T[K] }
/// @type.symbol symbol=WithoutSecret.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=WithoutSecret.K source=[K in keyof T as K extends "secret" ? never : K] type=K
/// @resolution.name source=T target=WithoutSecret.T
/// @resolution.name source=K target=WithoutSecret.K
/// @resolution.name source=K target=WithoutSecret.K
/// @resolution.name source=T target=WithoutSecret.T
/// @resolution.name source=K target=WithoutSecret.K

type Actual = WithoutSecret<{ name: string; secret: string }>;
/// @type.symbol symbol=Actual source="type Actual = WithoutSecret<{ name: string; secret: string }>" type={ name: string }
/// @definition.type symbol=Actual source="type Actual = WithoutSecret<{ name: string; secret: string }>" value=WithoutSecret<{ name: string; secret: string }>
/// @resolution.name source=WithoutSecret target=WithoutSecret
/// @type.symbol symbol=Actual.name source="name: string" type=string
/// @type.symbol symbol=Actual.secret source="secret: string" type=string
"#,
    );
}

#[test]
fn test_mapped_type_combines_readonly_and_optional_modifiers() {
    let session = TestSession::single(
        r#"
type Locked<T> = { +readonly [K in keyof T]+?: T[K] };
type Actual = Locked<{ name: string; age: int32 }>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Locked<T> = { +readonly [K in keyof T]+?: T[K] };
type Actual = Locked<{ name: string; age: int32 }>;

=== dir ===
type Locked<T> = { +readonly [K in keyof T]+?: T[K] };
/// @generic.template symbol=Locked parameters=(T)
/// @type.symbol symbol=Locked source="type Locked<T> = { +readonly [K in keyof T]+?: T[K] }" type={ +readonly[K in keyof T]+?: T[K] }
/// @definition.type symbol=Locked source="type Locked<T> = { +readonly [K in keyof T]+?: T[K] }" template=(T) value={ +readonly[K in keyof T]+?: T[K] }
/// @type.symbol symbol=Locked.T source=T type=T
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=Locked.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Locked.T
/// @resolution.name source=T target=Locked.T
/// @resolution.name source=K target=Locked.K

type Actual = Locked<{ name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Locked<{ name: string; age: int32 }>" type={ readonly name?: string; readonly age?: int32 }
/// @definition.type symbol=Actual source="type Actual = Locked<{ name: string; age: int32 }>" value=Locked<{ name: string; age: int32 }>
/// @resolution.name source=Locked target=Locked
/// @type.symbol symbol=Actual.name source="name: string" type=string
/// @type.symbol symbol=Actual.age source="age: int32" type=int32
"#,
    );
}

#[test]
fn test_infer_a_generic_through_an_identity_mapped_parameter() {
    let session = TestSession::single(
        r#"
function first<T>(value: { [K in keyof T]: T[K] }, fallback: T): T {
    return fallback;
}

function build(): int64 {
    const point = first({ x: 1, y: 2 }, { x: 3, y: 4 });
    return point.x;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function first<T>(value: { [K in keyof T]: T[K] }, fallback: T): T {
    return fallback;
}

function build(): int64 {
    const point: { x: int64; y: int64 } = first<{ x: int64; y: int64 }>(
        { x: 1, y: 2 },
        { x: 3, y: 4 },
    );
    return point.x;
}

=== dir ===
function first<T>(value: { [K in keyof T]: T[K] }, fallback: T): T {
/// @generic.template symbol=first parameters=(T)
/// @type.symbol symbol=first type=<T>({ [K in keyof T]: T[K] }, T) => T
/// @type.symbol symbol=first.T source=T type=T
/// @type.symbol symbol=first.value source="value: { [K in keyof T]: T[K] }" type={ [K in keyof T]: T[K] }
/// @generic.template source=type_expression parent=template#0 parameters=(K: keyof T)
/// @type.symbol symbol=first.K source=[K in keyof T] type=K
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T
/// @resolution.name source=K target=first.K
/// @type.symbol symbol=first.fallback source="fallback: T" type=T
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

    return fallback;
    /// @resolution.name source=fallback target=first.fallback
    /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=fallback root=first.fallback

}

function build(): int64 {
/// @type.symbol symbol=build type=() => int64

    const point = first({ x: 1, y: 2 }, { x: 3, y: 4 });
    /// @type.symbol symbol=build.point source=point type={ x: int64; y: int64 }
    /// @resolution.pattern source=point kind=binding target=build.point
    /// @resolution.name source=first target=first
    /// @resolution.call source="first({ x: 1, y: 2 }, { x: 3, y: 4 })" parameters=({ [K in keyof { x: int64; y: int64 }]: { x: int64; y: int64 }[K] }, { x: int64; y: int64 }) arguments=(provided({ x: 1, y: 2 }) as { [K in keyof { x: int64; y: int64 }]: { x: int64; y: int64 }[K] }, provided({ x: 3, y: 4 }) as { x: int64; y: int64 }) return={ x: int64; y: int64 } kind=symbol target=first instance="first<{ x: int64; y: int64 }>"
    /// @generic.instantiation id="first<{ x: int64; y: int64 }>" template=first arguments=({ x: int64; y: int64 })
    /// @generic.instance id="first<{ x: int64; y: int64 }>" template=first arguments=({ x: int64; y: int64 }) dependents=({ x: int64; y: int64 })

    return point.x;
    /// @resolution.name source=point target=build.point
    /// @resolution.member source=point.x receiver={ x: int64; y: int64 } type=int64 kind=field target_receiver={ x: int64; y: int64 } key=x target_type=int64
    /// @resolution.place source=point placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=point root=build.point
    /// @resolution.place source=point.x placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=point.x root=build.point keys=[x]

}
"#,
    );
}
