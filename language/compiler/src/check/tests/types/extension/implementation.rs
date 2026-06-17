use crate::tests::{DirRows, TestSession};

#[test]
fn test_duplicate_interface_extension_reports_conflict() {
    let session = TestSession::single(
        r#"
newtype interface Show {
    show(): string;
}

struct User {}

extension of User implements Show {
    show(): string {
        return "user";
    }
}

extension of User implements Show {
    show(): string {
        return "debug";
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Show {
    show(): string;
}

struct User {}

extension of User implements Show {
    show(): string {
        return "user";
    }
}

extension of User implements Show {
    show(): string {
        return "debug";
    }
}

=== checked ===
newtype interface Show {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show nominal=true

    show(): string;
    /// @type.symbol symbol=Show.show type=(this: Show) => string
}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension form=interface target=User interfaces=[Show]
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @type.symbol symbol=show type=(this: User) => string

        return "user";
    }
}

extension of User implements Show {
/// @definition.extension form=interface target=User interfaces=[Show]
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @type.symbol symbol=show type=(this: User) => string

        return "debug";
    }
}
"#,
        r#"
/// @diagnostic.error code=EC604 message="conflicting implementations of interface 'Show' for type 'User'"
/// @diagnostic.label line=14 column=1 source="extension of User implements Show {"
"#,
    );
}
