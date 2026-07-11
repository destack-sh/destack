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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags<T> = { [K in keyof T]: boolean };
type Actual = Flags<{ name: string; age: int32 }>;

declare const value: Actual;

=== checked ===
type Flags<T> = { [K in keyof T]: boolean };
/// @generic.template symbol=Flags parameters=(T)
/// @type.symbol symbol=Flags source="type Flags<T> = { [K in keyof T]: boolean }" type={ [K in keyof T]: boolean }
/// @definition.type symbol=Flags source="type Flags<T> = { [K in keyof T]: boolean }" template=(T) value={ [K in keyof T]: boolean }
/// @type.symbol symbol=Flags.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Flags.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Flags.T

type Actual = Flags<{ name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Flags<{ name: string; age: int32 }>" type=Flags<{ name: string; age: int32 }> reduced={ name: boolean; age: boolean }
/// @definition.type symbol=Actual source="type Actual = Flags<{ name: string; age: int32 }>" value=Flags<{ name: string; age: int32 }> reduced={ name: boolean; age: boolean }
/// @resolution.name source=Flags target=Flags

declare const value: Actual;
/// @type.symbol symbol=value source=value type=Actual reduced={ name: boolean; age: boolean }
/// @resolution.name source=Actual target=Actual

/// @generic.instance id="Flags<{ name: string; age: int32 }>" template=Flags arguments=({ name: string; age: int32 })
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Clone<T> = { [K in keyof T]: T[K] };
type Actual = Clone<{ readonly name: string; age?: int32 }>;

=== checked ===
type Clone<T> = { [K in keyof T]: T[K] };
/// @generic.template symbol=Clone parameters=(T)
/// @type.symbol symbol=Clone source="type Clone<T> = { [K in keyof T]: T[K] }" type={ [K in keyof T]: T[K] }
/// @definition.type symbol=Clone source="type Clone<T> = { [K in keyof T]: T[K] }" template=(T) value={ [K in keyof T]: T[K] }
/// @type.symbol symbol=Clone.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Clone.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Clone.T
/// @resolution.name source=T target=Clone.T
/// @resolution.name source=K target=Clone.K

type Actual = Clone<{ readonly name: string; age?: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Clone<{ readonly name: string; age?: int32 }>" type=Clone<{ readonly name: string; age?: int32 }> reduced={ readonly name: string; age?: int32 }
/// @definition.type symbol=Actual source="type Actual = Clone<{ readonly name: string; age?: int32 }>" value=Clone<{ readonly name: string; age?: int32 }> reduced={ readonly name: string; age?: int32 }
/// @resolution.name source=Clone target=Clone

/// @generic.instance id="Clone<{ readonly name: string; age?: int32 }>" template=Clone arguments=({ readonly name: string; age?: int32 })
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Loose<T> = { [K in keyof T]?: T[K] };
type Actual = Loose<{ name: string; age: int32 }>;

=== checked ===
type Loose<T> = { [K in keyof T]?: T[K] };
/// @generic.template symbol=Loose parameters=(T)
/// @type.symbol symbol=Loose source="type Loose<T> = { [K in keyof T]?: T[K] }" type={ [K in keyof T]?: T[K] }
/// @definition.type symbol=Loose source="type Loose<T> = { [K in keyof T]?: T[K] }" template=(T) value={ [K in keyof T]?: T[K] }
/// @type.symbol symbol=Loose.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Loose.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Loose.T
/// @resolution.name source=T target=Loose.T
/// @resolution.name source=K target=Loose.K

type Actual = Loose<{ name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Loose<{ name: string; age: int32 }>" type=Loose<{ name: string; age: int32 }> reduced={ name?: string; age?: int32 }
/// @definition.type symbol=Actual source="type Actual = Loose<{ name: string; age: int32 }>" value=Loose<{ name: string; age: int32 }> reduced={ name?: string; age?: int32 }
/// @resolution.name source=Loose target=Loose

/// @generic.instance id="Loose<{ name: string; age: int32 }>" template=Loose arguments=({ name: string; age: int32 })
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Optional<T> = { [K in keyof T]?: T[K] };
type Value = Optional<{ name: string }>["name"];

const missing: Value = undefined as Value;

=== checked ===
type Optional<T> = { [K in keyof T]?: T[K] };
/// @generic.template symbol=Optional parameters=(T)
/// @type.symbol symbol=Optional source="type Optional<T> = { [K in keyof T]?: T[K] }" type={ [K in keyof T]?: T[K] }
/// @definition.type symbol=Optional source="type Optional<T> = { [K in keyof T]?: T[K] }" template=(T) value={ [K in keyof T]?: T[K] }
/// @type.symbol symbol=Optional.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Optional.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Optional.T
/// @resolution.name source=T target=Optional.T
/// @resolution.name source=K target=Optional.K

type Value = Optional<{ name: string }>["name"];
/// @type.symbol symbol=Value source="type Value = Optional<{ name: string }>[\"name\"]" type=Optional<{ name: string }>["name"] reduced=string | undefined
/// @definition.type symbol=Value source="type Value = Optional<{ name: string }>[\"name\"]" value=Optional<{ name: string }>["name"] reduced=string | undefined
/// @resolution.name source=Optional target=Optional

const missing: Value = undefined;
/// @type.symbol symbol=missing source=missing type=Value reduced=string | undefined
/// @resolution.name source=Value target=Value

/// @generic.instance id="Optional<{ name: string }>" template=Optional arguments=({ name: string })
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Optional<T> = { [K in keyof T]?: T[K] };
type Value = Optional<{ name: string }>["name"];

const bad: Value = 1;

=== checked ===
type Optional<T> = { [K in keyof T]?: T[K] };
/// @generic.template symbol=Optional parameters=(T)
/// @type.symbol symbol=Optional source="type Optional<T> = { [K in keyof T]?: T[K] }" type={ [K in keyof T]?: T[K] }
/// @definition.type symbol=Optional source="type Optional<T> = { [K in keyof T]?: T[K] }" template=(T) value={ [K in keyof T]?: T[K] }
/// @type.symbol symbol=Optional.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Optional.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Optional.T
/// @resolution.name source=T target=Optional.T
/// @resolution.name source=K target=Optional.K

type Value = Optional<{ name: string }>["name"];
/// @type.symbol symbol=Value source="type Value = Optional<{ name: string }>[\"name\"]" type=Optional<{ name: string }>["name"] reduced=string | undefined
/// @definition.type symbol=Value source="type Value = Optional<{ name: string }>[\"name\"]" value=Optional<{ name: string }>["name"] reduced=string | undefined
/// @resolution.name source=Optional target=Optional

const bad: Value = 1;
/// @type.symbol symbol=bad source=bad type=Value reduced=string | undefined
/// @resolution.name source=Value target=Value

/// @generic.instance id="Optional<{ name: string }>" template=Optional arguments=({ name: string })
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '1' is not assignable to type 'Value'"
/// @diagnostic.label line=5 column=20 span="1" line_source="const bad: Value = 1;"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Strict<T> = { -readonly [K in keyof T]-?: T[K] };
type Actual = Strict<{ readonly name?: string }>;

=== checked ===
type Strict<T> = { -readonly [K in keyof T]-?: T[K] };
/// @generic.template symbol=Strict parameters=(T)
/// @type.symbol symbol=Strict source="type Strict<T> = { -readonly [K in keyof T]-?: T[K] }" type={ -readonly[K in keyof T]-?: T[K] }
/// @definition.type symbol=Strict source="type Strict<T> = { -readonly [K in keyof T]-?: T[K] }" template=(T) value={ -readonly[K in keyof T]-?: T[K] }
/// @type.symbol symbol=Strict.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Strict.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Strict.T
/// @resolution.name source=T target=Strict.T
/// @resolution.name source=K target=Strict.K

type Actual = Strict<{ readonly name?: string }>;
/// @type.symbol symbol=Actual source="type Actual = Strict<{ readonly name?: string }>" type=Strict<{ readonly name?: string }> reduced={ name: string }
/// @definition.type symbol=Actual source="type Actual = Strict<{ readonly name?: string }>" value=Strict<{ readonly name?: string }> reduced={ name: string }
/// @resolution.name source=Strict target=Strict

/// @generic.instance id="Strict<{ readonly name?: string }>" template=Strict arguments=({ readonly name?: string })
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Caps = { [K in "name" | "age" as Uppercase<K>]: boolean };

=== checked ===
type Caps = { [K in "name" | "age" as Uppercase<K>]: boolean };
/// @type.symbol symbol=Caps source="type Caps = { [K in \"name\" | \"age\" as Uppercase<K>]: boolean }" type={ [K in "name" | "age" as Uppercase<K>]: boolean } reduced={ NAME: boolean; AGE: boolean }
/// @definition.type symbol=Caps source="type Caps = { [K in \"name\" | \"age\" as Uppercase<K>]: boolean }" value={ [K in "name" | "age" as Uppercase<K>]: boolean } reduced={ NAME: boolean; AGE: boolean }
/// @generic.template source=type_mapped_parameter parameters=(K: "name" | "age")
/// @type.symbol symbol=Caps.K source=[K in "name" | "age" as Uppercase<K>] type=K
/// @resolution.name source=Uppercase target=types.string.Uppercase
/// @resolution.name source=K target=Caps.K

/// @generic.instance id=Uppercase<K> template=types.string.Uppercase arguments=(K)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Collide<T> = { [K in keyof T as "value"]: T[K] };
type Actual = Collide<{ name: string; age: int32 }>;

declare const actual: Actual;

actual.value satisfies string | int32;

=== checked ===
type Collide<T> = { [K in keyof T as "value"]: T[K] };
/// @generic.template symbol=Collide parameters=(T)
/// @type.symbol symbol=Collide source="type Collide<T> = { [K in keyof T as \"value\"]: T[K] }" type={ [K in keyof T as "value"]: T[K] }
/// @definition.type symbol=Collide source="type Collide<T> = { [K in keyof T as \"value\"]: T[K] }" template=(T) value={ [K in keyof T as "value"]: T[K] }
/// @type.symbol symbol=Collide.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Collide.K source=[K in keyof T as "value"] type=K
/// @resolution.name source=T target=Collide.T
/// @resolution.name source=T target=Collide.T
/// @resolution.name source=K target=Collide.K

type Actual = Collide<{ name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Collide<{ name: string; age: int32 }>" type=Collide<{ name: string; age: int32 }> reduced={ value: string; value: int32 }
/// @definition.type symbol=Actual source="type Actual = Collide<{ name: string; age: int32 }>" value=Collide<{ name: string; age: int32 }> reduced={ value: string; value: int32 }
/// @resolution.name source=Collide target=Collide

declare const actual: Actual;
/// @type.symbol symbol=actual source=actual type=Actual reduced={ value: string; value: int32 }
/// @resolution.name source=Actual target=Actual

actual.value satisfies string | int32;
/// @resolution.name source=actual target=actual
/// @resolution.member source=actual.value receiver={ value: string; value: int32 } kind=field key=value

/// @generic.instance id="Collide<{ name: string; age: int32 }>" template=Collide arguments=({ name: string; age: int32 })
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type WithoutSecret<T> = { [K in keyof T as K extends "secret" ? never : K]: T[K] };
type Actual = WithoutSecret<{ name: string; secret: string }>;

=== checked ===
type WithoutSecret<T> = { [K in keyof T as K extends "secret" ? never : K]: T[K] };
/// @generic.template symbol=WithoutSecret parameters=(T)
/// @type.symbol symbol=WithoutSecret type={ [K in keyof T as K extends "secret" ? never : K]: T[K] }
/// @definition.type symbol=WithoutSecret template=(T) value={ [K in keyof T as K extends "secret" ? never : K]: T[K] }
/// @type.symbol symbol=WithoutSecret.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=WithoutSecret.K source=[K in keyof T as K extends "secret" ? never : K] type=K
/// @resolution.name source=T target=WithoutSecret.T
/// @resolution.name source=K target=WithoutSecret.K
/// @resolution.name source=K target=WithoutSecret.K
/// @resolution.name source=T target=WithoutSecret.T
/// @resolution.name source=K target=WithoutSecret.K

type Actual = WithoutSecret<{ name: string; secret: string }>;
/// @type.symbol symbol=Actual source="type Actual = WithoutSecret<{ name: string; secret: string }>" type=WithoutSecret<{ name: string; secret: string }> reduced={ name: string }
/// @definition.type symbol=Actual source="type Actual = WithoutSecret<{ name: string; secret: string }>" value=WithoutSecret<{ name: string; secret: string }> reduced={ name: string }
/// @resolution.name source=WithoutSecret target=WithoutSecret

/// @generic.instance id="WithoutSecret<{ name: string; secret: string }>" template=WithoutSecret arguments=({ name: string; secret: string })
"#,
    );
}

#[test]
fn test_mapped_type_preserves_unique_symbol_key() {
    let session = TestSession::single(
        r#"
declare const token: unique symbol;

type Clone<T> = { [K in keyof T]: T[K] };
type Actual = Clone<{ readonly [token]: int32 }>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const token: unique symbol;

type Clone<T> = { [K in keyof T]: T[K] };
type Actual = Clone<{ readonly [token]: int32 }>;

=== checked ===
declare const token: unique symbol;
/// @type.symbol symbol=token source=token type=unique symbol

type Clone<T> = { [K in keyof T]: T[K] };
/// @generic.template symbol=Clone parameters=(T)
/// @type.symbol symbol=Clone source="type Clone<T> = { [K in keyof T]: T[K] }" type={ [K in keyof T]: T[K] }
/// @definition.type symbol=Clone source="type Clone<T> = { [K in keyof T]: T[K] }" template=(T) value={ [K in keyof T]: T[K] }
/// @type.symbol symbol=Clone.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Clone.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Clone.T
/// @resolution.name source=T target=Clone.T
/// @resolution.name source=K target=Clone.K

type Actual = Clone<{ readonly [token]: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Clone<{ readonly [token]: int32 }>" type=Clone<{ readonly [token]: int32 }> reduced={ readonly [token]: int32 }
/// @definition.type symbol=Actual source="type Actual = Clone<{ readonly [token]: int32 }>" value=Clone<{ readonly [token]: int32 }> reduced={ readonly [token]: int32 }
/// @resolution.name source=Clone target=Clone

/// @generic.instance id="Clone<{ readonly [token]: int32 }>" template=Clone arguments=({ readonly [token]: int32 })
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Locked<T> = { +readonly [K in keyof T]+?: T[K] };
type Actual = Locked<{ name: string; age: int32 }>;

=== checked ===
type Locked<T> = { +readonly [K in keyof T]+?: T[K] };
/// @generic.template symbol=Locked parameters=(T)
/// @type.symbol symbol=Locked source="type Locked<T> = { +readonly [K in keyof T]+?: T[K] }" type={ +readonly[K in keyof T]+?: T[K] }
/// @definition.type symbol=Locked source="type Locked<T> = { +readonly [K in keyof T]+?: T[K] }" template=(T) value={ +readonly[K in keyof T]+?: T[K] }
/// @type.symbol symbol=Locked.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=(K: keyof T)
/// @type.symbol symbol=Locked.K source=[K in keyof T] type=K
/// @resolution.name source=T target=Locked.T
/// @resolution.name source=T target=Locked.T
/// @resolution.name source=K target=Locked.K

type Actual = Locked<{ name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Locked<{ name: string; age: int32 }>" type=Locked<{ name: string; age: int32 }> reduced={ readonly name?: string; readonly age?: int32 }
/// @definition.type symbol=Actual source="type Actual = Locked<{ name: string; age: int32 }>" value=Locked<{ name: string; age: int32 }> reduced={ readonly name?: string; readonly age?: int32 }
/// @resolution.name source=Locked target=Locked

/// @generic.instance id="Locked<{ name: string; age: int32 }>" template=Locked arguments=({ name: string; age: int32 })
"#,
    );
}
