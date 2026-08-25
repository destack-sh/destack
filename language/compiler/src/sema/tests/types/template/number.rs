use crate::tests::{DirRows, TestSession};

/// A number template span accepts decimal, exponent, and hexadecimal text.
#[test]
fn test_accept_numeric_string_forms_in_a_number_template_span() {
    let session = TestSession::single(
        r#"
type Numeric = `${number}`;

const decimal: Numeric = "42";
const exponent: Numeric = "1e3";
const hexadecimal: Numeric = "0x1";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Numeric = `${number}`;

const decimal: `${float64}` = "42";
const exponent: `${float64}` = "1e3";
const hexadecimal: `${float64}` = "0x1";

=== dir ===
type Numeric = `${number}`;
/// @type.symbol symbol=Numeric source="type Numeric = `${number}`" type=`${float64}`
/// @definition.type symbol=Numeric source="type Numeric = `${number}`" value=`${float64}`

const decimal: Numeric = "42";
/// @type.symbol symbol=decimal source=decimal type=`${float64}`
/// @resolution.pattern source=decimal kind=binding target=decimal
/// @resolution.name source=Numeric target=Numeric

const exponent: Numeric = "1e3";
/// @type.symbol symbol=exponent source=exponent type=`${float64}`
/// @resolution.pattern source=exponent kind=binding target=exponent
/// @resolution.name source=Numeric target=Numeric

const hexadecimal: Numeric = "0x1";
/// @type.symbol symbol=hexadecimal source=hexadecimal type=`${float64}`
/// @resolution.pattern source=hexadecimal kind=binding target=hexadecimal
/// @resolution.name source=Numeric target=Numeric
"#,
    );
}

/// A text that names no number reports a diagnostic at a number span.
#[test]
fn test_reject_invalid_number_strings_in_a_number_template_span() {
    let session = TestSession::single(
        r#"
type Numeric = `${number}`;

const bad: Numeric = "NaN";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Numeric = `${number}`;

const bad: `${float64}` = "NaN";

=== dir ===
type Numeric = `${number}`;
/// @type.symbol symbol=Numeric source="type Numeric = `${number}`" type=`${float64}`
/// @definition.type symbol=Numeric source="type Numeric = `${number}`" value=`${float64}`

const bad: Numeric = "NaN";
/// @type.symbol symbol=bad source=bad type=`${float64}`
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Numeric target=Numeric
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"NaN\"' is not assignable to type '`${float64}`'"
/// @diagnostic.label line=4 column=22 span="\"NaN\"" line_source="const bad: Numeric = \"NaN\";"
/// @diagnostic.related line=4 column=12 span="Numeric" line_source="const bad: Numeric = \"NaN\";" message="expected due to this annotation"
"#,
    );
}

/// A bigint template span accepts decimal, negative, and hexadecimal text.
#[test]
fn test_match_bigint_literals_in_a_bigint_template_span() {
    let session = TestSession::single(
        r#"
type Big = `${bigint}`;

const decimal: Big = "900";
const negative: Big = "-1";
const hexadecimal: Big = "0x1";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Big = `${bigint}`;

const decimal: `${bigint}` = "900";
const negative: `${bigint}` = "-1";
const hexadecimal: `${bigint}` = "0x1";

=== dir ===
type Big = `${bigint}`;
/// @type.symbol symbol=Big source="type Big = `${bigint}`" type=`${bigint}`
/// @definition.type symbol=Big source="type Big = `${bigint}`" value=`${bigint}`

const decimal: Big = "900";
/// @type.symbol symbol=decimal source=decimal type=`${bigint}`
/// @resolution.pattern source=decimal kind=binding target=decimal
/// @resolution.name source=Big target=Big

const negative: Big = "-1";
/// @type.symbol symbol=negative source=negative type=`${bigint}`
/// @resolution.pattern source=negative kind=binding target=negative
/// @resolution.name source=Big target=Big

const hexadecimal: Big = "0x1";
/// @type.symbol symbol=hexadecimal source=hexadecimal type=`${bigint}`
/// @resolution.pattern source=hexadecimal kind=binding target=hexadecimal
/// @resolution.name source=Big target=Big
"#,
    );
}

/// A text outside the width reports a diagnostic at an int template span.
#[test]
fn test_reject_out_of_range_strings_in_an_int_template_span() {
    let session = TestSession::single(
        r#"
type Small = `${int8}`;

const bad: Small = "128";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Small = `${int8}`;

const bad: `${int8}` = "128";

=== dir ===
type Small = `${int8}`;
/// @type.symbol symbol=Small source="type Small = `${int8}`" type=`${int8}`
/// @definition.type symbol=Small source="type Small = `${int8}`" value=`${int8}`

const bad: Small = "128";
/// @type.symbol symbol=bad source=bad type=`${int8}`
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Small target=Small
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"128\"' is not assignable to type '`${int8}`'"
/// @diagnostic.label line=4 column=20 span="\"128\"" line_source="const bad: Small = \"128\";"
/// @diagnostic.related line=4 column=12 span="Small" line_source="const bad: Small = \"128\";" message="expected due to this annotation"
"#,
    );
}

