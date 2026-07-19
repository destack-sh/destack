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
/// @type.symbol symbol=User type={ profile: { name: string } }
/// @definition.type symbol=User value={ profile: { name: string } }

    profile: {
        name: string;
    };
};

declare const user: readonly User;
/// @type.symbol symbol=user source=user type=Readonly<User> reduced=Readonly<{ profile: { name: string } }>
/// @resolution.name source=User target=User

user.profile.name = "Grace";
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Readonly<{ profile: { name: string } }> kind=field key=profile
/// @resolution.pattern.assign source=user.profile.name kind=place place=field(name) type=string
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=9 column=14 span="name" line_source="user.profile.name = \"Grace\";"
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
declare let values: float64[];
let frozen: readonly float64[] = values;

frozen satisfies readonly number[];

=== checked ===
declare let values: number[];
/// @type.symbol symbol=values source=values type=Array<float64>

let frozen: readonly number[] = values;
/// @type.symbol symbol=frozen source=frozen type=readonly Array<float64>
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
declare let frozen: readonly float64[];
let bad: float64[] = frozen;

=== checked ===
declare let frozen: readonly number[];
/// @type.symbol symbol=frozen source=frozen type=readonly Array<float64>

let bad: number[] = frozen;
/// @type.symbol symbol=bad source=bad type=Array<float64>
/// @resolution.name source=frozen target=frozen
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'readonly Array<float64>' is not assignable to type 'Array<float64>'"
/// @diagnostic.label line=3 column=21 span="frozen" line_source="let bad: number[] = frozen;"
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
/// @definition.field symbol=Profile.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Profile.name source="name: string" type=string

}

struct User {
/// @type.symbol symbol=User type=User
/// @definition.struct symbol=User
/// @definition.field symbol=User.profile source="profile: Profile" key=profile type=Profile

    profile: Profile;
    /// @type.symbol symbol=User.profile source="profile: Profile" type=Profile
    /// @resolution.name source=Profile target=Profile

}

declare const user: readonly User;
/// @type.symbol symbol=user source=user type=Readonly<User>
/// @resolution.name source=User target=User

user.profile.name = "Grace";
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Readonly<User> kind=symbol target=User.profile
/// @resolution.pattern.assign source=user.profile.name kind=place place=field(Profile.name) type=string
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=11 column=14 span="name" line_source="user.profile.name = \"Grace\";"
"#,
    );
}
