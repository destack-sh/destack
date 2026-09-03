use crate::tests::{DirRows, TestSession};

/// A readonly object rejects a write to a nested field.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
type User = {
/// @type.symbol symbol=User type={ profile: { name: string } }
/// @definition.type symbol=User value={ profile: { name: string } }

    profile: {
    /// @type.symbol symbol=User.profile type={ name: string }

        name: string;
        /// @type.symbol symbol=User.name source="name: string" type=string

    };
};

declare const user: readonly User;
/// @type.symbol symbol=user source=user type=Readonly<User>
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

user.profile.name = "Grace";
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Readonly<User> type=Readonly<{ name: string }> kind=field target_receiver=Readonly<User> key=profile target_type=Readonly<{ name: string }>
/// @resolution.place source=user placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=user root=user
/// @resolution.place source=user.profile placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=user.profile root=user keys=[profile]
/// @resolution.pattern.assign source=user.profile.name kind=place
/// @resolution.access source=user.profile.name root=user keys=[profile, name]
/// @resolution.assignment source=user.profile.name write="receiver=Readonly<{ name: string }>, target=field(receiver=Readonly<{ name: string }>, target=name, type=Readonly<string>), type=Readonly<string>" type=Readonly<string>
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=9 column=14 span="name" line_source="user.profile.name = \"Grace\";"
"#,
    );
}

/// A readonly array binding accepts a mutable array.
#[test]
fn test_readonly_arrays_accept_mutable_arrays() {
    let session = TestSession::single(
        r#"
declare let values: number[];
let frozen: readonly number[] = values;

frozen satisfies readonly number[];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare let values: float64[];
let frozen: readonly float64[] = values;

frozen satisfies readonly number[];

=== dir ===
declare let values: number[];
/// @type.symbol symbol=values source=values type=float64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="elementSlot<float64, \"exclusive\">" template=elementSlot arguments=(float64, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive float64[], usize) => &elementSlot.'a exclusive MaybeUninit<float64>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<float64>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<float64>>, usize) => &elementSlot.'a exclusive MaybeUninit<float64>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<float64>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<float64>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<float64>>, usize) => &elementSlot.'a exclusive MaybeUninit<float64>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive float64[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<float64>>)
/// @generic.instance id="initAsPointer<float64, \"exclusive\">" template=initAsPointer arguments=(float64, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<float64>) => Raw<float64>)
/// @generic.instance id="sliceIndex<MaybeUninit<float64>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<float64>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<float64>>, usize) => &sliceIndex.'a exclusive MaybeUninit<float64>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=MaybeUninit<MaybeUninit<float64>> template=MaybeUninit arguments=(MaybeUninit<float64>)
/// @generic.instance id=MaybeUninit<float64> template=MaybeUninit arguments=(float64)
/// @generic.instance id=assumeInitDrop#1<float64> template=assumeInitDrop#1 arguments=(float64)
/// @generic.instance id=assumeInitDrop<float64> template=assumeInitDrop arguments=(float64) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<float64>) => Raw<float64>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<float64>)
/// @generic.instance id=clear<float64> template=clear arguments=(float64)
/// @generic.instance id=drop<float64> template=drop arguments=(float64)
/// @generic.instance id=dropInPlace<float64> template=dropInPlace arguments=(float64)
/// @generic.instance id=new<MaybeUninit<float64>> template=new arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<float64>> template=sliceAssumeInit arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceUninit<MaybeUninit<float64>> template=sliceUninit arguments=(MaybeUninit<float64>)
/// @generic.instance id=truncate<float64> template=truncate arguments=(float64) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<float64>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive float64[], usize) => &truncate.'a exclusive MaybeUninit<float64>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive float64[])

let frozen: readonly number[] = values;
/// @type.symbol symbol=frozen source=frozen type=readonly float64[]
/// @resolution.pattern source=frozen kind=binding target=frozen
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=values root=values

frozen satisfies readonly number[];
/// @resolution.name source=frozen target=frozen
/// @resolution.place source=frozen placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=frozen root=frozen
"#,
    );
}

/// A mutable array binding rejects a readonly array.
#[test]
fn test_readonly_arrays_reject_mutable_assignment() {
    let session = TestSession::single(
        r#"
declare let frozen: readonly number[];
let bad: number[] = frozen;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare let frozen: readonly float64[];
let bad: float64[] = frozen;

=== dir ===
declare let frozen: readonly number[];
/// @type.symbol symbol=frozen source=frozen type=readonly float64[]
/// @resolution.pattern source=frozen kind=binding target=frozen

let bad: number[] = frozen;
/// @type.symbol symbol=bad source=bad type=float64[]
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=frozen target=frozen
/// @resolution.place source=frozen placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=frozen root=frozen
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'readonly float64[]' is not assignable to type 'float64[]'"
/// @diagnostic.label line=3 column=21 span="frozen" line_source="let bad: number[] = frozen;"
/// @diagnostic.related line=3 column=10 span="number[]" line_source="let bad: number[] = frozen;" message="expected due to this annotation"
"#,
    );
}

/// A readonly struct rejects a write to a nested field.
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

user.profile.name = "Grace";
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Readonly<User> type=Profile kind=field target_receiver=Readonly<User> key=profile target=User.profile target_type=Profile
/// @resolution.place source=user placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=user root=user
/// @resolution.place source=user.profile placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=user.profile root=user keys=[profile]
/// @resolution.pattern.assign source=user.profile.name kind=place
/// @resolution.access source=user.profile.name root=user keys=[profile, name]
/// @resolution.assignment source=user.profile.name write="receiver=Readonly<Profile>, target=field(receiver=Readonly<Profile>, target=Profile.name, type=Readonly<string>), type=Readonly<string>" type=Readonly<string>
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=11 column=14 span="name" line_source="user.profile.name = \"Grace\";"
"#,
    );
}
