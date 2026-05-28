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

const value: Person = { name: "Ada", extra: true };
/// @resolution.name source=Person target=Person
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: "Ada"; extra: true }
/// @type.symbol symbol=value type=Person

"#,
        r#"
/// @diagnostic.error code=EC205 message="excess property 'extra'"
/// @diagnostic.label line=4 column=39 source="const value: Person = { name: \"Ada\", extra: true };"
"#);
}
