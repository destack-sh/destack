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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string = "Ada";
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
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
fn test_infer_class_field_from_initializer() {
    let session = TestSession::single(
        r#"
class User {
    active = true;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    active: boolean = true;
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;

    constructor(name: string): this {
        this.name = name;
    }
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(string) => this

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=User.constructor type=(string) => this
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

        this.name = name;
        /// @type.node source="this.name = name" type=string
        /// @type.node source=this type=User
        /// @type.node source=this.name type=string
        /// @resolution.receiver source=this kind=this declaration=User type=User
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=User, target=field(receiver=User, target=User.name, type=string), type=string" type=string
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
fn test_super_assignment_satisfies_definite_initialization() {
    let session = TestSession::single(
        r#"
class Base {
    name: string = "";
}

class User extends Base {
    override name: string;

    constructor() {
        super();
        super.name = "Ada";
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Base {
    name: string = "";
}

class User extends Base {
    override name: string;

    constructor(): this {
        super();
        super.name = "Ada";
    }
}

=== checked ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.field symbol=Base.name source="name: string = \"\"" key=name type=string

    name: string = "";
    /// @type.symbol symbol=Base.name source="name: string = \"\"" type=string
    /// @type.node source="\"\"" type=""

}

class User extends Base {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.extends symbol=User source=Base target=Base
/// @definition.field symbol=User.name source="override name: string" key=name override=true type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=() => this
/// @resolution.name source=Base target=Base

    override name: string;
    /// @type.symbol symbol=User.name source="override name: string" type=string

    constructor() {
    /// @type.symbol symbol=User.constructor type=() => this

        super();
        /// @type.node source=super type=Base
        /// @type.node source=super() type=void
        /// @resolution.receiver source=super kind=super declaration=User type=Base
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.construct source=super() parameters=() return=void kind=class target=Base constructor=default

        super.name = "Ada";
        /// @type.node source="super.name = \"Ada\"" type="Ada"
        /// @type.node source=super type=Base
        /// @type.node source=super.name type=string
        /// @resolution.receiver source=super kind=super declaration=User type=Base
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.pattern.assign source=super.name kind=place
        /// @resolution.access source=super.name root=this keys=[name]
        /// @resolution.assignment source=super.name write="receiver=Base, target=field(receiver=Base, target=Base.name, type=string), type=string" type=string
        /// @type.node source="\"Ada\"" type="Ada"

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;

    constructor(enabled: boolean, name: string): this {
        if (enabled) {
            this.name = name;
        }
    }
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(boolean, string) => this

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    constructor(enabled: boolean, name: string) {
    /// @type.symbol symbol=User.constructor type=(boolean, string) => this
    /// @type.symbol symbol=User.constructor.enabled source="enabled: boolean" type=boolean
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

        if (enabled) {
        /// @type.node source=enabled type=boolean
        /// @resolution.name source=enabled target=User.constructor.enabled
        /// @resolution.place source=enabled placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=enabled root=User.constructor.enabled

            this.name = name;
            /// @type.node source="this.name = name" type=string
            /// @type.node source=this type=User
            /// @type.node source=this.name type=string
            /// @resolution.receiver source=this kind=this declaration=User type=User
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.pattern.assign source=this.name kind=place
            /// @resolution.access source=this.name root=this keys=[name]
            /// @resolution.assignment source=this.name write="receiver=User, target=field(receiver=User, target=User.name, type=string), type=string" type=string
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

#[test]
fn test_definite_assertion_waives_constructor_initialization() {
    let session = TestSession::single(
        r#"
class Connection {
    handle!: int32;
    count: int32;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Connection {
    handle!: int32;
    count: int32;
}

=== checked ===
class Connection {
/// @type.symbol symbol=Connection type=Connection
/// @definition.class symbol=Connection
/// @definition.field symbol=Connection.count source="count: int32" key=count type=int32
/// @definition.field symbol=Connection.handle source="handle!: int32" key=handle type=int32

    handle!: int32;
    /// @type.symbol symbol=Connection.handle source="handle!: int32" type=int32

    count: int32;
    /// @type.symbol symbol=Connection.count source="count: int32" type=int32

}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'count' is not initialized on every constructor path"
/// @diagnostic.label line=4 column=5 span="count" line_source="count: int32;"
"#,
    );
}

#[test]
fn test_unannotated_field_reports_and_binds_the_error_type() {
    // assert a bare field reports its missing annotation and settles
    //  as the error type, leaving the checked write intact
    let session = TestSession::single(
        r#"
class Foo {
    like
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Foo {
    like;
}

=== checked ===
class Foo {
/// @type.symbol symbol=Foo type=Foo
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
