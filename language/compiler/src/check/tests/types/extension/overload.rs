use crate::tests::{DirRows, TestSession};

#[test]
fn test_unreachable_extension_overload_reports_warning() {
    let session = TestSession::single(
        r#"
struct User {}

extension of User {
    display(): string {
        return "first";
    }

    display(): string {
        return "second";
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}

extension of User {
    display(): string {
        return "first";
    }

    display(): string {
        return "second";
    }
}

=== checked ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=display#1 slot=display type=(this: User) => string
/// @definition.method symbol=display#2 slot=display type=(this: User) => string
/// @resolution.name source=User target=User

    display(): string {
    /// @type.symbol symbol=display#1 type=(this: User) => string

        return "first";
    }

    display(): string {
    /// @type.symbol symbol=display#2 type=(this: User) => string

        return "second";
    }
}
"#,
        r#"

"#,
    );
}
