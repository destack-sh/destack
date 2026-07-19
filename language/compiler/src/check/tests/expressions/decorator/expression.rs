use crate::tests::{DirRows, TestSession};

#[test]
fn test_decorator_resolves_outside_declaration_scope() {
    let session = TestSession::single(
        r#"
newtype mark = (string,);

@mark("checked")
declare function read(mark: int32): void;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
newtype mark = (string,);

@mark("checked")
declare function read(mark: int32): void;

=== checked ===
newtype mark = (string,);
/// @type.symbol symbol=mark source="newtype mark = (string,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = (string,)" backing=(string,)

@mark("checked")
/// @decorator.node source="@mark(\"checked\")" owner="declare function read(mark: int32): void" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("checked") as string) newtype=mark backing=(string,) value="mark(\"checked\")"
/// @type.node source=mark type=mark
/// @resolution.name source=mark target=mark
/// @type.node source="\"checked\"" type="checked"

declare function read(mark: int32): void;
/// @type.symbol symbol=read source="declare function read(mark: int32): void" type=(int32) => void
/// @type.symbol symbol=read.mark source="mark: int32" type=int32
"#,
    );
}

#[test]
fn test_member_access_decorator_reports_error() {
    let session = TestSession::single(
        r#"
const subject = 1;

@subject.field
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const subject: 1 = 1;

@subject.field
const value: 1 = 1;

=== checked ===
const subject = 1;
/// @type.symbol symbol=subject source=subject type=1
/// @type.node source=1 type=1

@subject.field
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=invalid-decorator-target message="decorator must name a newtype declaration"
/// @diagnostic.label line=4 column=10 span="field" line_source="@subject.field"
"#,
    );
}

#[test]
fn test_decorator_preserves_nested_newtype_value() {
    let session = TestSession::single(
        r#"
newtype Payload = { reason: string };
newtype mark = (Payload,);

@mark(Payload({ reason: "intentional" }))
const value = 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
newtype Payload = { reason: string };
newtype mark = (Payload,);

@mark(Payload({ reason: "intentional" }))
const value: 1 = 1;

=== checked ===
newtype Payload = { reason: string };
/// @type.symbol symbol=Payload source="newtype Payload = { reason: string }" type=Payload
/// @definition.newtype symbol=Payload source="newtype Payload = { reason: string }" backing={ reason: string }

newtype mark = (Payload,);
/// @type.symbol symbol=mark source="newtype mark = (Payload,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = (Payload,)" backing=(Payload,)
/// @resolution.name source=Payload target=Payload

@mark(Payload({ reason: "intentional" }))
/// @decorator.node source="@mark(Payload({ reason: \"intentional\" }))" owner="const value = 1" expression=mark target=mark type=mark kind=newtype parameters=(Payload) arguments=(provided(Payload({ reason: "intentional" })) as Payload) newtype=mark backing=(Payload,) value="mark(Payload({ reason: \"intentional\" }))"
/// @type.node source=mark type=mark
/// @resolution.name source=mark target=mark
/// @type.node source="Payload({ reason: \"intentional\" })" type=Payload
/// @type.node source=Payload type=Payload
/// @resolution.name source=Payload target=Payload
/// @resolution.construct source="Payload({ reason: \"intentional\" })" parameters=({ reason: string }) arguments=(provided({ reason: "intentional" }) as { reason: string }) return=Payload kind=newtype target=Payload backing={ reason: string }
/// @type.node source={ reason: "intentional" } type={ reason: "intentional" }
/// @type.node source="\"intentional\"" type="intentional"

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
    );
}
