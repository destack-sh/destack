use crate::tests::{DirRows, TestSession};

#[test]
fn test_fresh_object_rejects_excess_property_at_typed_binding() {
    let session = TestSession::single(
        r#"
type Named = { name: string };

const value: Named = { name: "Ada", extra: true };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
type Named = { name: string };
/// @type.symbol symbol=Named type={ name: string }

const value: Named = { name: "Ada", extra: true };
/// @resolution.name source=Named target=Named
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: "Ada"; extra: true }
/// @type.symbol symbol=value type=Named

"#,
        r#"
/// @diagnostic.error code=EC205 message="excess property 'extra'"
/// @diagnostic.label line=4 column=37 source="const value: Named = { name: \"Ada\", extra: true };"
"#,
    );
}

#[test]
fn test_stale_object_allows_extra_properties() {
    let session = TestSession::single(
        r#"
type Named = { name: string };

const source = { name: "Ada", extra: true };
const value: Named = source;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
type Named = { name: string };
/// @type.symbol symbol=Named type={ name: string }

const source = { name: "Ada", extra: true };
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: string; extra: boolean }
/// @type.symbol symbol=source type={ name: string; extra: boolean }

const value: Named = source;
/// @resolution.name source=Named target=Named
/// @resolution.name source=source target=source
/// @type.symbol symbol=value type=Named
"#,
    );
}
