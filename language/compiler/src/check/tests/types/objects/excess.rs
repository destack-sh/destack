use crate::tests::{DirRows, TestSession};

#[test]
fn test_fresh_object_with_excess_property_reports_error() {
    let session = TestSession::single(
        r#"
type Person = { name: string };

const value: Person = { name: "Ada", extra: true };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
type Person = { name: string };
/// @type.symbol symbol=Person type={ name: string }
/// @type.symbol symbol=Person.name type=string

const value: Person = { name: "Ada", extra: true };
/// @type.symbol symbol=value type={ name: string }
/// @resolution.name source=Person target=Person
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: string; extra: boolean }
/// @type.node source="\"Ada\"" type=string
/// @type.node source=true type=boolean

"#,
        r#"
/// @diagnostic.error code=EC205 message="excess property 'extra'"
/// @diagnostic.label line=4 column=38 source="const value: Person = { name: \"Ada\", extra: true };"
"#,
    );
}
