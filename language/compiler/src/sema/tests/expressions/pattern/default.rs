use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_default_binds_missing_field() {
    let session = TestSession::single(
        r#"
let { name = "Ada" } = {};

name satisfies string;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let { name = "Ada" } = {};

name satisfies string;

=== dir ===
let { name = "Ada" } = {};
/// @resolution.pattern source={ name = "Ada" } kind=object fields={ absent(undefined): name }
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.pattern source=name kind=default pattern=pattern value=expression
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source={} type={}

name satisfies string;
/// @type.node source="name satisfies string" type=string
/// @type.node source=name type=string
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=name root=name
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 0;
declare const fallback: int32;
declare const values: [int32; 1];

[value = fallback] = values;

=== dir ===
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
/// @resolution.access source=[value = fallback] root=values
/// @generic.instantiation id="index#2<int32, 1, \"frame\" & \"local\">" template=index#2 arguments=(int32, 1, "frame" & "local")
/// @generic.instance id="FixedArray<int32, 1>" template=FixedArray arguments=(int32, 1)
/// @generic.instance id="index#2<int32, 1, \"bound0\" & \"local\">" template=index#2 arguments=(int32, 1, "bound0" & "local")
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source="value = fallback" kind=default pattern=value value=expression
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int32
/// @type.node source=fallback type=int32
/// @resolution.name source=fallback target=fallback
/// @resolution.place source=fallback placement="local" lifetime="static" access="immutable"
/// @resolution.access source=fallback root=fallback
/// @type.node source=values type=FixedArray<int32, 1>
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
"#,
    );
}