/// A call through a number template span infers a number written in exponent form.
#[test]
fn test_infer_a_non_canonical_number_through_a_number_template_call() {
    let session = TestSession::single(
        r#"
declare function parse<T: number>(value: `${T}`): T;

const value = parse("1e3");
value satisfies number;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: number>(value: `${T}`): T;

const value: 1000 = parse<1000>("1e3");
value satisfies number;

=== dir ===
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
/// @generic.instantiation id=parse<1000> template=parse arguments=(1000)
/// @generic.instance id=parse<1000> template=parse arguments=(1000)

value satisfies number;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
"#,
    );
}

/// A text outside the width reports a diagnostic at an int template call.
#[test]
fn test_reject_an_out_of_range_span_in_an_int_template_call() {
    let session = TestSession::single(
        r#"
declare function parse<T: int8>(value: `${T}`): T;

parse("128");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: int8>(value: `${T}`): T;

parse("128");

=== dir ===
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
/// @generic.instantiation id=parse<<error>> template=parse arguments=(<error>)
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"128\"' is not assignable to parameter of type '`${_}`'"
/// @diagnostic.label line=4 column=7 span="\"128\"" line_source="parse(\"128\");"
/// @diagnostic.related line=4 column=1 span="parse(\"128\")" line_source="parse(\"128\");" message="in this call"
"#,
    );
}

/// A call through a bigint template span infers the bigint literal.
#[test]
fn test_infer_a_bigint_literal_through_a_bigint_template_call() {
    let session = TestSession::single(
        r#"
declare function parse<T: bigint>(value: `${T}`): T;

const value = parse("-1");
value satisfies -1n;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: bigint>(value: `${T}`): T;

const value: -1n = parse<-1n>("-1");
value satisfies -1n;

=== dir ===
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
/// @generic.instantiation id=parse<-1n> template=parse arguments=(-1n)
/// @generic.instance id=parse<-1n> template=parse arguments=(-1n)

value satisfies -1n;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
"#,
    );
}

/// A closed number span accepts every text naming its value.
#[test]
fn test_accept_an_equivalent_written_form_for_a_number_template_typed_binding() {
    let session = TestSession::single(
        r#"
const canonical: `${1000}` = "1000";
const exponent: `${1000}` = "1e3";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const canonical: `${1000}` = "1000";
const exponent: `${1000}` = "1e3";

=== dir ===
const canonical: `${1000}` = "1000";
/// @type.symbol symbol=canonical source=canonical type=`${1000}`
/// @resolution.pattern source=canonical kind=binding target=canonical

const exponent: `${1000}` = "1e3";
/// @type.symbol symbol=exponent source=exponent type=`${1000}`
/// @resolution.pattern source=exponent kind=binding target=exponent
"#,
    );
}

/// A text naming another value reports a diagnostic at a closed number span.
#[test]
fn test_reject_another_value_for_a_number_template_typed_binding() {
    let session = TestSession::single(
        r#"
const wrong: `${1000}` = "1001";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const wrong: `${1000}` = "1001";

=== dir ===
const wrong: `${1000}` = "1001";
/// @type.symbol symbol=wrong source=wrong type=`${1000}`
/// @resolution.pattern source=wrong kind=binding target=wrong
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"1001\"' is not assignable to type '`${1000}`'"
/// @diagnostic.label line=2 column=26 span="\"1001\"" line_source="const wrong: `${1000}` = \"1001\";"
/// @diagnostic.related line=2 column=14 span="`${1000}`" line_source="const wrong: `${1000}` = \"1001\";" message="expected due to this annotation"
"#,
    );
}

/// Keep integer spans matching every written form of an integer in the domain.
#[test]
fn test_keep_integer_domain_spans_for_prefixed_numeric_strings() {
    let session = TestSession::single(
        r#"
type Count = `${int32}`;

const decimal: Count = "10";
const binary: Count = "0b1010";
const octal: Count = "0o12";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = `${int32}`;

const decimal: `${int32}` = "10";
const binary: `${int32}` = "0b1010";
const octal: `${int32}` = "0o12";

=== dir ===
type Count = `${int32}`;
/// @type.symbol symbol=Count source="type Count = `${int32}`" type=`${int32}`
/// @definition.type symbol=Count source="type Count = `${int32}`" value=`${int32}`

const decimal: Count = "10";
/// @type.symbol symbol=decimal source=decimal type=`${int32}`
/// @resolution.pattern source=decimal kind=binding target=decimal
/// @resolution.name source=Count target=Count

const binary: Count = "0b1010";
/// @type.symbol symbol=binary source=binary type=`${int32}`
/// @resolution.pattern source=binary kind=binding target=binary
/// @resolution.name source=Count target=Count

const octal: Count = "0o12";
/// @type.symbol symbol=octal source=octal type=`${int32}`
/// @resolution.pattern source=octal kind=binding target=octal
/// @resolution.name source=Count target=Count
"#,
        r#"
"#,
    );
}
