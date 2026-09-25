use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_bigint_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = 42n;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 42n = 42n;

=== dir ===
const value = 42n;
/// @type.symbol symbol=value source=value type=42n
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=42n type=42n
"#,
    );
}

#[test]
fn test_let_bigint_widens_binding_type() {
    let session = TestSession::single(
        r#"
let value = 42n;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: bigint = 42n;

=== dir ===
let value = 42n;
/// @type.symbol symbol=value source=value type=bigint
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=42n type=42n
"#,
    );
}

#[test]
fn test_bigint_annotation_sets_binding_type() {
    let session = TestSession::single(
        r#"
const value: bigint = 42n;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: bigint = 42n;

=== dir ===
const value: bigint = 42n;
/// @type.symbol symbol=value source=value type=bigint
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=42n type=42n
"#,
    );
}

#[test]
fn test_bigint_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = 42n;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: float64 = 42n;

=== dir ===
const value: number = 42n;
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=42n type=42n
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '42n' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 span="42n" line_source="const value: number = 42n;"
/// @diagnostic.related line=2 column=14 span="number" line_source="const value: number = 42n;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_bigint_literal_flows_into_bigint_union() {
    let session = TestSession::single(
        r#"
const value: bigint | string = 42n;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: bigint | string = 42n as bigint | string;

=== dir ===
const value: bigint | string = 42n;
/// @type.symbol symbol=value source=value type=bigint | string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=42n type=42n
"#,
    );
}

#[test]
fn test_bigint_literal_resolves_extension_member() {
    let session = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const isZero = (1n).isZero;

isZero satisfies boolean;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const isZero: boolean = 1n.isZero;

isZero satisfies boolean;

=== dir ===
const isZero = (1n).isZero;
/// @type.symbol symbol=isZero source=isZero type=boolean
/// @resolution.pattern source=isZero kind=binding target=isZero
/// @type.node source=(1n).isZero type=boolean
/// @resolution.member source=(1n).isZero receiver=1n type=boolean kind=call target="isZero(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="isZero<\"managed\" & \"local\">" template=isZero arguments=("managed" & "local")
/// @generic.instance id="isZero<\"bound0\" & \"local\">" template=isZero arguments=("bound0" & "local")
/// @type.node source=1n type=1n

isZero satisfies boolean;
/// @type.node source="isZero satisfies boolean" type=boolean
/// @type.node source=isZero type=boolean
/// @resolution.name source=isZero target=isZero
/// @resolution.place source=isZero placement="local" lifetime="static" access="immutable"
/// @resolution.access source=isZero root=isZero
"#,
    );
}

#[test]
fn test_bigint_alias_resolves_extension_member() {
    let session = TestSession::builder()
        .module(
            "main.tspp",
            r#"
let value: bigint = 1n;
const isZero = value.isZero;

isZero satisfies boolean;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: bigint = 1n;
const isZero: boolean = value.isZero;

isZero satisfies boolean;

=== dir ===
let value: bigint = 1n;
/// @type.symbol symbol=value source=value type=bigint
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1n type=1n

const isZero = value.isZero;
/// @type.symbol symbol=isZero source=isZero type=boolean
/// @resolution.pattern source=isZero kind=binding target=isZero
/// @type.node source=value type=bigint
/// @type.node source=value.isZero type=boolean
/// @resolution.name source=value target=value
/// @resolution.member source=value.isZero receiver=bigint type=boolean kind=call target="isZero(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @generic.instantiation id="isZero<\"managed\" & \"local\">" template=isZero arguments=("managed" & "local")
/// @generic.instance id="isZero<\"bound0\" & \"local\">" template=isZero arguments=("bound0" & "local")

isZero satisfies boolean;
/// @type.node source="isZero satisfies boolean" type=boolean
/// @type.node source=isZero type=boolean
/// @resolution.name source=isZero target=isZero
/// @resolution.place source=isZero placement="local" lifetime="static" access="immutable"
/// @resolution.access source=isZero root=isZero
"#,
    );
}
