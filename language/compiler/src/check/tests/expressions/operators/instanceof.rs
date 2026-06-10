use crate::tests::{DirRows, TestSession};

#[test]
fn test_instanceof_reduces_to_boolean() {
    let session = TestSession::single(
        r#"
class User {}
declare const value: unknown;

const ok = value instanceof User;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown

const ok = value instanceof User;
/// @type.symbol symbol=ok source=ok type=boolean
/// @type.node source="value instanceof User" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @type.node source=User type=User
/// @resolution.name source=User target=User
"#,
    );
}
