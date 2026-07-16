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
/// @diagnostic.error code=EC613 message="field 'name' is not initialized on every constructor path"
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
        /// @resolution.pattern.assign source=this.name kind=place place=field(User.name) type=string
        /// @type.node source=name type=string
        /// @resolution.name source=name target=User.constructor.name

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

            this.name = name;
            /// @type.node source="this.name = name" type=string
            /// @type.node source=this type=User
            /// @type.node source=this.name type=string
            /// @resolution.receiver source=this kind=this declaration=User type=User
            /// @resolution.pattern.assign source=this.name kind=place place=field(User.name) type=string
            /// @type.node source=name type=string
            /// @resolution.name source=name target=User.constructor.name

        }
    }
}
"#,
        r#"
/// @diagnostic.error code=EC613 message="field 'name' is not initialized on every constructor path"
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
/// @diagnostic.error code=EC613 message="field 'count' is not initialized on every constructor path"
/// @diagnostic.label line=4 column=5 span="count" line_source="count: int32;"
"#,
    );
}
