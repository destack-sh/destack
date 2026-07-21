use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_default_binds_missing_field() {
    let session = TestSession::single(
        r#"
let { name = "Ada" } = {};

name satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let { name = "Ada" } = {};

name satisfies string;

=== checked ===
let { name = "Ada" } = {};
/// @resolution.pattern source={ name = "Ada" } kind=object fields={ name }
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.pattern source=name kind=default pattern=pattern value=expression
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source={} type={}

name satisfies string;
/// @type.node source="name satisfies string" type=string
/// @type.node source=name type=string
/// @resolution.name source=name target=name
"#,
    );
}

#[test]
fn test_assignment_pattern_default_resolves_fallback() {
    let session = TestSession::single(
        r#"
let value: int32 = 0;
declare const fallback: int32;
declare const values: [int32; 1];

[value = fallback] = values;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 0;
declare const fallback: int32;
declare const values: [int32; 1];

[value = fallback] = values;

=== checked ===
let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0

declare const fallback: int32;
/// @type.symbol symbol=fallback source=fallback type=int32
/// @resolution.pattern source=fallback kind=binding target=fallback

declare const values: [int32; 1];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 1>
/// @resolution.pattern source=values kind=binding target=values

[value = fallback] = values;
/// @type.node source="[value = fallback] = values" type=FixedArray<int32, 1>
/// @resolution.pattern.assign source=[value = fallback] kind=sequence element=int32 arity=1 fields=(value)
/// @type.node source=value type=int32
/// @resolution.pattern.assign source="value = fallback" kind=default pattern=value value=expression
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source=fallback type=int32
/// @resolution.name source=fallback target=fallback
/// @type.node source=values type=FixedArray<int32, 1>
/// @resolution.name source=values target=values
"#,
    );
}
