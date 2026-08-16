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

    session.assert_dir_and_diagnostics(
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

=== dir ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=display#1 slot=display type=<display#1.'a>(this: &display#1.'a readonly this) => string
/// @definition.method symbol=display#2 slot=display type=<display#2.'a>(this: &display#2.'a readonly this) => string
/// @resolution.name source=User target=User

    display(): string {
    /// @generic.template symbol=display#1 parameters=('a)
    /// @type.symbol symbol=display#1 type=<display#1.'a>(this: &display#1.'a readonly this) => string

        return "first";
    }

    display(): string {
    /// @generic.template symbol=display#2 parameters=('a)
    /// @type.symbol symbol=display#2 type=<display#2.'a>(this: &display#2.'a readonly this) => string

        return "second";
    }
}
"#,
        r#"
"#,
    );
}
