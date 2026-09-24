use crate::tests::{DirRows, TestSession};

#[test]
fn test_direct_object_literal_rejects_excess_property_at_typed_binding() {
    let session = TestSession::single(
        r#"
type Person = { name: string };

const value: Person = { name: "Ada", extra: true };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Person = { name: string };

const value: Person = { name: "Ada", extra: true };

=== dir ===
type Person = { name: string };
/// @type.symbol symbol=Person source="type Person = { name: string }" type={ name: string }
/// @definition.type symbol=Person source="type Person = { name: string }" value={ name: string }
/// @type.symbol symbol=Person.name source="name: string" type=string

const value: Person = { name: "Ada", extra: true };
/// @type.symbol symbol=value source=value type=Person
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Person target=Person
/// @type.node source={ name: "Ada", extra: true } type={ name: string; extra: boolean }
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'extra' in object literal for type 'Person'"
/// @diagnostic.label line=4 column=23 span="{ name: \"Ada\", extra: true }" line_source="const value: Person = { name: \"Ada\", extra: true };"
/// @diagnostic.related line=4 column=14 span="Person" line_source="const value: Person = { name: \"Ada\", extra: true };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}

#[test]
fn test_named_object_value_rejects_aliased_extra_properties() {
    // aliased values store exactly into object-typed slots, wider rows need an interface
    let session = TestSession::single(
        r#"
type Person = { name: string };

const source = { name: "Ada", extra: true };
const value: Person = source;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Person = { name: string };

const source: { name: string; extra: boolean } = { name: "Ada", extra: true };
const value: Person = source;

=== dir ===
type Person = { name: string };
/// @type.symbol symbol=Person source="type Person = { name: string }" type={ name: string }
/// @definition.type symbol=Person source="type Person = { name: string }" value={ name: string }
/// @type.symbol symbol=Person.name source="name: string" type=string

const source = { name: "Ada", extra: true };
/// @type.symbol symbol=source source=source type={ name: string; extra: boolean }
/// @resolution.pattern source=source kind=binding target=source
/// @type.node source={ name: "Ada", extra: true } type={ name: string; extra: boolean }
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source=true type=true

const value: Person = source;
/// @type.symbol symbol=value source=value type=Person
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Person target=Person
/// @type.node source=source type={ name: string; extra: boolean }
/// @resolution.name source=source target=source
/// @resolution.place source=source placement="local" lifetime="static" access="immutable"
/// @resolution.access source=source root=source
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ name: string; extra: boolean }' is not assignable to type 'Person'"
/// @diagnostic.label line=5 column=23 span="source" line_source="const value: Person = source;"
/// @diagnostic.related line=5 column=14 span="Person" line_source="const value: Person = source;" message="expected due to this annotation"
/// @diagnostic.note message="'Person' reduces to '{ name: string }'"
/// @diagnostic.note message="'{ name: string }' stores its exact object type, declare an interface to accept structurally wider values"
"#,
    );
}

#[test]
fn test_generic_object_literal_keeps_inferred_extra_properties() {
    let session = TestSession::single(
        r#"
function keep<T: { name: string }>(value: T): T {
    return value;
}

const value = keep({ name: "Ada", extra: true });
const extra = value.extra;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function keep<T: { name: string }>(value: T): T {
    return value;
}

const value: { name: string; extra: boolean } = keep<{ name: string; extra: boolean }>({
    name: "Ada",
    extra: true,
});
const extra: boolean = value.extra;

=== dir ===
function keep<T: { name: string }>(value: T): T {
/// @generic.template symbol=keep parameters=(T: { name: string })
/// @type.symbol symbol=keep type=<T: { name: string }>(T) => T
/// @type.symbol symbol=keep.T source="T: { name: string }" type=T
/// @type.symbol symbol=keep.name source="name: string" type=string
/// @type.symbol symbol=keep.value source="value: T" type=T
/// @resolution.name source=T target=keep.T
/// @resolution.name source=T target=keep.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=keep.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=keep.value

}

const value = keep({ name: "Ada", extra: true });
/// @type.symbol symbol=value source=value type={ name: string; extra: boolean }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="keep({ name: \"Ada\", extra: true })" type={ name: string; extra: boolean }
/// @type.node source=keep type=({ name: string; extra: boolean }) => { name: string; extra: boolean }
/// @resolution.name source=keep target=keep
/// @resolution.call source="keep({ name: \"Ada\", extra: true })" parameters=({ name: string; extra: boolean }) arguments=(provided({ name: "Ada", extra: true }) as { name: string; extra: boolean }) return={ name: string; extra: boolean } kind=symbol target=keep instance="keep<{ name: string; extra: boolean }>"
/// @generic.instantiation id="keep<{ name: string; extra: boolean }>" template=keep arguments=({ name: string; extra: boolean })
/// @generic.instance id="keep<{ name: string; extra: boolean }>" template=keep arguments=({ name: string; extra: boolean })
/// @type.node source={ name: "Ada", extra: true } type={ name: string; extra: boolean }
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source=true type=true

const extra = value.extra;
/// @type.symbol symbol=extra source=extra type=boolean
/// @resolution.pattern source=extra kind=binding target=extra
/// @type.node source=value type={ name: string; extra: boolean }
/// @type.node source=value.extra type=boolean
/// @resolution.name source=value target=value
/// @resolution.member source=value.extra receiver={ name: string; extra: boolean } type=boolean kind=field target_receiver={ name: string; extra: boolean } key=extra target_type=boolean
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.access source=value.extra root=value keys=[extra]
"#,
    );
}
