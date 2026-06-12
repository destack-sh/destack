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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
type Named = { name: string };

const value: Named = { name: "Ada", extra: true };

=== checked ===
type Named = { name: string };
/// @type.symbol symbol=Named source="type Named = { name: string }" type={ name: string }
/// @definition.type symbol=Named source="type Named = { name: string }" value={ name: string }

const value: Named = { name: "Ada", extra: true };
/// @type.symbol symbol=value source=value type={ name: string }
/// @resolution.name source=Named target=Named
/// @type.node source="{ name: \"Ada\", extra: true }" type=Managed<{ name: "Ada"; extra: true }>
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source=true type=true

/// @check.stats.solve variables=0 types=8 constraints=1 obligations=0 solutions=0 bounds=0 decisions=1

"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'extra' in object literal for type '{ name: string }'"
/// @diagnostic.label line=4 column=22 source="const value: Named = { name: \"Ada\", extra: true };"
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
type Named = { name: string };

const source: { name: string; extra: boolean } = { name: "Ada", extra: true };
const value: Named = source;

=== checked ===
type Named = { name: string };
/// @type.symbol symbol=Named source="type Named = { name: string }" type={ name: string }
/// @definition.type symbol=Named source="type Named = { name: string }" value={ name: string }

const source = { name: "Ada", extra: true };
/// @type.symbol symbol=source source=source type=Managed<{ name: string; extra: boolean }>
/// @type.node source="{ name: \"Ada\", extra: true }" type=Managed<{ name: "Ada"; extra: true }>
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source=true type=true

const value: Named = source;
/// @type.symbol symbol=value source=value type={ name: string }
/// @resolution.name source=Named target=Named
/// @type.node source=source type=Managed<{ name: string; extra: boolean }>
/// @resolution.name source=source target=source

/// @check.stats.solve variables=0 types=13 constraints=1 obligations=0 solutions=0 bounds=0 decisions=2
"#,
    );
}
