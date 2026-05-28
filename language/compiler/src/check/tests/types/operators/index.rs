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
type User = { name: string; age: int32 };
/// @type.symbol symbol=User type={ name: string; age: int32 }

type Keys = keyof User;
/// @resolution.name source=User target=User
/// @type.symbol symbol=Keys type="name" | "age"

type Name = User["name"];
/// @resolution.name source=User target=User
/// @type.symbol symbol=Name type=string

type Value = User["name" | "age"];
/// @resolution.name source=User target=User
/// @type.symbol symbol=Value type=string | int32

declare const key: Keys;
/// @resolution.name source=Keys target=Keys
/// @type.symbol symbol=key type="name" | "age"

declare const name: Name;
/// @resolution.name source=Name target=Name
/// @type.symbol symbol=name type=string

declare const value: Value;
/// @resolution.name source=Value target=Value
/// @type.symbol symbol=value type=string | int32
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
type User = { name: string; age: int32 };
/// @type.symbol symbol=User type={ name: string; age: int32 }

type Missing = User["missing"];
/// @resolution.name source=User target=User

"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'missing'"
/// @diagnostic.label line=3 column=21 source="type Missing = User[\"missing\"];"
"#,
    );
}
