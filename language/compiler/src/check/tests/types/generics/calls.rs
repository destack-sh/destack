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
        DirRows::checked().with_reference_types(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const number = identity(1);
/// @type.symbol symbol=number type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(1) parameters=(1) return=1 kind=symbol target=identity instance=identity<1>
/// @generic.application source=identity(1) id=identity<1>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity(1) type=1
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text type="x"
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(\"x\")" parameters=("x") return="x" kind=symbol target=identity instance="identity<\"x\">"
/// @generic.application source="identity(\"x\")" id="identity<\"x\">"
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=<T>(T) => T
/// @type.node source="\"x\"" type="x"
/// @generic.instance id="identity<\"x\">" symbol=identity arguments=["x"]
/// @generic.instance id=identity<1> symbol=identity arguments=[1]
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
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const values = identity([1, 2]);
/// @type.symbol symbol=values type=Array<int32>
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity([1, 2])" parameters=(Array<int32>) return=Array<int32> kind=symbol target=identity instance="identity<Array<int32>>"
/// @generic.application source="identity([1, 2])" id="identity<Array<int32>>"
/// @type.node source="identity([1, 2])" type=Array<int32>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @generic.instance id="identity<Array<int32>>" symbol=identity arguments=[Array<int32>]
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
function first<T>(values: T[]): T {
/// @generic.slot symbol=first.T index=0 kind=type
/// @type.symbol symbol=first type=<T>(Array<T>) => T
/// @type.symbol symbol=values type=Array<T>
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T

    return values[0];
    /// @type.node source=values type=Array<T>
    /// @type.node source=values[0] type=T
    /// @resolution.name source=values target=values
    /// @resolution.member source=values[0] receiver=Array<T> kind=builtin builtin=subscript.index
    /// @type.node source=0 type=0

}

const value = first([1, 2]);
/// @type.symbol symbol=value type=int32
/// @generic.application source="first([1, 2])" id=first<int32>
/// @type.node source="first([1, 2])" type=int32
/// @type.node source=first type=<T>(Array<T>) => T
/// @resolution.name source=first target=first
/// @resolution.call source="first([1, 2])" parameters=(Array<int32>) return=int32 kind=symbol target=first instance=first<int32>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @generic.instance id=first<int32> symbol=first arguments=[int32]
"#,
    );
}

#[test]
fn test_literal_arguments_create_distinct_generic_instances() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const first = identity(1);
const second = identity(2);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const first = identity(1);
/// @type.symbol symbol=first type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(1) parameters=(1) return=1 kind=symbol target=identity instance=identity<1>
/// @generic.application source=identity(1) id=identity<1>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity(1) type=1
/// @type.node source=1 type=1

const second = identity(2);
/// @type.symbol symbol=second type=2
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(2) parameters=(2) return=2 kind=symbol target=identity instance=identity<2>
/// @generic.application source=identity(2) id=identity<2>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity(2) type=2
/// @type.node source=2 type=2
/// @generic.instance id=identity<1> symbol=identity arguments=[1]
/// @generic.instance id=identity<2> symbol=identity arguments=[2]
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
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const text = identity<string>("x");
/// @type.symbol symbol=text type=string
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity<string>(\"x\")" parameters=(string) return=string kind=symbol target=identity instance=identity<string>
/// @generic.application source="identity<string>(\"x\")" id=identity<string>
/// @type.node source="identity<string>(\"x\")" type=string
/// @type.node source=identity type=<T>(T) => T
/// @type.node source="\"x\"" type=string
/// @generic.instance id=identity<string> symbol=identity arguments=[string]
"#);
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
function identity<T>(value: T): T {
/// @generic.slot symbol=identity.T index=0 kind=type
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=value type=T

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=T

}

const asInt = identity<int32>;
/// @type.symbol symbol=asInt type=(int32) => int32
/// @resolution.name source=identity target=identity
/// @generic.application source=identity<int32> id=identity<int32>
/// @type.node source=identity type=<T>(T) => T
/// @type.node source=identity<int32> type=(int32) => int32
/// @generic.instance id=identity<int32> symbol=identity arguments=[int32]
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
declare function pair<T, U = T>(left: T, right?: U): (T, U);
/// @generic.slot symbol=pair.T index=0 kind=type
/// @generic.slot symbol=pair.U index=1 kind=type default=T
/// @type.symbol symbol=pair type=<T, U = T>(T, U?) => (T, U)
/// @type.symbol symbol=left type=T
/// @type.symbol symbol=right type=U

const defaulted = pair(1);
/// @type.symbol symbol=defaulted type=(1, 1)
/// @resolution.name source=pair target=pair
/// @resolution.call source=pair(1) parameters=(1, 1) return=(1, 1) kind=symbol target=pair instance="pair<1, 1>"
/// @generic.application source=pair(1) id="pair<1, 1>"
/// @type.node source=pair type=<T, U = T>(T, U?) => (T, U)
/// @type.node source=pair(1) type=(1, 1)
/// @type.node source=1 type=1

const overridden = pair(1, "x");
/// @type.symbol symbol=overridden type=(1, "x")
/// @resolution.name source=pair target=pair
/// @resolution.call source="pair(1, \"x\")" parameters=(1, "x") return=(1, "x") kind=symbol target=pair instance="pair<1, \"x\">"
/// @generic.application source="pair(1, \"x\")" id="pair<1, \"x\">"
/// @type.node source="pair(1, \"x\")" type=(1, "x")
/// @type.node source=pair type=<T, U = T>(T, U?) => (T, U)
/// @type.node source=1 type=1
/// @type.node source="\"x\"" type="x"
/// @generic.instance id="pair<1, 1>" symbol=pair arguments=[1, 1]
/// @generic.instance id="pair<1, \"x\">" symbol=pair arguments=[1, "x"]
"#);
}
