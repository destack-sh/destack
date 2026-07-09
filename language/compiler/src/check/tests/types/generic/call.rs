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

const number: 1 = identity<1>(1);
const text: "x" = identity<"x">("x");

=== checked ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value

}

const number = identity(1);
/// @type.symbol symbol=number source=number type=1
/// @type.node source=identity type=(1) => 1
/// @type.node source=identity(1) type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(1) parameters=(1) arguments=(provided(1) as 1) return=1 kind=symbol target=identity instance=identity<1>
/// @generic.instance source=identity(1) id=identity<1>
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text source=text type="x"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=("x") => "x"
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(\"x\")" parameters=("x") arguments=(provided("x") as "x") return="x" kind=symbol target=identity instance="identity<\"x\">"
/// @generic.instance source="identity(\"x\")" id="identity<\"x\">"
/// @type.node source="\"x\"" type="x"

/// @generic.instance id="identity<\"x\">" template=identity arguments=("x")
/// @generic.instance id=identity<1> template=identity arguments=(1)

/// @check.stats.solve variables=2 types=10 constraints=0 obligations=0 solutions=2 bounds=2 decisions=7
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
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value

}

const values = identity([1, 2]);
/// @type.symbol symbol=values source=values type=Array<float64>
/// @type.node source="identity([1, 2])" type=Array<float64>
/// @type.node source=identity type=(Array<float64>) => Array<float64>
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity([1, 2])" parameters=(Array<float64>) arguments=(provided([1, 2]) as Array<float64>) return=Array<float64> kind=symbol target=identity instance=identity<Array<float64>>
/// @generic.instance source="identity([1, 2])" id=identity<Array<float64>>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @generic.instance id=identity<Array<float64>> template=identity arguments=(Array<float64>)
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
/// @type.symbol symbol=first.values source="values: T[]" type=Array<T>
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

    return values[0];
    /// @type.node source=values type=Array<T>
    /// @type.node source=values[0] type=T
    /// @resolution.name source=values target=first.values
    /// @resolution.call source=values[0] parameters=(usize) arguments=(provided(0) as usize) return=T kind=symbol target=collections.array.index#4 receiver=Array<T> instance=Array<T>.<extension#6>.index#4
    /// @generic.instance source=values[0] id=Array<T>.<extension#6>.index#4
    /// @type.node source=0 type=0

}

const value = first([1, 2]);
/// @type.symbol symbol=value source=value type=float64
/// @type.node source="first([1, 2])" type=float64
/// @type.node source=first type=(Array<float64>) => float64
/// @resolution.name source=first target=first
/// @resolution.call source="first([1, 2])" parameters=(Array<float64>) arguments=(provided([1, 2]) as Array<float64>) return=float64 kind=symbol target=first instance=first<float64>
/// @generic.instance source="first([1, 2])" id=first<float64>
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @generic.instance id=Array<T>.<extension#6>.index#4 template=collections.array.index#4 arguments=(T, T)
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
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value

}

const first = identity<1>(1);
/// @type.symbol symbol=first source=first type=1
/// @type.node source=identity type=(1) => 1
/// @type.node source=identity<1>(1) type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity<1>(1) parameters=(1) arguments=(provided(1) as 1) return=1 kind=symbol target=identity instance=identity<1>
/// @generic.instance source=identity<1>(1) id=identity<1>
/// @type.node source=1 type=1

const second = identity<2>(2);
/// @type.symbol symbol=second source=second type=2
/// @type.node source=identity type=(2) => 2
/// @type.node source=identity<2>(2) type=2
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity<2>(2) parameters=(2) arguments=(provided(2) as 2) return=2 kind=symbol target=identity instance=identity<2>
/// @generic.instance source=identity<2>(2) id=identity<2>
/// @type.node source=2 type=2

/// @generic.instance id=identity<1> template=identity arguments=(1)
/// @generic.instance id=identity<2> template=identity arguments=(2)
"#,
    );
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
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value

}

const text = identity<string>("x");
/// @type.symbol symbol=text source=text type=string
/// @type.node source="identity<string>(\"x\")" type=string
/// @type.node source=identity type=(string) => string
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity<string>(\"x\")" parameters=(string) arguments=(provided("x") as string) return=string kind=symbol target=identity instance=identity<string>
/// @generic.instance source="identity<string>(\"x\")" id=identity<string>
/// @type.node source="\"x\"" type="x"

/// @generic.instance id=identity<string> template=identity arguments=(string)
"#,
    );
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
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value

}

identity<int32>("x");
/// @type.node source="identity<int32>(\"x\")" type=int32
/// @type.node source=identity type=(int32) => int32
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity<int32>(\"x\")" parameters=(int32) arguments=(provided("x") as int32) return=int32 kind=symbol target=identity instance=identity<int32>
/// @generic.instance source="identity<int32>(\"x\")" id=identity<int32>
/// @type.node source="\"x\"" type="x"

/// @generic.instance id=identity<int32> template=identity arguments=(int32)
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type '\"x\"' is not assignable to parameter of type 'int32'"
/// @diagnostic.label line=6 column=17 span="\"x\"" line_source="identity<int32>(\"x\");"
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

const asInt = identity<int32>;

=== checked ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value

}

