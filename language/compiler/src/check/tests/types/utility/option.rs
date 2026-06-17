use crate::tests::{DirRows, TestSession};

#[test]
fn test_option_accepts_values_and_null() {
    let session = TestSession::single(
        r#"
const some: Option<int32> = 1 as Option<int32>;
const none: Option<int32> = null as Option<int32>;

some satisfies Option<int32>;
none satisfies Option<int32>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const some: Option<int32> = 1 as Option<int32>;
const none: Option<int32> = null as Option<int32>;

some satisfies Option<int32>;
none satisfies Option<int32>;

=== checked ===
const some: Option<int32> = 1 as Option<int32>;
/// @type.symbol symbol=some source=some type=Option<int32>
/// @resolution.name source=Option target=types.option.Option

const none: Option<int32> = null as Option<int32>;
/// @type.symbol symbol=none source=none type=Option<int32>
/// @resolution.name source=Option target=types.option.Option

some satisfies Option<int32>;
/// @resolution.name source=some target=some
/// @resolution.name source=Option target=types.option.Option

none satisfies Option<int32>;
/// @resolution.name source=none target=none
/// @resolution.name source=Option target=types.option.Option
"#,
    );
}

#[test]
fn test_option_constructors_select_presence_arms() {
    let session = TestSession::single(
        r#"
const some = Option.some(1);
const none = Option<int32>.none();

some satisfies Option<int32>;
none satisfies Option<int32>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const some: Option<int32> = Option<int32>.some(1);
const none: Option<int32> = Option<int32>.none();

some satisfies Option<int32>;
none satisfies Option<int32>;

=== checked ===
const some = Option.some(1);
/// @type.symbol symbol=some type=Option<int32>
/// @resolution.name source=Option target=types.option.Option
/// @resolution.call source=Option.some(1) parameters=(int32) return=Option<int32> kind=symbol target=types.option.Option.some instance=types.option.Option.some<int32>
/// @generic.instance source=Option.some(1) id=types.option.Option.some<int32>

const none = Option<int32>.none();
/// @type.symbol symbol=none type=Option<int32>
/// @resolution.name source=Option target=types.option.Option
/// @resolution.call source="Option<int32>.none()" parameters=() return=Option<int32> kind=symbol target=types.option.Option.none instance=types.option.Option.none<int32>
/// @generic.instance source="Option<int32>.none()" id=types.option.Option.none<int32>

some satisfies Option<int32>;
/// @resolution.name source=some target=some
/// @resolution.name source=Option target=types.option.Option

none satisfies Option<int32>;
/// @resolution.name source=none target=none
/// @resolution.name source=Option target=types.option.Option
/// @generic.instance id=types.option.Option.none<int32> symbol=types.option.Option.none arguments=[int32]
/// @generic.instance id=types.option.Option.some<int32> symbol=types.option.Option.some arguments=[int32]
"#,
    );
}

#[test]
fn test_option_from_nullish_folds_undefined_into_none() {
    let session = TestSession::single(
        r#"
declare const input: string | null | undefined;

const value = Option.fromNullish(input);
value satisfies Option<string>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const input: string | null | undefined;

const value: Option<string> = Option<string>.fromNullish(input);
value satisfies Option<string>;

=== checked ===
declare const input: string | null | undefined;
/// @type.symbol symbol=input source=input type=string | null | undefined

const value = Option.fromNullish(input);
/// @type.symbol symbol=value type=Option<string>
/// @resolution.name source=Option target=types.option.Option
/// @resolution.name source=input target=input
/// @resolution.call source=Option.fromNullish(input) parameters=(string | null | undefined) return=Option<string> kind=symbol target=types.option.Option.fromNullish instance=types.option.Option.fromNullish<string>
/// @generic.instance source=Option.fromNullish(input) id=types.option.Option.fromNullish<string>

value satisfies Option<string>;
/// @resolution.name source=value target=value
/// @resolution.name source=Option target=types.option.Option
/// @generic.instance id=types.option.Option.fromNullish<string> symbol=types.option.Option.fromNullish arguments=[string]
"#,
    );
}

#[test]
fn test_option_rejects_implicit_undefined() {
    let session = TestSession::single(
        r#"
const value: Option<int32> = undefined;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: Option<int32> = undefined;

=== checked ===
const value: Option<int32> = undefined;
/// @type.symbol symbol=value source=value type=Option<int32>
/// @resolution.name source=Option target=types.option.Option
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'undefined' is not assignable to type 'Option<int32>'"
/// @diagnostic.label line=2 column=7 source="const value: Option<int32> = undefined;"
"#,
    );
}

#[test]
fn test_option_predicates_and_map_methods_preserve_carrier() {
    let session = TestSession::single(
        r#"
const value = Option.some(1);

value.isSome() satisfies boolean;
value.isNone() satisfies boolean;
value.map((x) => x + 1) satisfies Option<int32>;
value.mapOr(0, (x) => x + 1) satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: Option<int32> = Option<int32>.some(1);

value.isSome() satisfies boolean;
value.isNone() satisfies boolean;
value.map<int32>((x: int32) => x + 1) satisfies Option<int32>;
value.mapOr<int32>(0, (x: int32) => x + 1) satisfies int32;

=== checked ===
const value = Option.some(1);
/// @type.symbol symbol=value type=Option<int32>
/// @resolution.name source=Option target=types.option.Option
/// @resolution.call source=Option.some(1) parameters=(int32) return=Option<int32> kind=symbol target=types.option.Option.some instance=types.option.Option.some<int32>
/// @generic.instance source=Option.some(1) id=types.option.Option.some<int32>

value.isSome() satisfies boolean;
/// @resolution.name source=value target=value
/// @resolution.member source=value.isSome receiver=Option<int32> kind=symbol target=types.option.Option.isSome

value.isNone() satisfies boolean;
/// @resolution.name source=value target=value
/// @resolution.member source=value.isNone receiver=Option<int32> kind=symbol target=types.option.Option.isNone

value.map((x) => x + 1) satisfies Option<int32>;
/// @resolution.name source=value target=value
/// @resolution.member source=value.map receiver=Option<int32> kind=symbol target=types.option.Option.map

value.mapOr(0, (x) => x + 1) satisfies int32;
/// @resolution.name source=value target=value
/// @resolution.member source=value.mapOr receiver=Option<int32> kind=symbol target=types.option.Option.mapOr
/// @generic.instance id=types.option.Option.some<int32> symbol=types.option.Option.some arguments=[int32]
"#,
    );
}
