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
=== annotated ===
class User {}
declare const value: unknown;

const ok: boolean = value instanceof User;

=== checked ===
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
/// @resolution.predicate source="value instanceof User" kind=instanceof value=unknown target=User
/// @type.node source=User type=User
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_instanceof_narrows_positive_branch_to_class_arm() {
    let session = TestSession::single(
        r#"
class User {
    name: string;
}

class Team {
    title: string;
}

declare const value: User | Team;

if (value instanceof User) {
    value.name satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;
}

class Team {
    title: string;
}

declare const value: User | Team;

if (value instanceof User) {
    value.name satisfies string;
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

}

class Team {
/// @type.symbol symbol=Team type=Team
/// @definition.class symbol=Team
/// @definition.field symbol=Team.title source="title: string" key=title type=string

    title: string;
    /// @type.symbol symbol=Team.title source="title: string" type=string

}

declare const value: User | Team;
/// @type.symbol symbol=value source=value type=User | Team
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

if (value instanceof User) {
/// @type.node type=void | void
/// @type.node source="value instanceof User" type=boolean
/// @type.node source=value type=User | Team
/// @resolution.name source=value target=value
/// @resolution.predicate source="value instanceof User" kind=instanceof value=User | Team target=User
/// @type.node source=User type=User
/// @resolution.name source=User target=User

    value.name satisfies string;
    /// @type.node source="value.name satisfies string" type=string
    /// @type.node source=value type=User
    /// @type.node source=value.name type=string
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.name receiver=User kind=symbol target=User.name

}
"#,
    );
}

#[test]
fn test_instanceof_narrows_negative_branch_by_removing_class_arm() {
    let session = TestSession::single(
        r#"
class User {
    name: string;
}

class Team {
    title: string;
}

declare const value: User | Team;

if (value instanceof User) {
} else {
    value.title satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;
}

class Team {
    title: string;
}

declare const value: User | Team;

if (value instanceof User) {
} else {
    value.title satisfies string;
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

}

class Team {
/// @type.symbol symbol=Team type=Team
/// @definition.class symbol=Team
/// @definition.field symbol=Team.title source="title: string" key=title type=string

    title: string;
    /// @type.symbol symbol=Team.title source="title: string" type=string

}

declare const value: User | Team;
/// @type.symbol symbol=value source=value type=User | Team
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

if (value instanceof User) {
/// @type.node type=void | void
/// @type.node source="value instanceof User" type=boolean
/// @type.node source=value type=User | Team
/// @resolution.name source=value target=value
/// @resolution.predicate source="value instanceof User" kind=instanceof value=User | Team target=User
/// @type.node source=User type=User
/// @resolution.name source=User target=User

} else {
    value.title satisfies string;
    /// @type.node source="value.title satisfies string" type=string
    /// @type.node source=value type=Team
    /// @type.node source=value.title type=string
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.title receiver=Team kind=symbol target=Team.title

}
"#,
    );
}

#[test]
fn test_instanceof_rejects_non_class_right_hand_side() {
    let session = TestSession::single(
        r#"
interface Named {
    name: string;
}

declare const value: unknown;

const ok = value instanceof Named;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Named {
    name: string;
}

declare const value: unknown;

const ok: boolean = value instanceof Named;

=== checked ===
interface Named {
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named
/// @definition.field symbol=Named.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Named.name source="name: string" type=string

}

declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown

const ok = value instanceof Named;
/// @type.symbol symbol=ok source=ok type=boolean
/// @type.node source="value instanceof Named" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @resolution.predicate source="value instanceof Named" kind=instanceof value=unknown target=Named
/// @type.node source=Named type=Named
/// @resolution.name source=Named target=Named
"#,
        r#"
/// @diagnostic.error code=EC317 message="right-hand side of 'instanceof' must be a class"
/// @diagnostic.label line=8 column=29 source="const ok = value instanceof Named;"
"#,
    );
}

#[test]
fn test_instanceof_rejects_impossible_primitive_check() {
    let session = TestSession::single(
        r#"
class User {}
declare const value: string;

const ok = value instanceof User;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}
declare const value: string;

const ok: boolean = value instanceof User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const value: string;
/// @type.symbol symbol=value source=value type=string

const ok = value instanceof User;
/// @type.symbol symbol=ok source=ok type=boolean
/// @type.node source="value instanceof User" type=boolean
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @resolution.predicate source="value instanceof User" kind=instanceof value=string target=User
/// @type.node source=User type=User
/// @resolution.name source=User target=User
"#,
        r#"
/// @diagnostic.error code=EC318 message="type 'string' can never be an instance of 'User'"
/// @diagnostic.label line=5 column=12 source="const ok = value instanceof User;"
"#,
    );
}
