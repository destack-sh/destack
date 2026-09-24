use crate::tests::{DirRows, TestSession};

#[test]
fn test_enum_member_type_retains_variant_identity() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

type Selected = Mode.Read;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Mode {
    Read = 1,
    Write = 2,
}

type Selected = Mode.Read;

=== dir ===
enum Mode {
/// @type.symbol symbol=Mode type=Mode
/// @definition.enum symbol=Mode
/// @definition.variant symbol=Mode.Read source="Read = 1" key=Read value=1
/// @definition.variant symbol=Mode.Write source="Write = 2" key=Write value=2

    Read = 1,
    /// @type.symbol symbol=Mode.Read source="Read = 1" type=Mode.Read

    Write = 2,
    /// @type.symbol symbol=Mode.Write source="Write = 2" type=Mode.Write

}

type Selected = Mode.Read;
/// @type.symbol symbol=Selected source="type Selected = Mode.Read" type=Mode.Read
/// @definition.type symbol=Selected source="type Selected = Mode.Read" value=Mode.Read
/// @resolution.name source=Mode.Read target=Mode
/// @resolution.path source=Mode.Read index=1 target=Mode.Read
"#,
    );
}

#[test]
fn test_enum_member_initializes_inferred_binding() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

const selected = Mode.Read;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Mode {
    Read = 1,
    Write = 2,
}

const selected = Mode.Read;

=== dir ===
enum Mode {
/// @type.symbol symbol=Mode type=Mode
/// @definition.enum symbol=Mode
/// @definition.variant symbol=Mode.Read source="Read = 1" key=Read value=1
/// @definition.variant symbol=Mode.Write source="Write = 2" key=Write value=2

    Read = 1,
    /// @type.symbol symbol=Mode.Read source="Read = 1" type=Mode.Read
    /// @type.node source=1 type=1

    Write = 2,
    /// @type.symbol symbol=Mode.Write source="Write = 2" type=Mode.Write
    /// @type.node source=2 type=2

}

const selected = Mode.Read;
/// @type.symbol symbol=selected source=selected type=Mode.Read
/// @resolution.pattern source=selected kind=binding target=selected
/// @type.node source=Mode type=Mode
/// @type.node source=Mode.Read type=Mode.Read
/// @resolution.name source=Mode target=Mode
/// @resolution.member source=Mode.Read receiver=Mode type=Mode.Read kind=symbol target_receiver=Mode target=Mode.Read
"#,
    );
}

#[test]
fn test_narrowed_enum_member_initializes_inferred_binding() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

declare const mode: Mode;
switch (mode) {
    case Mode.Read:
        const selected = mode;
        break;
    default:
        const remaining = mode;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Mode {
    Read = 1,
    Write = 2,
}

declare const mode: Mode;
switch (mode) {
    case Mode.Read:
        const selected = mode;
        break;
    default:
        const remaining = mode;
}

=== dir ===
enum Mode {
/// @type.symbol symbol=Mode type=Mode
/// @definition.enum symbol=Mode
/// @definition.variant symbol=Mode.Read source="Read = 1" key=Read value=1
/// @definition.variant symbol=Mode.Write source="Write = 2" key=Write value=2

    Read = 1,
    /// @type.symbol symbol=Mode.Read source="Read = 1" type=Mode.Read
    /// @type.node source=1 type=1

    Write = 2,
    /// @type.symbol symbol=Mode.Write source="Write = 2" type=Mode.Write
    /// @type.node source=2 type=2

}

declare const mode: Mode;
/// @type.symbol symbol=mode source=mode type=Mode
/// @resolution.pattern source=mode kind=binding target=mode
/// @resolution.name source=Mode target=Mode

switch (mode) {
/// @type.node source=mode type=Mode
/// @resolution.name source=mode target=mode
/// @resolution.place source=mode placement="local" lifetime="static" access="immutable"
/// @resolution.access source=mode root=mode

    case Mode.Read:
    /// @resolution.operator type=boolean operator="===" kind=builtin operands=[mode as Mode families=(Mode), Mode.Read as Mode.Read families=(Mode)]
    /// @type.node source=Mode type=Mode
    /// @type.node source=Mode.Read type=Mode.Read
    /// @resolution.name source=Mode target=Mode
    /// @resolution.member source=Mode.Read receiver=Mode type=Mode.Read kind=symbol target_receiver=Mode target=Mode.Read

        const selected = mode;
        /// @type.symbol symbol=selected source=selected type=Mode.Read
        /// @resolution.pattern source=selected kind=binding target=selected
        /// @type.node source=mode type=Mode.Read
        /// @resolution.name source=mode target=mode
        /// @resolution.access source=mode root=mode

        break;
        /// @type.node source=break type=never
        /// @resolution.transfer source=break target=switch

    default:
        const remaining = mode;
        /// @type.symbol symbol=remaining source=remaining type=Mode.Write
        /// @resolution.pattern source=remaining kind=binding target=remaining
        /// @type.node source=mode type=Mode.Write
        /// @resolution.name source=mode target=mode
        /// @resolution.access source=mode root=mode

}
"#,
    );
}

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

    session.assert_dir(
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

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Active source="Active = 1" key=Active value=1
/// @definition.field symbol=Status.Default source="static Default = Status.Active" key=Default static=true type=Status
/// @definition.variant symbol=Status.Inactive source="Inactive = 2" key=Inactive value=2

    Active = 1,
    /// @type.symbol symbol=Status.Active source="Active = 1" type=Status.Active

    Inactive = 2,
    /// @type.symbol symbol=Status.Inactive source="Inactive = 2" type=Status.Inactive

    static Default = Status.Active;
    /// @type.symbol symbol=Status.Default source="static Default = Status.Active" type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Active receiver=Status type=Status.Active kind=symbol target_receiver=Status target=Status.Active

}

