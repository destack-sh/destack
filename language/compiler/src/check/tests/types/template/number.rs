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
/// @type.symbol symbol=Numeric source="type Numeric = `${number}`" type=`${float64}`
/// @definition.type symbol=Numeric source="type Numeric = `${number}`" value=`${float64}`

const decimal: Numeric = "42";
/// @type.symbol symbol=decimal source=decimal type=Numeric reduced=`${float64}`
/// @resolution.pattern source=decimal kind=binding target=decimal
/// @resolution.name source=Numeric target=Numeric

const exponent: Numeric = "1e3";
/// @type.symbol symbol=exponent source=exponent type=Numeric reduced=`${float64}`
/// @resolution.pattern source=exponent kind=binding target=exponent
/// @resolution.name source=Numeric target=Numeric

const hexadecimal: Numeric = "0x1";
/// @type.symbol symbol=hexadecimal source=hexadecimal type=Numeric reduced=`${float64}`
/// @resolution.pattern source=hexadecimal kind=binding target=hexadecimal
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
/// @type.symbol symbol=Numeric source="type Numeric = `${number}`" type=`${float64}`
/// @definition.type symbol=Numeric source="type Numeric = `${number}`" value=`${float64}`

const bad: Numeric = "NaN";
/// @type.symbol symbol=bad source=bad type=Numeric reduced=`${float64}`
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Numeric target=Numeric
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"NaN\"' is not assignable to type 'Numeric'"
/// @diagnostic.label line=4 column=22 span="\"NaN\"" line_source="const bad: Numeric = \"NaN\";"
/// @diagnostic.related line=4 column=12 span="Numeric" line_source="const bad: Numeric = \"NaN\";" message="expected due to this annotation"
/// @diagnostic.note message="'Numeric' reduces to '`${float64}`'"
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
/// @type.symbol symbol=decimal source=decimal type=Big reduced=`${bigint}`
/// @resolution.pattern source=decimal kind=binding target=decimal
/// @resolution.name source=Big target=Big

const negative: Big = "-1";
/// @type.symbol symbol=negative source=negative type=Big reduced=`${bigint}`
/// @resolution.pattern source=negative kind=binding target=negative
/// @resolution.name source=Big target=Big

const hexadecimal: Big = "0x1";
/// @type.symbol symbol=hexadecimal source=hexadecimal type=Big reduced=`${bigint}`
/// @resolution.pattern source=hexadecimal kind=binding target=hexadecimal
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
/// @type.symbol symbol=bad source=bad type=Small reduced=`${int8}`
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Small target=Small
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"128\"' is not assignable to type 'Small'"
/// @diagnostic.label line=4 column=20 span="\"128\"" line_source="const bad: Small = \"128\";"
/// @diagnostic.related line=4 column=12 span="Small" line_source="const bad: Small = \"128\";" message="expected due to this annotation"
/// @diagnostic.note message="'Small' reduces to '`${int8}`'"
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

const value: 1000 = parse<1000>("1e3");
value satisfies number;

=== checked ===
declare function parse<T: number>(value: `${T}`): T;
/// @generic.template symbol=parse parameters=(T: float64)
/// @type.symbol symbol=parse source="declare function parse<T: number>(value: `${T}`): T" type=<T: float64>(`${T}`) => T
/// @type.symbol symbol=parse.T source="T: number" type=T
/// @type.symbol symbol=parse.value source="value: `${T}`" type=`${T}`
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const value = parse("1e3");
/// @type.symbol symbol=value source=value type=1000
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"1e3\")" parameters=(`${1000}`) arguments=(provided("1e3") as `${1000}`) return=1000 kind=symbol target=parse instance=parse<1000>
/// @generic.instance source="parse(\"1e3\")" id=parse<1000>

value satisfies number;
/// @resolution.name source=value target=value

/// @generic.instance id=parse<1000> template=parse arguments=(1000)
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
/// @generic.template symbol=parse parameters=(T: int8)
/// @type.symbol symbol=parse source="declare function parse<T: int8>(value: `${T}`): T" type=<T: int8>(`${T}`) => T
/// @type.symbol symbol=parse.T source="T: int8" type=T
/// @type.symbol symbol=parse.value source="value: `${T}`" type=`${T}`
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

parse("128");
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"128\")" parameters=(`${<error>}`) arguments=(provided("128") as `${<error>}`) return=<error> kind=symbol target=parse instance=parse<<error>>
/// @generic.instance source="parse(\"128\")" id=parse<<error>>

/// @generic.instance id=parse<<error>> template=parse arguments=(<error>)
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"128\"' is not assignable to parameter of type '`${_}`'"
/// @diagnostic.label line=4 column=7 span="\"128\"" line_source="parse(\"128\");"
/// @diagnostic.related line=4 column=1 span="parse(\"128\")" line_source="parse(\"128\");" message="in this call"
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
/// @generic.template symbol=parse parameters=(T: bigint)
/// @type.symbol symbol=parse source="declare function parse<T: bigint>(value: `${T}`): T" type=<T: bigint>(`${T}`) => T
/// @type.symbol symbol=parse.T source="T: bigint" type=T
/// @type.symbol symbol=parse.value source="value: `${T}`" type=`${T}`
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const value = parse("-1");
/// @type.symbol symbol=value source=value type=-1n
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"-1\")" parameters=(`${-1n}`) arguments=(provided("-1") as `${-1n}`) return=-1n kind=symbol target=parse instance=parse<-1n>
/// @generic.instance source="parse(\"-1\")" id=parse<-1n>

value satisfies -1n;
/// @resolution.name source=value target=value

/// @generic.instance id=parse<-1n> template=parse arguments=(-1n)
"#,
    );
}
