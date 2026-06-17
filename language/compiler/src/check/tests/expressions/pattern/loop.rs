use crate::tests::{DirRows, TestSession};

#[test]
fn test_for_of_destructures_object_elements() {
    let session = TestSession::single(
        r#"
declare const items: { name: string; value: int32 }[];

for (const { name, value } of items) {
    name satisfies string;
    value satisfies int32;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const items: { name: string; value: int32 }[];

for (const { name, value } of items) {
    name satisfies string;
    value satisfies int32;
}

=== checked ===
declare const items: { name: string; value: int32 }[];
/// @type.symbol symbol=items source=items type=Array<{ name: string; value: int32 }>

for (const { name, value } of items) {
/// @type.node type=void
/// @type.symbol symbol=name source=name type=string
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source="{ name, value }" kind=object fields={ name, value }
/// @type.node source=items type=Array<{ name: string; value: int32 }>
/// @resolution.name source=items target=items

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value

}
"#,
    );
}

#[test]
fn test_for_in_rejects_destructuring_pattern() {
    let session = TestSession::single(
        r#"
declare const item: { name: string };

for (const { name } in item) {
    name;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const item: { name: string };

for (const { name } in item) {
    name;
}

=== checked ===
declare const item: { name: string };
/// @type.symbol symbol=item source=item type={ name: string }

for (const { name } in item) {
/// @type.node type=void
/// @type.symbol symbol=name source=name type=<error>
/// @resolution.pattern source="{ name }" kind=object fields={ name }
/// @type.node source=item type={ name: string }
/// @resolution.name source=item target=item

    name;
    /// @type.node source=name type=<error>
    /// @resolution.name source=name target=name

}
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'string' is not assignable to type '{ name: unknown }'"
/// @diagnostic.label line=4 column=12 source="{ name }"
"#,
    );
}
