use crate::tests::{DirRows, TestSession};

#[test]
fn test_call_infers_type_argument_from_parameter() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const number = identity(1);
const text = identity("x");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_check_stats(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const number: float64 = identity<float64>(1);
const text: string = identity<string>("x");

=== checked ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

const number = identity(1);
/// @type.symbol symbol=number source=number type=float64
/// @type.node source=identity type=(float64) => float64
/// @type.node source=identity(1) type=float64
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(1) parameters=(float64) return=float64 kind=symbol target=identity instance=identity<float64>
/// @generic.instance source=identity(1) id=identity<float64>
/// @type.node source=1 type=float64

const text = identity("x");
/// @type.symbol symbol=text source=text type=string
/// @type.node source="identity(\"x\")" type=string
/// @type.node source=identity type=(string) => string
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(\"x\")" parameters=(string) return=string kind=symbol target=identity instance=identity<string>
/// @generic.instance source="identity(\"x\")" id=identity<string>
/// @type.node source="\"x\"" type=string

/// @generic.instance id=identity<float64> template=identity arguments=(float64)
/// @generic.instance id=identity<string> template=identity arguments=(string)
/// @check.stats.solve variables=6 types=18 constraints=6 obligations=0 solutions=6 bounds=6 decisions=7
"#);
}

#[test]
fn test_array_literal_argument_widens_generic_container() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const values = identity([1, 2]);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const values: float64[] = identity<float64[]>([1, 2]);

=== checked ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

const values = identity([1, 2]);
/// @type.symbol symbol=values source=values type=Array<float64>
/// @type.node source="identity([1, 2])" type=Array<float64>
/// @type.node source=identity type=(Array<float64>) => Array<float64>
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity([1, 2])" parameters=(Array<float64>) return=Array<float64> kind=symbol target=identity instance="identity<Array<float64>>"
/// @generic.instance source="identity([1, 2])" id="identity<Array<float64>>"
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @generic.instance id="identity<Array<float64>>" template=identity arguments=(Array<float64>)
"#,
    );
}

#[test]
fn test_array_parameter_infers_widened_element_type() {
    let session = TestSession::single(
        r#"
function first<T>(values: T[]): T {
    return values[0];
}

const value = first([1, 2]);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function first<T>(values: T[]): T {
    return values[0];
}

const value: float64 = first<float64>([1, 2]);

=== checked ===
function first<T>(values: T[]): T {
/// @generic.template symbol=first parameters=(T)
/// @type.symbol symbol=first type=<T>(Array<T>) => T
/// @type.symbol symbol=first.T source=T type=T
/// @type.symbol symbol=values source="values: T[]" type=Array<T>
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

    return values[0];
    /// @type.node source=values type=Array<T>
    /// @type.node source=values[0] type=T
    /// @resolution.name source=values target=values
    /// @resolution.call source=values[0] parameters=(usize) return=T kind=symbol target=collections.array.index#8 receiver=Array<T>
    /// @type.node source=0 type=usize

}

const value = first([1, 2]);
/// @type.symbol symbol=value source=value type=float64
/// @type.node source="first([1, 2])" type=float64
/// @type.node source=first type=(Array<float64>) => float64
/// @resolution.name source=first target=first
/// @resolution.call source="first([1, 2])" parameters=(Array<float64>) return=float64 kind=symbol target=first instance=first<float64>
/// @generic.instance source="first([1, 2])" id=first<float64>
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @generic.instance id=first<float64> template=first arguments=(float64)
"#,
    );
}

#[test]
fn test_explicit_literal_type_arguments_create_distinct_generic_instances() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const first = identity<1>(1);
const second = identity<2>(2);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const first: 1 = identity<1>(1);
const second: 2 = identity<2>(2);

=== checked ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

const first = identity<1>(1);
/// @type.symbol symbol=first source=first type=1
/// @type.node source=identity type=(1) => 1
/// @type.node source=identity<1>(1) type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity<1>(1) parameters=(1) return=1 kind=symbol target=identity instance=identity<1>
/// @generic.instance source=identity<1>(1) id=identity<1>
/// @type.node source=1 type=1

const second = identity<2>(2);
/// @type.symbol symbol=second source=second type=2
/// @type.node source=identity type=(2) => 2
/// @type.node source=identity<2>(2) type=2
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity<2>(2) parameters=(2) return=2 kind=symbol target=identity instance=identity<2>
/// @generic.instance source=identity<2>(2) id=identity<2>
/// @type.node source=2 type=2
/// @generic.instance id=identity<1> template=identity arguments=(1)
/// @generic.instance id=identity<2> template=identity arguments=(2)
"#);
}

