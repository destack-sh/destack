use crate::tests::{DirRows, TestSession};

#[test]
fn test_constructor_parameters_extracts_constructor_arguments() {
    let session = TestSession::single(
        r#"
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<User>;

const ok: Args = ("Ada", 42);
ok satisfies (string, number);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<User>;

const ok: Args = ("Ada", 42);
ok satisfies (string, number);

=== checked ===
class User {
/// @type.symbol symbol=User type=User

    constructor(name: string, age: number) {}
    /// @type.symbol symbol=User.constructor type=(string, number) => User
}

type Args = ConstructorParameters<User>;
/// @type.symbol symbol=Args source="type Args = ConstructorParameters<User>" type=(string, number)
/// @definition.type symbol=Args source="type Args = ConstructorParameters<User>" value=(string, number)
/// @resolution.name source=ConstructorParameters target=types.function.ConstructorParameters
/// @resolution.name source=User target=User

const ok: Args = ("Ada", 42);
/// @type.symbol symbol=ok source=ok type=(string, number)
/// @resolution.name source=Args target=Args

ok satisfies (string, number);
/// @resolution.name source=ok target=ok
"#,
    );
}

#[test]
fn test_instance_type_extracts_class_instance() {
    let session = TestSession::single(
        r#"
class User {
    name: string = "";
}

type Value = InstanceType<User>;

const ok: Value = new User();
ok.name satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    name: string = "";
}

type Value = InstanceType<User>;

const ok: Value = new User();
ok.name satisfies string;

=== checked ===
class User {
/// @type.symbol symbol=User type=User

    name: string = "";
    /// @type.symbol symbol=User.name source=name type=string
}

type Value = InstanceType<User>;
/// @type.symbol symbol=Value source="type Value = InstanceType<User>" type=User
/// @definition.type symbol=Value source="type Value = InstanceType<User>" value=User
/// @resolution.name source=InstanceType target=types.function.InstanceType
/// @resolution.name source=User target=User

const ok: Value = new User();
/// @type.symbol symbol=ok source=ok type=User
/// @resolution.name source=Value target=Value
/// @resolution.name source=User target=User
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User

ok.name satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.member source=ok.name receiver=Value kind=field key=name
"#,
    );
}

#[test]
fn test_constructor_parameters_rejects_wrong_argument_types() {
    let session = TestSession::single(
        r#"
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<User>;

const bad: Args = ("Ada", "old");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<User>;

const bad: Args = ("Ada", "old");

=== checked ===
class User {
/// @type.symbol symbol=User type=User

    constructor(name: string, age: number) {}
    /// @type.symbol symbol=User.constructor type=(string, number) => User
}

type Args = ConstructorParameters<User>;
/// @type.symbol symbol=Args source="type Args = ConstructorParameters<User>" type=(string, number)
/// @definition.type symbol=Args source="type Args = ConstructorParameters<User>" value=(string, number)
/// @resolution.name source=ConstructorParameters target=types.function.ConstructorParameters
/// @resolution.name source=User target=User

const bad: Args = ("Ada", "old");
/// @type.symbol symbol=bad source=bad type=(string, number)
/// @resolution.name source=Args target=Args
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '(\"Ada\", \"old\")' is not assignable to type 'Args'"
/// @diagnostic.label line=8 column=7 source="const bad: Args = (\"Ada\", \"old\");"
"#,
    );
}
