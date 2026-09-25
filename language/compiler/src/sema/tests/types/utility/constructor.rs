use crate::tests::{DirRows, TestSession};

/// ConstructorParameters extracts the argument tuple of a class constructor.
#[test]
fn test_constructor_parameters_extracts_constructor_arguments() {
    let session = TestSession::single(
        r#"
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const ok: Args = ("Ada", 42);
ok satisfies (string, number);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    constructor(name: string, age: float64) {}
}

type Args = ConstructorParameters<typeof User>;

const ok: Args = ("Ada", 42);
ok satisfies (string, number);

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor source="constructor(name: string, age: number) {}" slot=constructor role=constructor type=(this: &'managed User, string, float64) => User

    constructor(name: string, age: number) {}
    /// @type.symbol symbol=User.constructor source="constructor(name: string, age: number) {}" type=(this: &'managed User, string, float64) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string
    /// @type.symbol symbol=User.constructor.age source="age: number" type=float64

}

type Args = ConstructorParameters<typeof User>;
/// @type.symbol symbol=Args source="type Args = ConstructorParameters<typeof User>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = ConstructorParameters<typeof User>" value=ConstructorParameters<typeof User>
/// @resolution.name source=ConstructorParameters target=ConstructorParameters
/// @resolution.name source=User target=User

const ok: Args = ("Ada", 42);
/// @type.symbol symbol=ok source=ok type=Args
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Args target=Args

ok satisfies (string, number);
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ok root=ok
"#,
    );
}

/// ConstructorParameters extracts the argument tuple of an imported class.
#[test]
fn test_constructor_parameters_extracts_imported_class_arguments() {
    let session = TestSession::builder()
        .module(
            "user.tspp",
            r#"
export class User {
    constructor(name: string, age: number) {}
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { User } from "./user.tspp";

type Args = ConstructorParameters<typeof User>;

const value: Args = ("Ada", 42);
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { User } from "./user.tspp";

type Args = ConstructorParameters<typeof User>;

const value: Args = ("Ada", 42);

=== dir ===
import { User } from "./user.tspp";

type Args = ConstructorParameters<typeof User>;
/// @type.symbol symbol=Args source="type Args = ConstructorParameters<typeof User>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = ConstructorParameters<typeof User>" value=ConstructorParameters<typeof user.User>
/// @resolution.name source=ConstructorParameters target=ConstructorParameters
/// @resolution.name source=User target=user.User

const value: Args = ("Ada", 42);
/// @type.symbol symbol=value source=value type=Args
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Args target=Args
"#,
    );
}

/// InstanceType extracts the instance type of a class.
#[test]
fn test_instance_type_extracts_class_instance() {
    let session = TestSession::single(
        r#"
class User {
    name: string = "";
}

type Value = InstanceType<typeof User>;

const ok: Value = new User();
ok.name satisfies string;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    name: string = "";
}

type Value = InstanceType<typeof User>;

const ok: Value = new User();
ok.name satisfies string;

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"\"" key=name type=string

    name: string = "";
    /// @type.symbol symbol=User.name source="name: string = \"\"" type=string

}

type Value = InstanceType<typeof User>;
/// @type.symbol symbol=Value source="type Value = InstanceType<typeof User>" type=User
/// @definition.type symbol=Value source="type Value = InstanceType<typeof User>" value=InstanceType<typeof User>
/// @resolution.name source=InstanceType target=InstanceType
/// @resolution.name source=User target=User

const ok: Value = new User();
/// @type.symbol symbol=ok source=ok type=Value
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
/// @resolution.name source=User target=User

ok.name satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.member source=ok.name receiver=Value type=string kind=field target_receiver=Value key=name target=User.name target_type=string
/// @resolution.place source=ok placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ok root=ok
/// @resolution.place source=ok.name placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=ok.name root=ok keys=[name]
"#,
    );
}

/// An argument of another type reports a diagnostic against ConstructorParameters.
#[test]
fn test_constructor_parameters_rejects_wrong_argument_types() {
    let session = TestSession::single(
        r#"
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const bad: Args = ("Ada", "old");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    constructor(name: string, age: float64) {}
}

type Args = ConstructorParameters<typeof User>;

const bad: Args = ("Ada", "old");

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor source="constructor(name: string, age: number) {}" slot=constructor role=constructor type=(this: &'managed User, string, float64) => User

    constructor(name: string, age: number) {}
    /// @type.symbol symbol=User.constructor source="constructor(name: string, age: number) {}" type=(this: &'managed User, string, float64) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string
    /// @type.symbol symbol=User.constructor.age source="age: number" type=float64

}

type Args = ConstructorParameters<typeof User>;
/// @type.symbol symbol=Args source="type Args = ConstructorParameters<typeof User>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = ConstructorParameters<typeof User>" value=ConstructorParameters<typeof User>
/// @resolution.name source=ConstructorParameters target=ConstructorParameters
/// @resolution.name source=User target=User

const bad: Args = ("Ada", "old");
/// @type.symbol symbol=bad source=bad type=Args
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Args target=Args
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"old\"' is not assignable to type 'float64'"
/// @diagnostic.label line=8 column=27 span="\"old\"" line_source="const bad: Args = (\"Ada\", \"old\");"
/// @diagnostic.related line=8 column=12 span="Args" line_source="const bad: Args = (\"Ada\", \"old\");" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}