#[test]
fn test_explicit_call_type_argument_selects_instance() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const text = identity<string>("x");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const text: string = identity<string>("x");

=== checked ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

const text = identity<string>("x");
/// @type.symbol symbol=text source=text type=string
/// @type.node source="identity<string>(\"x\")" type=string
/// @type.node source=identity type=(string) => string
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity<string>(\"x\")" parameters=(string) return=string kind=symbol target=identity instance=identity<string>
/// @generic.instance source="identity<string>(\"x\")" id=identity<string>
/// @type.node source="\"x\"" type=string
/// @generic.instance id=identity<string> template=identity arguments=(string)
"#);
}

#[test]
fn test_explicit_call_type_argument_mismatch_reports_assignability_error() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

identity<int32>("x");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

identity<int32>("x");

=== checked ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

identity<int32>("x");
/// @type.node source="identity<int32>(\"x\")" type=<error>
/// @type.node source=identity type=(int32) => int32
/// @resolution.name source=identity target=identity
/// @type.node source="\"x\"" type="x"

"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type '\"x\"' is not assignable to parameter of type 'int32'"
/// @diagnostic.label line=6 column=17 source="identity<int32>(\"x\");"
"#,
    );
}

#[test]
fn test_explicit_function_type_argument_specializes_reference() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const asInt = identity<int32>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const asInt: (p0: int32) => int32 = identity<int32>;

=== checked ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=value

}

const asInt = identity<int32>;
/// @type.symbol symbol=asInt source=asInt type=(int32) => int32
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity<int32> type=(int32) => int32
/// @generic.instance source=identity<int32> id=identity<int32>
/// @resolution.name source=identity target=identity
/// @resolution.name source=identity<int32> target=identity
/// @generic.instance id=identity<int32> template=identity arguments=(int32)
"#,
    );
}

#[test]
fn test_defaulted_type_argument_uses_prior_type_parameter() {
    let session = TestSession::single(
        r#"
declare function pair<T, U = T>(left: T, right?: U): (T, U);

const defaulted = pair(1);
const overridden = pair(1, "x");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function pair<T, U = T>(left: T, right?: U): (T, U);

const defaulted: (float64, float64) = pair<float64, float64>(1);
const overridden: (float64, string) = pair<float64, string>(1, "x" as string | undefined);

=== checked ===
declare function pair<T, U = T>(left: T, right?: U): (T, U);
/// @generic.template symbol=pair parameters=(T, U = T)
/// @type.symbol symbol=pair source="declare function pair<T, U = T>(left: T, right?: U): (T, U)" type=<T, U = T>(T, U | undefined?) => (T, U)
/// @type.symbol symbol=pair.T source=T type=T
/// @type.symbol symbol=pair.U source="U = T" type=U
/// @resolution.name source=T target=pair.T
/// @type.symbol symbol=left source="left: T" type=T
/// @resolution.name source=T target=pair.T
/// @type.symbol symbol=right source="right?: U" type=U | undefined
/// @resolution.name source=U target=pair.U
/// @resolution.name source=T target=pair.T
/// @resolution.name source=U target=pair.U

const defaulted = pair(1);
/// @type.symbol symbol=defaulted source=defaulted type=(float64, float64)
/// @type.node source=pair type=(float64, float64 | undefined?) => (float64, float64)
/// @type.node source=pair(1) type=(float64, float64)
/// @resolution.name source=pair target=pair
/// @resolution.call source=pair(1) parameters=(float64) return=(float64, float64) kind=symbol target=pair instance="pair<float64, float64>"
/// @generic.instance source=pair(1) id="pair<float64, float64>"
/// @type.node source=1 type=float64

const overridden = pair(1, "x");
/// @type.symbol symbol=overridden source=overridden type=(float64, string)
/// @type.node source="pair(1, \"x\")" type=(float64, string)
/// @type.node source=pair type=(float64, string | undefined?) => (float64, string)
/// @resolution.name source=pair target=pair
/// @resolution.call source="pair(1, \"x\")" parameters=(float64, string | undefined) return=(float64, string) kind=symbol target=pair instance="pair<float64, string>"
/// @generic.instance source="pair(1, \"x\")" id="pair<float64, string>"
/// @type.node source=1 type=float64
/// @type.node source="\"x\"" type=string
/// @generic.instance id="pair<float64, float64>" template=pair arguments=(float64, float64)
/// @generic.instance id="pair<float64, string>" template=pair arguments=(float64, string)
"#);
}
