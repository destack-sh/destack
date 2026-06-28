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

const value: Status = Status.Default;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Active = 1,
    Inactive = 2,

    static Default: Status = Status.Active;
}

const value: Status = Status.Default;

=== checked ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Active source="Active = 1" key=Active
/// @definition.field symbol=Status.Default source="static Default = Status.Active" key=Default static=true type=Status
/// @definition.variant symbol=Status.Inactive source="Inactive = 2" key=Inactive

    Active = 1,
    /// @type.symbol symbol=Status.Active source="Active = 1" type=Status.Active

    Inactive = 2,
    /// @type.symbol symbol=Status.Inactive source="Inactive = 2" type=Status.Inactive

    static Default = Status.Active;
    /// @type.symbol symbol=Status.Default source="static Default = Status.Active" type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Active receiver=Status kind=symbol target=Status.Active

}

const value: Status = Status.Default;
/// @type.symbol symbol=value source=value type=Status
/// @resolution.name source=Status target=Status
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Default receiver=Status kind=symbol target=Status.Default
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
=== annotated ===
enum Status {
    Active = 1,
    Inactive = 2,

    isActive(): boolean {
        return this == Status.Active;
    }
}

const value: boolean = Status.Active.isActive();

=== checked ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Active source="Active = 1" key=Active
/// @definition.variant symbol=Status.Inactive source="Inactive = 2" key=Inactive
/// @definition.method symbol=Status.isActive slot=isActive type=(this: Status) => boolean

    Active = 1,
    /// @type.symbol symbol=Status.Active source="Active = 1" type=Status.Active
    /// @type.node source=1 type=1

    Inactive = 2,
    /// @type.symbol symbol=Status.Inactive source="Inactive = 2" type=Status.Inactive
    /// @type.node source=2 type=2

    isActive(): boolean {
    /// @type.symbol symbol=Status.isActive type=(this: Status) => boolean

        return this == Status.Active;
        /// @type.node source="this == Status.Active" type=boolean
        /// @type.node source=this type=Status
        /// @resolution.call source="this == Status.Active" parameters=() return=boolean kind=builtin builtin=binary.equal
        /// @resolution.receiver source=this kind=this declaration=Status type=Status
        /// @type.node source=Status type=Status
        /// @type.node source=Status.Active type=Status.Active
        /// @resolution.name source=Status target=Status
        /// @resolution.member source=Status.Active receiver=Status kind=symbol target=Status.Active

    }
}

const value = Status.Active.isActive();
/// @type.symbol symbol=value source=value type=boolean
/// @type.node source=Status type=Status
/// @type.node source=Status.Active type=Status.Active
/// @type.node source=Status.Active.isActive type=(this: Status) => boolean
/// @type.node source=Status.Active.isActive() type=boolean
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Active receiver=Status kind=symbol target=Status.Active
/// @resolution.member source=Status.Active.isActive receiver=Status.Active kind=symbol target=Status.isActive
/// @resolution.call source=Status.Active.isActive() parameters=() return=boolean kind=symbol target=Status.isActive receiver=Status.Active
"#,
    );
}

#[test]
fn test_enum_duplicate_member_reports_error() {
    let session = TestSession::single(
        r#"
enum Status {
    Ready,
    Ready,
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Ready,
    Ready,
}

=== checked ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Ready#1 source=Ready key=Ready
/// @definition.variant symbol=Status.Ready#2 source=Ready key=Ready

    Ready,
    /// @type.symbol symbol=Status.Ready#1 source=Ready type=Status.Ready#1

    Ready,
    /// @type.symbol symbol=Status.Ready#2 source=Ready type=Status.Ready#2

}
"#,
        r#"
/// @diagnostic.error code=EC612 message="member 'Ready' is already declared"
/// @diagnostic.label line=4 column=5 span="Ready" line_source="Ready,"
"#,
    );
}
