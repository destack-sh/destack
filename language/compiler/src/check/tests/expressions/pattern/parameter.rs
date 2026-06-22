use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_parameter_binds_fields() {
    let session = TestSession::single(
        r#"
function label({ name, age }: { name: string; age: int32 }): string {
    name satisfies string;
    age satisfies int32;

    name
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function label({ name, age }: { name: string; age: int32 }): string {
    name satisfies string;
    age satisfies int32;

    name
}

=== checked ===
function label({ name, age }: { name: string; age: int32 }): string {
/// @type.symbol symbol=label type=({ name: string; age: int32 }) => string
/// @type.symbol symbol=name source=name type=string
/// @type.symbol symbol=age source=age type=int32
/// @resolution.pattern source="{ name, age }" kind=object fields={ name, age }

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name

    age satisfies int32;
    /// @type.node source="age satisfies int32" type=int32
    /// @type.node source=age type=int32
    /// @resolution.name source=age target=age

    name
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name

}
"#,
    );
}

#[test]
fn test_sequence_pattern_parameter_binds_elements() {
    let session = TestSession::single(
        r#"
function first([head]: int32[]): int32 {
    head
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function first([head]: int32[]): int32 {
    head
}

=== checked ===
function first([head]: int32[]): int32 {
/// @type.symbol symbol=first type=(Array<int32>) => int32
/// @type.symbol symbol=head source=head type=int32
/// @resolution.pattern source=[head] kind=sequence sequence=array fields=(head)
/// @resolution.pattern source=head kind=binding target=head

    head
    /// @type.node source=head type=int32
    /// @resolution.name source=head target=head

}
"#,
    );
}
