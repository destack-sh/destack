use crate::tests::{DirRows, TestSession};

#[test]
fn test_mapped_type_projects_each_source_key() {
    let session = TestSession::single(
        r#"
type Flags<T> = { [K in keyof T]: boolean };
type Actual = Flags<type { name: string; age: int32 }>;

declare const value: Actual;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags<T> = { [K in keyof T]: boolean };
type Actual = Flags<type { name: string; age: int32 }>;

declare const value: Actual;

=== checked ===
type Flags<T> = { [K in keyof T]: boolean };
/// @generic.template symbol=Flags parameters=[T]
/// @type.symbol symbol=Flags source="type Flags<T> = { [K in keyof T]: boolean }" type={ [K in keyof T]: boolean }
/// @definition.type symbol=Flags source="type Flags<T> = { [K in keyof T]: boolean }" template=LocalGenericTemplateId(0) value={ [K in keyof T]: boolean }
/// @type.symbol symbol=Flags.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=[K: keyof T]
/// @type.symbol symbol=K source=[K in keyof T] type=K
/// @resolution.name source=T target=Flags.T

type Actual = Flags<type { name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Flags<type { name: string; age: int32 }>" type={ name: boolean; age: boolean }
/// @definition.type symbol=Actual source="type Actual = Flags<type { name: string; age: int32 }>" value={ name: boolean; age: boolean }
/// @resolution.name source=Flags target=Flags

declare const value: Actual;
/// @type.symbol symbol=value source=value type=Flags<{ name: string; age: int32 }>
/// @resolution.name source=Actual target=Actual
"#,
    );
}

#[test]
fn test_mapped_type_carries_source_field_modifiers() {
    let session = TestSession::single(
        r#"
type Clone<T> = { [K in keyof T]: T[K] };
type Actual = Clone<type { readonly name: string; age?: int32 }>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Clone<T> = { [K in keyof T]: T[K] };
type Actual = Clone<type { readonly name: string; age?: int32 }>;

=== checked ===
type Clone<T> = { [K in keyof T]: T[K] };
/// @generic.template symbol=Clone parameters=[T]
/// @type.symbol symbol=Clone source="type Clone<T> = { [K in keyof T]: T[K] }" type={ [K in keyof T]: T[K] }
/// @definition.type symbol=Clone source="type Clone<T> = { [K in keyof T]: T[K] }" template=LocalGenericTemplateId(0) value={ [K in keyof T]: T[K] }
/// @type.symbol symbol=Clone.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=[K: keyof T]
/// @type.symbol symbol=K source=[K in keyof T] type=K
/// @resolution.name source=T target=Clone.T
/// @resolution.name source=T target=Clone.T
/// @resolution.name source=K target=K

type Actual = Clone<type { readonly name: string; age?: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Clone<type { readonly name: string; age?: int32 }>" type={ readonly name: string; age?: int32 }
/// @definition.type symbol=Actual source="type Actual = Clone<type { readonly name: string; age?: int32 }>" value={ readonly name: string; age?: int32 }
/// @resolution.name source=Clone target=Clone
"#,
    );
}

#[test]
fn test_mapped_type_adds_optional_modifier() {
    let session = TestSession::single(
        r#"
type Loose<T> = { [K in keyof T]?: T[K] };
type Actual = Loose<type { name: string; age: int32 }>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Loose<T> = { [K in keyof T]?: T[K] };
type Actual = Loose<type { name: string; age: int32 }>;

=== checked ===
type Loose<T> = { [K in keyof T]?: T[K] };
/// @generic.template symbol=Loose parameters=[T]
/// @type.symbol symbol=Loose source="type Loose<T> = { [K in keyof T]?: T[K] }" type={ [K in keyof T]?: T[K] }
/// @definition.type symbol=Loose source="type Loose<T> = { [K in keyof T]?: T[K] }" template=LocalGenericTemplateId(0) value={ [K in keyof T]?: T[K] }
/// @type.symbol symbol=Loose.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=[K: keyof T]
/// @type.symbol symbol=K source=[K in keyof T] type=K
/// @resolution.name source=T target=Loose.T
/// @resolution.name source=T target=Loose.T
/// @resolution.name source=K target=K

type Actual = Loose<type { name: string; age: int32 }>;
/// @type.symbol symbol=Actual source="type Actual = Loose<type { name: string; age: int32 }>" type={ name?: string; age?: int32 }
/// @definition.type symbol=Actual source="type Actual = Loose<type { name: string; age: int32 }>" value={ name?: string; age?: int32 }
/// @resolution.name source=Loose target=Loose
"#,
    );
}

#[test]
fn test_mapped_type_removes_source_modifiers() {
    let session = TestSession::single(
        r#"
type Strict<T> = { -readonly [K in keyof T]-?: T[K] };
type Actual = Strict<type { readonly name?: string }>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Strict<T> = { -readonly [K in keyof T]-?: T[K] };
type Actual = Strict<type { readonly name?: string }>;

=== checked ===
type Strict<T> = { -readonly [K in keyof T]-?: T[K] };
/// @generic.template symbol=Strict parameters=[T]
/// @type.symbol symbol=Strict source="type Strict<T> = { -readonly [K in keyof T]-?: T[K] }" type={ -readonly[K in keyof T]-?: T[K] }
/// @definition.type symbol=Strict source="type Strict<T> = { -readonly [K in keyof T]-?: T[K] }" template=LocalGenericTemplateId(0) value={ -readonly[K in keyof T]-?: T[K] }
/// @type.symbol symbol=Strict.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=[K: keyof T]
/// @type.symbol symbol=K source=[K in keyof T] type=K
/// @resolution.name source=T target=Strict.T
/// @resolution.name source=T target=Strict.T
/// @resolution.name source=K target=K

type Actual = Strict<type { readonly name?: string }>;
/// @type.symbol symbol=Actual source="type Actual = Strict<type { readonly name?: string }>" type={ name: string }
/// @definition.type symbol=Actual source="type Actual = Strict<type { readonly name?: string }>" value={ name: string }
/// @resolution.name source=Strict target=Strict
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
/// @type.symbol symbol=Caps source="type Caps = { [K in \"name\" | \"age\" as Uppercase<K>]: boolean }" type={ NAME: boolean; AGE: boolean }
/// @definition.type symbol=Caps source="type Caps = { [K in \"name\" | \"age\" as Uppercase<K>]: boolean }" value={ NAME: boolean; AGE: boolean }
/// @generic.template source=type_mapped_parameter parameters=[K: "name" | "age"]
/// @type.symbol symbol=K source=[K in "name" | "age" as Uppercase<K>] type=K
/// @resolution.name source=Uppercase target=types.string.Uppercase
/// @resolution.name source=K target=K
"#,
    );
}

#[test]
fn test_mapped_type_drops_never_remapped_keys() {
    let session = TestSession::single(
        r#"
type WithoutSecret<T> = { [K in keyof T as K extends "secret" ? never : K]: T[K] };
type Actual = WithoutSecret<type { name: string; secret: string }>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type WithoutSecret<T> = { [K in keyof T as K extends "secret" ? never : K]: T[K] };
type Actual = WithoutSecret<type { name: string; secret: string }>;

=== checked ===
type WithoutSecret<T> = { [K in keyof T as K extends "secret" ? never : K]: T[K] };
/// @generic.template symbol=WithoutSecret parameters=[T]
/// @type.symbol symbol=WithoutSecret type={ [K in keyof T as K extends "secret" ? never : K]: T[K] }
/// @definition.type symbol=WithoutSecret template=LocalGenericTemplateId(0) value={ [K in keyof T as K extends "secret" ? never : K]: T[K] }
/// @type.symbol symbol=WithoutSecret.T source=T type=T
/// @generic.template source=type_mapped_parameter parameters=[K: keyof T]
/// @type.symbol symbol=K source=[K in keyof T as K extends "secret" ? never : K] type=K
/// @resolution.name source=T target=WithoutSecret.T
/// @resolution.name source=K target=K
/// @resolution.name source=K target=K
/// @resolution.name source=T target=WithoutSecret.T
/// @resolution.name source=K target=K

type Actual = WithoutSecret<type { name: string; secret: string }>;
/// @type.symbol symbol=Actual source="type Actual = WithoutSecret<type { name: string; secret: string }>" type={ name: string }
/// @definition.type symbol=Actual source="type Actual = WithoutSecret<type { name: string; secret: string }>" value={ name: string }
/// @resolution.name source=WithoutSecret target=WithoutSecret
"#,
    );
}
