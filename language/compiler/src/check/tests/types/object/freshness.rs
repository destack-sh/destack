use crate::tests::{DirRows, TestSession};

#[test]
fn test_fresh_object_rejects_excess_property_at_typed_binding() {
    let session = TestSession::single(
        r#"
type Person = { name: string };

const value: Person = { name: "Ada", extra: true };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
type Person = { name: string };

const value: Person = { name: "Ada", extra: true };

=== checked ===
type Person = { name: string };
/// @type.symbol symbol=Person source="type Person = { name: string }" type={ name: string }
/// @definition.type symbol=Person source="type Person = { name: string }" value={ name: string }

const value: Person = { name: "Ada", extra: true };
/// @type.symbol symbol=value source=value type={ name: string }
/// @resolution.name source=Person target=Person
/// @type.node source="{ name: \"Ada\", extra: true }" type=Managed<{ name: string; extra: boolean }>
/// @type.node source="\"Ada\"" type=string
/// @type.node source=true type=boolean

/// @check.stats.solve variables=0 types=8 constraints=1 obligations=0 solutions=0 bounds=0 decisions=1

"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'extra' in object literal for type '{ name: string }'"
/// @diagnostic.label line=4 column=23 source="const value: Person = { name: \"Ada\", extra: true };"
"#,
    );
}

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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
type Person = { name: string };

const source: { name: string; extra: boolean } = { name: "Ada", extra: true };
const value: Person = source;

=== checked ===
type Person = { name: string };
/// @type.symbol symbol=Person source="type Person = { name: string }" type={ name: string }
/// @definition.type symbol=Person source="type Person = { name: string }" value={ name: string }

const source = { name: "Ada", extra: true };
/// @type.symbol symbol=source source=source type=Managed<{ name: string; extra: boolean }>
/// @type.node source="{ name: \"Ada\", extra: true }" type=Managed<{ name: string; extra: boolean }>
/// @type.node source="\"Ada\"" type=string
/// @type.node source=true type=boolean

const value: Person = source;
/// @type.symbol symbol=value source=value type={ name: string }
/// @resolution.name source=Person target=Person
/// @type.node source=source type=Managed<{ name: string; extra: boolean }>
/// @resolution.name source=source target=source

/// @check.stats.solve variables=0 types=13 constraints=1 obligations=0 solutions=0 bounds=0 decisions=2
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
=== annotated ===
function keep<T: { name: string }>(value: T): T {
    return value;
}

const value: { name: string; extra: boolean } = keep<{ name: string; extra: boolean }>({ name: "Ada", extra: true });
const extra: boolean = value.extra;

=== checked ===
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
