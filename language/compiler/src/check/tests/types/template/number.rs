use crate::tests::{DirRows, TestSession};

#[test]
fn test_number_template_span_accepts_numeric_string_forms() {
    let session = TestSession::single(
        r#"
type Numeric = `${number}`;

const decimal: Numeric = "42";
const exponent: Numeric = "1e3";
const hexadecimal: Numeric = "0x1";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Numeric = `${number}`;

const decimal: Numeric = "42";
const exponent: Numeric = "1e3";
const hexadecimal: Numeric = "0x1";

=== checked ===
type Numeric = `${number}`;
/// @type.symbol symbol=Numeric source="type Numeric = `${number}`" type=`${number}`
/// @definition.type symbol=Numeric source="type Numeric = `${number}`" value=`${number}`

const decimal: Numeric = "42";
/// @type.symbol symbol=decimal source=decimal type=`${number}`
/// @resolution.name source=Numeric target=Numeric

const exponent: Numeric = "1e3";
/// @type.symbol symbol=exponent source=exponent type=`${number}`
/// @resolution.name source=Numeric target=Numeric

const hexadecimal: Numeric = "0x1";
/// @type.symbol symbol=hexadecimal source=hexadecimal type=`${number}`
/// @resolution.name source=Numeric target=Numeric
"#,
    );
}

#[test]
fn test_number_template_span_rejects_invalid_number_strings() {
    let session = TestSession::single(
        r#"
type Numeric = `${number}`;

const bad: Numeric = "NaN";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Numeric = `${number}`;

const bad: Numeric = "NaN";

=== checked ===
type Numeric = `${number}`;
/// @type.symbol symbol=Numeric source="type Numeric = `${number}`" type=`${number}`
/// @definition.type symbol=Numeric source="type Numeric = `${number}`" value=`${number}`

const bad: Numeric = "NaN";
/// @type.symbol symbol=bad source=bad type=`${number}`
/// @resolution.name source=Numeric target=Numeric
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"NaN\"' is not assignable to type 'Numeric'"
/// @diagnostic.label line=4 column=7 source="const bad: Numeric = \"NaN\";"
"#,
    );
}

#[test]
fn test_bigint_template_span_matches_bigint_literals() {
    let session = TestSession::single(
        r#"
type Big = `${bigint}`;

const decimal: Big = "900";
const negative: Big = "-1";
const hexadecimal: Big = "0x1";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Big = `${bigint}`;

const decimal: Big = "900";
const negative: Big = "-1";
const hexadecimal: Big = "0x1";

=== checked ===
type Big = `${bigint}`;
/// @type.symbol symbol=Big source="type Big = `${bigint}`" type=`${bigint}`
/// @definition.type symbol=Big source="type Big = `${bigint}`" value=`${bigint}`

const decimal: Big = "900";
/// @type.symbol symbol=decimal source=decimal type=`${bigint}`
/// @resolution.name source=Big target=Big

const negative: Big = "-1";
/// @type.symbol symbol=negative source=negative type=`${bigint}`
/// @resolution.name source=Big target=Big

const hexadecimal: Big = "0x1";
/// @type.symbol symbol=hexadecimal source=hexadecimal type=`${bigint}`
/// @resolution.name source=Big target=Big
"#,
    );
}

#[test]
fn test_int_template_span_rejects_out_of_range_strings() {
    let session = TestSession::single(
        r#"
type Small = `${int8}`;

const bad: Small = "128";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Small = `${int8}`;

const bad: Small = "128";

=== checked ===
type Small = `${int8}`;
/// @type.symbol symbol=Small source="type Small = `${int8}`" type=`${int8}`
/// @definition.type symbol=Small source="type Small = `${int8}`" value=`${int8}`

const bad: Small = "128";
/// @type.symbol symbol=bad source=bad type=`${int8}`
/// @resolution.name source=Small target=Small
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"128\"' is not assignable to type 'Small'"
/// @diagnostic.label line=4 column=7 source="const bad: Small = \"128\";"
"#,
    );
}

#[test]
fn test_number_template_call_infers_non_canonical_number() {
    let session = TestSession::single(
        r#"
declare function parse<T: number>(value: `${T}`): T;

const value = parse("1e3");
value satisfies number;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: number>(value: `${T}`): T;

const value: number = parse<number>("1e3");
value satisfies number;

=== checked ===
declare function parse<T: number>(value: `${T}`): T;
/// @generic.template symbol=parse parameters=[T: number]
/// @type.symbol symbol=parse type=<T: number>(`${T}`) => T
/// @type.symbol symbol=value type=`${T}`

const value = parse("1e3");
/// @type.symbol symbol=value type=number
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"1e3\")" parameters=(`${number}`) return=number kind=symbol target=parse instance=parse<number>
/// @generic.instance source="parse(\"1e3\")" id=parse<number>

value satisfies number;
/// @resolution.name source=value target=value
/// @generic.instance id=parse<number> symbol=parse arguments=[number]
"#,
    );
}

#[test]
fn test_int_template_call_rejects_out_of_range_span() {
    let session = TestSession::single(
        r#"
declare function parse<T: int8>(value: `${T}`): T;

parse("128");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: int8>(value: `${T}`): T;

parse("128");

=== checked ===
declare function parse<T: int8>(value: `${T}`): T;
/// @generic.template symbol=parse parameters=[T: int8]
/// @type.symbol symbol=parse type=<T: int8>(`${T}`) => T
/// @type.symbol symbol=value type=`${T}`

parse("128");
/// @resolution.name source=parse target=parse
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"128\"' is not assignable to type '`${int8}`'"
/// @diagnostic.label line=4 column=1 source="parse(\"128\");"
"#,
    );
}

#[test]
fn test_bigint_template_call_infers_bigint_literal() {
    let session = TestSession::single(
        r#"
declare function parse<T: bigint>(value: `${T}`): T;

const value = parse("-1");
value satisfies -1n;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: bigint>(value: `${T}`): T;

const value: -1n = parse<-1n>("-1");
value satisfies -1n;

=== checked ===
declare function parse<T: bigint>(value: `${T}`): T;
/// @generic.template symbol=parse parameters=[T: bigint]
/// @type.symbol symbol=parse type=<T: bigint>(`${T}`) => T
/// @type.symbol symbol=value type=`${T}`

const value = parse("-1");
/// @type.symbol symbol=value type=-1n
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"-1\")" parameters=(`${-1n}`) return=-1n kind=symbol target=parse instance="parse<-1n>"
/// @generic.instance source="parse(\"-1\")" id="parse<-1n>"

value satisfies -1n;
/// @resolution.name source=value target=value
/// @generic.instance id="parse<-1n>" symbol=parse arguments=[-1n]
"#,
    );
}
