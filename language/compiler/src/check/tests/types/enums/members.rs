use crate::tests::{DirRows, TestSession};

#[test]
fn test_enum_static_member_access_selects_variant_symbol() {
    let session = TestSession::single(
        r#"
enum Status {
    Active = 1,
    Inactive = 2,

    static Default = Status.Active;
}

const value = Status.Default;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
enum Status {
/// @type.symbol symbol=Status type=Status

    Active = 1,
    /// @type.symbol symbol=Status.Active type=Status

    Inactive = 2,
    /// @type.symbol symbol=Status.Inactive type=Status

    static Default = Status.Active;
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Active receiver=Status kind=symbol target=Status.Active
    /// @type.symbol symbol=Status.Default type=Status
}

const value = Status.Default;
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Default receiver=Status kind=symbol target=Status.Default
/// @type.symbol symbol=value type=Status
"#,
    );
}

#[test]
fn test_enum_instance_method_call_selects_method_symbol() {
    let session = TestSession::single(
        r#"
enum Status {
    Active = 1,
    Inactive = 2,

    isActive(): boolean {
        return this == Status.Active;
    }
}

const value = Status.Active.isActive();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
enum Status {
/// @type.symbol symbol=Status type=Status

    Active = 1,
    /// @type.symbol symbol=Status.Active type=Status

    Inactive = 2,
    /// @type.symbol symbol=Status.Inactive type=Status

    isActive(): boolean {
    /// @type.symbol symbol=Status.isActive type=(this: Status) => boolean

        return this == Status.Active;
        /// @resolution.name source=this target=this
        /// @resolution.name source=Status target=Status
        /// @resolution.member source=Status.Active receiver=Status kind=symbol target=Status.Active
        /// @type.node source="this == Status.Active" type=boolean
    }
}

const value = Status.Active.isActive();
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Active receiver=Status kind=symbol target=Status.Active
/// @resolution.member source=Status.Active.isActive receiver=Status kind=symbol target=Status.isActive
/// @resolution.call source="Status.Active.isActive()" parameters=() return=boolean kind=symbol target=Status.isActive receiver=Status
/// @type.symbol symbol=value type=boolean
"#,
    );
}