const asInt = identity<int32>;
/// @type.symbol symbol=asInt source=asInt type=<T>(int32) => int32
/// @type.node source=identity<int32> type=<T>(int32) => int32
/// @resolution.name source=identity target=identity
/// @resolution.instantiation source=identity<int32> target=identity instance=identity<int32>
/// @generic.instance source=identity<int32> id=identity<int32>

/// @generic.instance id=identity<int32> template=identity arguments=(int32)
"#,
    );
}

#[test]
fn test_explicit_function_type_argument_rejects_overload_set() {
    let session = TestSession::single(
        r#"
function parse<T>(value: T): T {
    return value;
}

function parse<T>(value: T[]): T {
    return value[0];
}

const parser = parse<int32>;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse<T>(value: T): T {
    return value;
}

function parse<T>(value: T[]): T {
    return value[0];
}

const parser = parse<int32>;

=== checked ===
function parse<T>(value: T): T {
/// @generic.template symbol=parse#1 parameters=(T#1)
/// @type.symbol symbol=parse#1 type=<T#1>(T#1) => T#1
/// @type.symbol symbol=parse.T#1 source=T type=T#1
/// @type.symbol symbol=parse.value#1 source="value: T" type=T#1
/// @resolution.name source=T target=parse.T#1
/// @resolution.name source=T target=parse.T#1

    return value;
    /// @type.node source=value type=T#1
    /// @resolution.name source=value target=parse.value#1

}

function parse<T>(value: T[]): T {
/// @generic.template symbol=parse#2 parameters=(T#2)
/// @type.symbol symbol=parse#2 type=<T#2>(Array<T#2>) => T#2
/// @type.symbol symbol=parse.T#2 source=T type=T#2
/// @type.symbol symbol=parse.value#2 source="value: T[]" type=Array<T#2>
/// @resolution.name source=T target=parse.T#2
/// @resolution.name source=T target=parse.T#2

    return value[0];
    /// @type.node source=value type=Array<T#2>
    /// @type.node source=value[0] type=T#2
    /// @resolution.name source=value target=parse.value#2
    /// @resolution.call source=value[0] parameters=(usize) arguments=(provided(0) as usize) return=T#2 kind=symbol target=collections.array.index#4 receiver=Array<T#2> instance=Array<T#2>.<extension#6>.index#4
    /// @generic.instance source=value[0] id=Array<T#2>.<extension#6>.index#4
    /// @type.node source=0 type=0

}

const parser = parse<int32>;
/// @type.symbol symbol=parser source=parser type=<error>
/// @type.node source=parse<int32> type=<error>
/// @resolution.name source=parse target=[parse#1, parse#2]

/// @generic.instance id=Array<T#2>.<extension#6>.index#4 template=collections.array.index#4 arguments=(T#2, T#2)
"#,
        r#"
/// @diagnostic.error code=EC309 message="ambiguous reference 'parse'"
/// @diagnostic.label line=10 column=16 span="parse" line_source="const parser = parse<int32>;"
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

const defaulted: (1, 1) = pair<1, 1>(1);
const overridden: (1, "x") = pair<1, "x">(1, "x" as "x" | undefined);

=== checked ===
declare function pair<T, U = T>(left: T, right?: U): (T, U);
/// @generic.template symbol=pair parameters=(T, U = T)
/// @type.symbol symbol=pair source="declare function pair<T, U = T>(left: T, right?: U): (T, U)" type=<T, U = T>(T, U | undefined) => (T, U)
/// @type.symbol symbol=pair.T source=T type=T
/// @type.symbol symbol=pair.U source="U = T" type=U
/// @resolution.name source=T target=pair.T
/// @type.symbol symbol=pair.left source="left: T" type=T
/// @resolution.name source=T target=pair.T
/// @type.symbol symbol=pair.right source="right?: U" type=U | undefined
/// @resolution.name source=U target=pair.U
/// @resolution.name source=T target=pair.T
/// @resolution.name source=U target=pair.U

const defaulted = pair(1);
/// @type.symbol symbol=defaulted source=defaulted type=(1, 1)
/// @type.node source=pair type=(1, 1 | undefined) => (1, 1)
/// @type.node source=pair(1) type=(1, 1)
/// @resolution.name source=pair target=pair
/// @resolution.call source=pair(1) parameters=(1, 1 | undefined) arguments=(provided(1) as 1, omitted as 1 | undefined) return=(1, 1) kind=symbol target=pair instance="pair<1, 1>"
/// @generic.instance source=pair(1) id="pair<1, 1>"
/// @type.node source=1 type=1

const overridden = pair(1, "x");
/// @type.symbol symbol=overridden source=overridden type=(1, "x")
/// @type.node source="pair(1, \"x\")" type=(1, "x")
/// @type.node source=pair type=(1, "x" | undefined) => (1, "x")
/// @resolution.name source=pair target=pair
/// @resolution.call source="pair(1, \"x\")" parameters=(1, "x" | undefined) arguments=(provided(1) as 1, provided("x") as "x" | undefined) return=(1, "x") kind=symbol target=pair instance="pair<1, \"x\">"
/// @generic.instance source="pair(1, \"x\")" id="pair<1, \"x\">"
/// @type.node source=1 type=1
/// @type.node source="\"x\"" type="x"

/// @generic.instance id="pair<1, 1>" template=pair arguments=(1, 1)
/// @generic.instance id="pair<1, \"x\">" template=pair arguments=(1, "x")
"#,
    );
}
