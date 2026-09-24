use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_string_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = "hello";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: "hello" = "hello";

=== dir ===
const value = "hello";
/// @type.symbol symbol=value source=value type="hello"
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"hello\"" type="hello"
"#,
    );
}

#[test]
fn test_let_string_widens_binding_type() {
    let session = TestSession::single(
        r#"
let value = "hello";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: string = "hello";

=== dir ===
let value = "hello";
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"hello\"" type="hello"
"#,
    );
}

#[test]
fn test_string_annotation_sets_binding_type() {
    let session = TestSession::single(
        r#"
const value: string = "hello";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: string = "hello";

=== dir ===
const value: string = "hello";
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"hello\"" type="hello"
"#,
    );
}

#[test]
fn test_empty_string_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = "";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: "" = "";

=== dir ===
const value = "";
/// @type.symbol symbol=value source=value type=""
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"\"" type=""
"#,
    );
}

#[test]
fn test_string_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = "hello";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: float64 = "hello";

=== dir ===
const value: number = "hello";
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"hello\"" type="hello"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"hello\"' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 span="\"hello\"" line_source="const value: number = \"hello\";"
/// @diagnostic.related line=2 column=14 span="number" line_source="const value: number = \"hello\";" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_string_literal_resolves_class_member() {
    let session = TestSession::builder()
        .module(
            "main.ds",
            r#"
const isEmpty = "".isEmpty;

isEmpty satisfies boolean;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const isEmpty: boolean = "".isEmpty;

isEmpty satisfies boolean;

=== dir ===
const isEmpty = "".isEmpty;
/// @type.symbol symbol=isEmpty source=isEmpty type=boolean
/// @resolution.pattern source=isEmpty kind=binding target=isEmpty
/// @type.node source="\"\"" type=""
/// @type.node source="\"\".isEmpty" type=boolean
/// @resolution.member source="\"\".isEmpty" receiver="" type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="isEmpty<\"managed\" & \"local\">" template=isEmpty arguments=("managed" & "local")
/// @generic.instance id="isEmpty<\"bound0\" & \"local\">" template=isEmpty arguments=("bound0" & "local")

isEmpty satisfies boolean;
/// @type.node source="isEmpty satisfies boolean" type=boolean
/// @type.node source=isEmpty type=boolean
/// @resolution.name source=isEmpty target=isEmpty
/// @resolution.place source=isEmpty placement="local" lifetime="static" access="immutable"
/// @resolution.access source=isEmpty root=isEmpty
"#,
    );
}

#[test]
fn test_string_alias_resolves_class_member() {
    let session = TestSession::builder()
        .module(
            "main.ds",
            r#"
let value: string = "";
const isEmpty = value.isEmpty;

isEmpty satisfies boolean;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: string = "";
const isEmpty: boolean = value.isEmpty;

isEmpty satisfies boolean;

=== dir ===
let value: string = "";
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"\"" type=""

const isEmpty = value.isEmpty;
/// @type.symbol symbol=isEmpty source=isEmpty type=boolean
/// @resolution.pattern source=isEmpty kind=binding target=isEmpty
/// @type.node source=value type=string
/// @type.node source=value.isEmpty type=boolean
/// @resolution.name source=value target=value
/// @resolution.member source=value.isEmpty receiver=string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @generic.instantiation id="isEmpty<\"managed\" & \"local\">" template=isEmpty arguments=("managed" & "local")
/// @generic.instance id="isEmpty<\"bound0\" & \"local\">" template=isEmpty arguments=("bound0" & "local")

isEmpty satisfies boolean;
/// @type.node source="isEmpty satisfies boolean" type=boolean
/// @type.node source=isEmpty type=boolean
/// @resolution.name source=isEmpty target=isEmpty
/// @resolution.place source=isEmpty placement="local" lifetime="static" access="immutable"
/// @resolution.access source=isEmpty root=isEmpty
"#,
    );
}
