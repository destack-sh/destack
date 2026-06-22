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
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=inherent target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show#1 slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @type.symbol symbol=show#1 type=(this: User) => string

        return "user";
    }
}

extension of User implements Show {
/// @definition.extension symbol=<module>#3 form=inherent target=User
/// @definition.implements symbol=<module>#3 source=Show target=Show
/// @definition.method symbol=show#2 slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @type.symbol symbol=show#2 type=(this: User) => string

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

#[test]
fn test_extension_implements_clause_requires_members() {
    let session = TestSession::single(
        r#"
newtype interface Show {
    show(): string;
}

struct User {}

extension of User implements Show {}
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

extension of User implements Show {}

=== checked ===
newtype interface Show {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show nominal=true
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {}
/// @definition.extension symbol=<module>#2 source="extension of User implements Show {}" form=inherent target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show
"#,
        r#"
/// @diagnostic.error code=EC203 message="type 'User' does not implement interface 'Show'"
/// @diagnostic.label line=8 column=30 source="extension of User implements Show {}"
"#,
    );
}

#[test]
fn test_extension_implements_clause_requires_inherited_members() {
    let session = TestSession::single(
        r#"
newtype interface Named {
    name(): string;
}

newtype interface Show extends Named {
    show(): string;
}

struct User {}

extension of User implements Show {
    show(): string {
        return "user";
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Named {
    name(): string;
}

newtype interface Show extends Named {
    show(): string;
}

struct User {}

extension of User implements Show {
    show(): string {
        return "user";
    }
}

=== checked ===
newtype interface Named {
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named nominal=true
/// @definition.method symbol=Named.name source="name(): string" slot=name type=(this: Named) => string

    name(): string;
    /// @type.symbol symbol=Named.name source="name(): string" type=(this: Named) => string

}

newtype interface Show extends Named {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show nominal=true
/// @definition.extends symbol=Show source=Named target=Named
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string
/// @resolution.name source=Named target=Named

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=inherent target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @type.symbol symbol=show type=(this: User) => string

        return "user";
    }
}
"#,
        r#"
/// @diagnostic.error code=EC203 message="type 'User' does not implement interface 'Show'"
/// @diagnostic.label line=12 column=30 source="extension of User implements Show {"
"#,
    );
}

#[test]
fn test_extension_implements_clause_requires_interface() {
    let session = TestSession::single(
        r#"
struct User {}
struct NotInterface {}

extension of User implements NotInterface {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}
struct NotInterface {}

extension of User implements NotInterface {}

=== checked ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

struct NotInterface {}
/// @type.symbol symbol=NotInterface source="struct NotInterface {}" type=NotInterface
/// @definition.struct symbol=NotInterface source="struct NotInterface {}"

extension of User implements NotInterface {}
/// @definition.extension symbol=<module>#2 source="extension of User implements NotInterface {}" form=inherent target=User
/// @resolution.name source=User target=User
/// @resolution.name source=NotInterface target=NotInterface
"#,
        r#"
/// @diagnostic.error code=EC616 message="type 'User' can only implement interfaces, not 'NotInterface'"
/// @diagnostic.label line=5 column=30 source="extension of User implements NotInterface {}"
"#,
    );
}

#[test]
fn test_extension_implements_rejects_type_alias_interface() {
    let session = TestSession::single(
        r#"
struct User {}
interface Show {
    show(): string;
}
type Alias = Show;

extension of User implements Alias {
    show(): string {
        return "";
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
interface Show {
    show(): string;
}
type Alias = Show;

extension of User implements Alias {
    show(): string {
        return "";
    }
}

=== checked ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

interface Show {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}
type Alias = Show;
/// @type.symbol symbol=Alias source="type Alias = Show" type=Show
/// @definition.type symbol=Alias source="type Alias = Show" value=Show
/// @resolution.name source=Show target=Show

extension of User implements Alias {
/// @definition.extension symbol=<module>#2 form=inherent target=User
/// @definition.method symbol=show slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Alias target=Alias

    show(): string {
    /// @type.symbol symbol=show type=(this: User) => string

        return "";
    }
}
"#,
        r#"
/// @diagnostic.error code=EC616 message="type 'User' can only implement interfaces, not 'Alias'"
/// @diagnostic.label line=8 column=30 source="extension of User implements Alias {"
"#,
    );
}
