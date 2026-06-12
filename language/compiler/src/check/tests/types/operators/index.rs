use crate::tests::{DirRows, TestSession};

#[test]
fn test_keyof_and_indexed_access_project_object_shape() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Keys = keyof User;
type Name = User["name"];
type Value = User["name" | "age"];

declare const key: Keys;
declare const name: Name;
declare const value: Value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Keys = keyof User;
type Name = User["name"];
type Value = User["name" | "age"];

declare const key: Keys;
declare const name: Name;
declare const value: Value;

=== checked ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }

type Keys = keyof User;
/// @type.symbol symbol=Keys source="type Keys = keyof User" type="name" | "age"
/// @definition.type symbol=Keys source="type Keys = keyof User" value=keyof { name: string; age: int32 }
/// @resolution.name source=User target=User

type Name = User["name"];
/// @type.symbol symbol=Name source="type Name = User[\"name\"]" type=string
/// @definition.type symbol=Name source="type Name = User[\"name\"]" value={ name: string; age: int32 }["name"]
/// @resolution.name source=User target=User

type Value = User["name" | "age"];
/// @type.symbol symbol=Value source="type Value = User[\"name\" | \"age\"]" type=string | int32
/// @definition.type symbol=Value source="type Value = User[\"name\" | \"age\"]" value={ name: string; age: int32 }["name" | "age"]
/// @resolution.name source=User target=User

declare const key: Keys;
/// @type.symbol symbol=key source=key type=keyof { name: string; age: int32 }
/// @resolution.name source=Keys target=Keys

declare const name: Name;
/// @type.symbol symbol=name source=name type={ name: string; age: int32 }["name"]
/// @resolution.name source=Name target=Name

declare const value: Value;
/// @type.symbol symbol=value source=value type={ name: string; age: int32 }["name" | "age"]
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_indexed_access_rejects_missing_object_key() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Missing = User["missing"];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Missing = User["missing"];

=== checked ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }

type Missing = User["missing"];
/// @type.symbol symbol=Missing source="type Missing = User[\"missing\"]" type={ name: string; age: int32 }["missing"]
/// @definition.type symbol=Missing source="type Missing = User[\"missing\"]" value={ name: string; age: int32 }["missing"]
/// @resolution.name source=User target=User

"#,
        r#"

"#,
    );
}
