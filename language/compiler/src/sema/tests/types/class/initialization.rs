use crate::tests::{DirRows, TestSession};

#[test]
fn test_class_field_initializer_satisfies_definite_initialization() {
    let session = TestSession::single(
        r#"
class User {
    name: string = "Ada";
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string = "Ada";
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"Ada\"" key=name type=string

    name: string = "Ada";
    /// @type.symbol symbol=User.name source="name: string = \"Ada\"" type=string
    /// @type.node source="\"Ada\"" type="Ada"

}
"#,
    );
}

#[test]
fn test_static_field_initializers_satisfy_initialization() {
    let session = TestSession::single(
        r#"
class ClassState {
    static value: int32 = 1;
}

struct StructState {
    static value: int32 = 1;
}

enum EnumState {
    Ready,

    static value: int32 = 1;
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
class ClassState {
    static value: int32 = 1;
}

struct StructState {
    static value: int32 = 1;
}

enum EnumState {
    Ready,

    static value: int32 = 1;
}

=== dir ===
class ClassState {
/// @type.symbol symbol=ClassState type=typeof ClassState
/// @definition.class symbol=ClassState
/// @definition.field symbol=ClassState.value source="static value: int32 = 1" key=value static=true type=int32

    static value: int32 = 1;
    /// @type.symbol symbol=ClassState.value source="static value: int32 = 1" type=int32

}

struct StructState {
/// @type.symbol symbol=StructState type=StructState
/// @definition.struct symbol=StructState
/// @definition.field symbol=StructState.value source="static value: int32 = 1" key=value static=true type=int32

    static value: int32 = 1;
    /// @type.symbol symbol=StructState.value source="static value: int32 = 1" type=int32

}

enum EnumState {
/// @type.symbol symbol=EnumState type=EnumState
/// @definition.enum symbol=EnumState
/// @definition.variant symbol=EnumState.Ready source=Ready key=Ready value=0
/// @definition.field symbol=EnumState.value source="static value: int32 = 1" key=value static=true type=int32

    Ready,
    /// @type.symbol symbol=EnumState.Ready source=Ready type=EnumState.Ready

    static value: int32 = 1;
    /// @type.symbol symbol=EnumState.value source="static value: int32 = 1" type=int32

}
"#);
}

#[test]
fn test_static_fields_require_initializers() {
    let session = TestSession::single(
        r#"
class ClassState {
    static value: int32;
}

struct StructState {
    static value: int32;
}

enum EnumState {
    Ready,

    static value: int32;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class ClassState {
    static value: int32;
}

struct StructState {
    static value: int32;
}

enum EnumState {
    Ready,

    static value: int32;
}

=== dir ===
class ClassState {
/// @type.symbol symbol=ClassState type=typeof ClassState
/// @definition.class symbol=ClassState
/// @definition.field symbol=ClassState.value source="static value: int32" key=value static=true type=int32

    static value: int32;
    /// @type.symbol symbol=ClassState.value source="static value: int32" type=int32

}

struct StructState {
/// @type.symbol symbol=StructState type=StructState
/// @definition.struct symbol=StructState
/// @definition.field symbol=StructState.value source="static value: int32" key=value static=true type=int32

    static value: int32;
    /// @type.symbol symbol=StructState.value source="static value: int32" type=int32

}

enum EnumState {
/// @type.symbol symbol=EnumState type=EnumState
/// @definition.enum symbol=EnumState
/// @definition.variant symbol=EnumState.Ready source=Ready key=Ready value=0
/// @definition.field symbol=EnumState.value source="static value: int32" key=value static=true type=int32

    Ready,
    /// @type.symbol symbol=EnumState.Ready source=Ready type=EnumState.Ready

    static value: int32;
    /// @type.symbol symbol=EnumState.value source="static value: int32" type=int32

}
"#,
        r#"
/// @diagnostic.error id=static-field-missing-initializer message="static field 'value' requires an initializer"
/// @diagnostic.label line=3 column=12 span="value" line_source="static value: int32;"
/// @diagnostic.error id=static-field-missing-initializer message="static field 'value' requires an initializer"
/// @diagnostic.label line=7 column=12 span="value" line_source="static value: int32;"
/// @diagnostic.error id=static-field-missing-initializer message="static field 'value' requires an initializer"
/// @diagnostic.label line=13 column=12 span="value" line_source="static value: int32;"
"#,
    );
}

#[test]
fn test_static_fields_admitting_undefined_need_no_initializer() {
    let session = TestSession::single(
        r#"
class State {
    static current: string | undefined;
    static optional?: string;
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
class State {
    static current: string | undefined;
    static optional?: string;
}

=== dir ===
class State {
/// @type.symbol symbol=State type=typeof State
/// @definition.class symbol=State
/// @definition.field symbol=State.current source="static current: string | undefined" key=current static=true type=string | undefined
/// @definition.field symbol=State.optional source="static optional?: string" key=optional static=true type=string

    static current: string | undefined;
    /// @type.symbol symbol=State.current source="static current: string | undefined" type=string | undefined

    static optional?: string;
    /// @type.symbol symbol=State.optional source="static optional?: string" type=string

}
"#);
}

#[test]
fn test_infer_class_field_from_initializer() {
    let session = TestSession::single(
        r#"
class User {
    active = true;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    active: boolean = true;
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.active source="active = true" key=active type=boolean

    active = true;
    /// @type.symbol symbol=User.active source="active = true" type=boolean
    /// @type.node source=true type=true

}
"#,
    );
}

#[test]
fn test_implicit_constructor_requires_initialized_fields() {
    let session = TestSession::single(
        r#"
class User {
    name: string;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'name' is not initialized on every constructor path"
/// @diagnostic.label line=3 column=5 span="name" line_source="name: string;"
"#,
    );
}

#[test]
fn test_constructor_assignment_satisfies_definite_initialization() {
    let session = TestSession::single(
        r#"
class User {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(this: &'managed User, string) => User

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=User.constructor type=(this: &'managed User, string) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

        this.name = name;
        /// @type.node source="this.name = name" type=string
        /// @type.node source=this type=&'managed User
        /// @type.node source=this.name type=string
        /// @resolution.receiver source=this kind=this declaration=User type=&'managed User
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed User, target=field(receiver=&'managed User, target=User.name, type=string), type=string" type=string
        /// @type.node source=name type=string
        /// @resolution.name source=name target=User.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=User.constructor.name

    }
}
"#,
    );
}

#[test]
fn test_missing_constructor_path_reports_uninitialized_field() {
    let session = TestSession::single(
        r#"
class User {
    name: string;

    constructor(enabled: boolean, name: string) {
        if (enabled) {
            this.name = name;
        }
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;

    constructor(enabled: boolean, name: string) {
        if (enabled) {
            this.name = name;
        }
    }
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(this: &'managed User, boolean, string) => User

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    constructor(enabled: boolean, name: string) {
    /// @type.symbol symbol=User.constructor type=(this: &'managed User, boolean, string) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User
    /// @type.symbol symbol=User.constructor.enabled source="enabled: boolean" type=boolean
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

        if (enabled) {
        /// @type.node source=enabled type=boolean
        /// @resolution.name source=enabled target=User.constructor.enabled
        /// @resolution.place source=enabled placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=enabled root=User.constructor.enabled

            this.name = name;
            /// @type.node source="this.name = name" type=string
            /// @type.node source=this type=&'managed User
            /// @type.node source=this.name type=string
            /// @resolution.receiver source=this kind=this declaration=User type=&'managed User
            /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
            /// @resolution.access source=this root=this
            /// @resolution.pattern.assign source=this.name kind=place
            /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
            /// @resolution.access source=this.name root=this keys=[name]
            /// @resolution.assignment source=this.name write="receiver=&'managed User, target=field(receiver=&'managed User, target=User.name, type=string), type=string" type=string
            /// @type.node source=name type=string
            /// @resolution.name source=name target=User.constructor.name
            /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=name root=User.constructor.name

        }
    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'name' is not initialized on every constructor path"
/// @diagnostic.label line=3 column=5 span="name" line_source="name: string;"
"#,
    );
}

/// Report an unannotated class field.
#[test]
fn test_report_unannotated_class_field() {
    let session = TestSession::single(
        r#"
class Foo {
    like
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Foo {
    like;
}

=== dir ===
class Foo {
/// @type.symbol symbol=Foo type=typeof Foo
/// @definition.class symbol=Foo
/// @definition.field symbol=Foo.like source=like key=like type=<error>

    like
    /// @type.symbol symbol=Foo.like source=like type=<error>

}
"#,
        r#"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=3 column=5 span="like" line_source="like"
"#,
    );
}
