use crate::tests::{DirRows, TestSession};

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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    constructor(name: string, age: float64): this {}
}

type Args = ConstructorParameters<typeof User>;

const ok: (string, float64) = ("Ada", 42);
ok satisfies (string, number);

=== dir ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor source="constructor(name: string, age: number) {}" slot=constructor role=constructor type=(string, float64) => this

    constructor(name: string, age: number) {}
    /// @type.symbol symbol=User.constructor source="constructor(name: string, age: number) {}" type=(string, float64) => this
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string
    /// @type.symbol symbol=User.constructor.age source="age: number" type=float64

}

type Args = ConstructorParameters<typeof User>;
/// @type.symbol symbol=Args source="type Args = ConstructorParameters<typeof User>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = ConstructorParameters<typeof User>" value=(string, float64)
/// @resolution.name source=ConstructorParameters target=types.function.ConstructorParameters
/// @resolution.name source=User target=User

const ok: Args = ("Ada", 42);
/// @type.symbol symbol=ok source=ok type=(string, float64)
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Args target=Args

ok satisfies (string, number);
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="static" access="readonly"
/// @resolution.access source=ok root=ok
"#,
    );
}

#[test]
fn test_constructor_parameters_extracts_imported_class_arguments() {
    let session = TestSession::builder()
        .module(
            "user.ds",
            r#"
export class User {
    constructor(name: string, age: number) {}
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { User } from "./user.ds";

type Args = ConstructorParameters<typeof User>;

const value: Args = ("Ada", 42);
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { User } from "./user.ds";

type Args = ConstructorParameters<typeof User>;

const value: (string, float64) = ("Ada", 42);

=== dir ===
import { User } from "./user.ds";

type Args = ConstructorParameters<typeof User>;
/// @type.symbol symbol=Args source="type Args = ConstructorParameters<typeof User>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = ConstructorParameters<typeof User>" value=(string, float64)
/// @resolution.name source=ConstructorParameters target=types.function.ConstructorParameters
/// @resolution.name source=User target=user.User

const value: Args = ("Ada", 42);
/// @type.symbol symbol=value source=value type=(string, float64)
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Args target=Args
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

type Value = InstanceType<typeof User>;

const ok: Value = new User();
ok.name satisfies string;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    name: string = "";
}

type Value = InstanceType<typeof User>;

const ok: User = new User();
ok.name satisfies string;

=== dir ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"\"" key=name type=string

    name: string = "";
    /// @type.symbol symbol=User.name source="name: string = \"\"" type=string

}

type Value = InstanceType<typeof User>;
/// @type.symbol symbol=Value source="type Value = InstanceType<typeof User>" type=User
/// @definition.type symbol=Value source="type Value = InstanceType<typeof User>" value=User
/// @resolution.name source=InstanceType target=types.function.InstanceType
/// @resolution.name source=User target=User

const ok: Value = new User();
/// @type.symbol symbol=ok source=ok type=User
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
/// @resolution.name source=User target=User

ok.name satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.member source=ok.name receiver=User type=string kind=field target_receiver=User key=name target=User.name target_type=string
/// @resolution.place source=ok placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=ok root=ok
/// @resolution.place source=ok.name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=ok.name root=ok keys=[name]
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

type Args = ConstructorParameters<typeof User>;

const bad: Args = ("Ada", "old");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    constructor(name: string, age: float64): this {}
}

type Args = ConstructorParameters<typeof User>;

const bad: (string, float64) = ("Ada", "old");

=== dir ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor source="constructor(name: string, age: number) {}" slot=constructor role=constructor type=(string, float64) => this

    constructor(name: string, age: number) {}
    /// @type.symbol symbol=User.constructor source="constructor(name: string, age: number) {}" type=(string, float64) => this
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string
    /// @type.symbol symbol=User.constructor.age source="age: number" type=float64

}

type Args = ConstructorParameters<typeof User>;
/// @type.symbol symbol=Args source="type Args = ConstructorParameters<typeof User>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = ConstructorParameters<typeof User>" value=(string, float64)
/// @resolution.name source=ConstructorParameters target=types.function.ConstructorParameters
/// @resolution.name source=User target=User

const bad: Args = ("Ada", "old");
/// @type.symbol symbol=bad source=bad type=(string, float64)
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
