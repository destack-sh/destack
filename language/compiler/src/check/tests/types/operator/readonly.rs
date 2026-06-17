use crate::tests::{DirRows, TestSession};

#[test]
fn test_readonly_object_rejects_nested_field_writes() {
    let session = TestSession::single(
        r#"
type User = {
    profile: {
        name: string;
    };
};

declare const user: readonly User;
user.profile.name = "Grace";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = {
    profile: {
        name: string;
    };
};

declare const user: readonly User;
user.profile.name = "Grace";

=== checked ===
type User = {
/// @type.symbol symbol=User source="type User = {\n    profile: {\n        name: string;\n    };\n}" type={ profile: { name: string } }
/// @definition.type symbol=User source="type User = {\n    profile: {\n        name: string;\n    };\n}" value={ profile: { name: string } }

    profile: {
        name: string;
    };
};

declare const user: readonly User;
/// @type.symbol symbol=user source=user type=readonly User
/// @resolution.name source=User target=User

user.profile.name = "Grace";
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=readonly User kind=field key=profile
/// @resolution.member source=user.profile.name receiver=readonly { name: string } kind=field key=name
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=9 column=1 source="user.profile.name = \"Grace\";"
"#,
    );
}

#[test]
fn test_readonly_arrays_accept_mutable_arrays() {
    let session = TestSession::single(
        r#"
declare let values: number[];
let frozen: readonly number[] = values;

frozen satisfies readonly number[];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare let values: number[];
let frozen: readonly number[] = values;

frozen satisfies readonly number[];

=== checked ===
declare let values: number[];
/// @type.symbol symbol=values source=values type=Array<number>

let frozen: readonly number[] = values;
/// @type.symbol symbol=frozen source=frozen type=readonly Array<number>
/// @resolution.name source=values target=values

frozen satisfies readonly number[];
/// @resolution.name source=frozen target=frozen
"#,
    );
}

#[test]
fn test_readonly_arrays_reject_mutable_assignment() {
    let session = TestSession::single(
        r#"
declare let frozen: readonly number[];
let bad: number[] = frozen;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare let frozen: readonly number[];
let bad: number[] = frozen;

=== checked ===
declare let frozen: readonly number[];
/// @type.symbol symbol=frozen source=frozen type=readonly Array<number>

let bad: number[] = frozen;
/// @type.symbol symbol=bad source=bad type=Array<number>
/// @resolution.name source=frozen target=frozen
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'readonly Array<number>' is not assignable to type 'Array<number>'"
/// @diagnostic.label line=3 column=5 source="let bad: number[] = frozen;"
"#,
    );
}

#[test]
fn test_readonly_struct_rejects_nested_field_writes() {
    let session = TestSession::single(
        r#"
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

declare const user: readonly User;
user.profile.name = "Grace";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

declare const user: readonly User;
user.profile.name = "Grace";

=== checked ===
struct Profile {
/// @type.symbol symbol=Profile type=Profile
/// @definition.struct symbol=Profile

    name: string;
    /// @type.symbol symbol=Profile.name source="name: string" type=string
}

struct User {
/// @type.symbol symbol=User type=User
/// @definition.struct symbol=User

    profile: Profile;
    /// @type.symbol symbol=User.profile source="profile: Profile" type=Profile
    /// @resolution.name source=Profile target=Profile
}

declare const user: readonly User;
/// @type.symbol symbol=user source=user type=readonly User
/// @resolution.name source=User target=User

user.profile.name = "Grace";
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=readonly User kind=symbol target=User.profile
/// @resolution.member source=user.profile.name receiver=readonly Profile kind=symbol target=Profile.name
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=11 column=1 source="user.profile.name = \"Grace\";"
"#,
    );
}
