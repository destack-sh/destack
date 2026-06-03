use crate::tests::{DirRows, TestSession};

#[test]
fn test_non_fresh_object_allows_extra_properties() {
    let session = TestSession::single(
        r#"
type Person = { name: string };

const source = { name: "Ada", extra: true };
const value: Person = source;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
type Person = { name: string };
/// @type.symbol symbol=Person type={ name: string }
/// @type.symbol symbol=Person.name type=string

const source = { name: "Ada", extra: true };
/// @type.symbol symbol=source type={ name: string; extra: boolean }
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: string; extra: boolean }
/// @type.node source="\"Ada\"" type=string
/// @type.node source=true type=boolean

const value: Person = source;
/// @type.symbol symbol=value type={ name: string }
/// @resolution.name source=Person target=Person
/// @type.node source=source type={ name: string; extra: boolean }
/// @resolution.name source=source target=source
"#,
    );
}

#[test]
fn test_generic_object_literal_freshness_only_guides_inference() {
    let session = TestSession::single(
        r#"
function keep<T: { name: string }>(value: T): T {
    return value;
}

const value = keep({ name: "Ada", extra: true });
const extra = value.extra;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function keep<T: { name: string }>(value: T): T {
/// @generic.template symbol=keep parameters=[T: { name: string }]
/// @type.symbol symbol=keep type=<T: { name: string }>(T) => T
/// @type.symbol symbol=keep.name type=string
/// @type.symbol symbol=value#1 type=T
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value#1

}

const value = keep({ name: "Ada", extra: true });
/// @type.symbol symbol=value#2 type={ name: string; extra: boolean }
/// @generic.instance source="keep({ name: \"Ada\", extra: true })" id="keep<{ name: string; extra: boolean }>"
/// @type.node source="keep({ name: \"Ada\", extra: true })" type={ name: string; extra: boolean }
/// @resolution.name source=keep target=keep
/// @resolution.call source="keep({ name: \"Ada\", extra: true })" parameters=({ name: string; extra: boolean }) return={ name: string; extra: boolean } kind=symbol target=keep instance="keep<{ name: string; extra: boolean }>"
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: string; extra: boolean }
/// @type.node source="\"Ada\"" type=string
/// @type.node source=true type=boolean

const extra = value.extra;
/// @type.symbol symbol=extra type=boolean
/// @type.node source=value type={ name: string; extra: boolean }
/// @type.node source=value.extra type=boolean
/// @resolution.name source=value target=value#2
/// @resolution.member source=value.extra receiver={ name: string; extra: boolean } kind=field key=extra
/// @generic.instance id="keep<{ name: string; extra: boolean }>" symbol=keep arguments=[{ name: string; extra: boolean }]
"#,
    );
}