const value: Status = Status.Default;
/// @type.symbol symbol=value source=value type=Status
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Status target=Status
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Default receiver=Status type=Status kind=field target_receiver=Status key=Default target=Status.Default target_type=Status
/// @resolution.place source=Status.Default placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=Status.Default root=Status keys=[Default]
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Status {
    Active = 1,
    Inactive = 2,

    isActive(): boolean {
        return (this as Status) == Status.Active;
    }
}

const value: boolean = Status.Active.isActive<"frame">();

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Active source="Active = 1" key=Active value=1
/// @definition.variant symbol=Status.Inactive source="Inactive = 2" key=Inactive value=2
/// @definition.method symbol=Status.isActive slot=isActive type=<Status.isActive.'a>(this: &Status.isActive.'a readonly Status) => boolean

    Active = 1,
    /// @type.symbol symbol=Status.Active source="Active = 1" type=Status.Active
    /// @type.node source=1 type=1

    Inactive = 2,
    /// @type.symbol symbol=Status.Inactive source="Inactive = 2" type=Status.Inactive
    /// @type.node source=2 type=2

    isActive(): boolean {
    /// @generic.template symbol=Status.isActive parameters=('a)
    /// @type.symbol symbol=Status.isActive type=<Status.isActive.'a>(this: &Status.isActive.'a readonly Status) => boolean
    /// @type.symbol symbol=Status.isActive.this type=&Status.isActive.'a readonly Status

        return this == Status.Active;
        /// @type.node source="this == Status.Active" type=boolean
        /// @type.node source=this type=&Status.isActive.'a readonly Status
        /// @resolution.operator source="this == Status.Active" type=boolean operator="==" kind=builtin operands=[this as Status families=(Status), Status.Active as Status.Active families=(Status)]
        /// @resolution.receiver source=this kind=this declaration=Status type=&Status.isActive.'a readonly Status
        /// @resolution.place source=this placement=Status.isActive.'a lifetime=Status.isActive.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @type.node source=Status type=Status
        /// @type.node source=Status.Active type=Status.Active
        /// @resolution.name source=Status target=Status
        /// @resolution.member source=Status.Active receiver=Status type=Status.Active kind=symbol target_receiver=Status target=Status.Active

    }
}

const value = Status.Active.isActive();
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=Status type=Status
/// @type.node source=Status.Active type=Status.Active
/// @type.node source=Status.Active.isActive type=<Status.isActive.'a>(this: &Status.isActive.'a readonly Status) => boolean
/// @type.node source=Status.Active.isActive() type=boolean
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Active receiver=Status type=Status.Active kind=symbol target_receiver=Status target=Status.Active
/// @resolution.member source=Status.Active.isActive receiver=Status.Active type=<Status.isActive.'a>(this: &Status.isActive.'a readonly Status) => boolean kind=symbol target_receiver=Status.Active target=Status.isActive
/// @resolution.call source=Status.Active.isActive() parameters=() return=boolean regions=("frame" & "local") kind=symbol target=Status.isActive receiver=Status.Active adjustments=(borrow(&'frame readonly Status.Active)) instance="Status.isActive<\"frame\" & \"local\">"
/// @generic.instantiation id="Status.isActive<\"frame\" & \"local\">" template=Status.isActive arguments=("frame" & "local")
/// @generic.instance id="Status.isActive<\"bound0\" & \"local\">" template=Status.isActive arguments=("bound0" & "local")
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Ready,
    Ready,
}

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Ready#1 source=Ready key=Ready value=0
/// @definition.variant symbol=Status.Ready#2 source=Ready key=Ready value=1

    Ready,
    /// @type.symbol symbol=Status.Ready#1 source=Ready type=Status.Ready

    Ready,
    /// @type.symbol symbol=Status.Ready#2 source=Ready type=Status.Ready

}
"#,
        r#"
/// @diagnostic.error id=duplicate-member message="member 'Ready' is already declared"
/// @diagnostic.label line=4 column=5 span="Ready" line_source="Ready,"
"#,
    );
}
