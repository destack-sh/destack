use crate::tests::{DirRows, TestSession};

/// A template literal type assigns to a template it refines.
#[test]
fn test_assign_a_template_literal_type_to_a_broader_template() {
    let session = TestSession::single(
        r#"
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare const tight: Tight;
const loose: Loose = tight;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare const tight: Tight;
const loose: Loose = tight;

=== dir ===
type Loose = `${string}-id`;
/// @type.symbol symbol=Loose source="type Loose = `${string}-id`" type=`${string}-id`
/// @definition.type symbol=Loose source="type Loose = `${string}-id`" value=`${string}-id`

type Tight = `user-${string}-id`;
/// @type.symbol symbol=Tight source="type Tight = `user-${string}-id`" type=`user-${string}-id`
/// @definition.type symbol=Tight source="type Tight = `user-${string}-id`" value=`user-${string}-id`

declare const tight: Tight;
/// @type.symbol symbol=tight source=tight type=Tight
/// @resolution.pattern source=tight kind=binding target=tight
/// @resolution.name source=Tight target=Tight

const loose: Loose = tight;
/// @type.symbol symbol=loose source=loose type=Loose
/// @resolution.pattern source=loose kind=binding target=loose
/// @resolution.name source=Loose target=Loose
/// @resolution.name source=tight target=tight
/// @resolution.place source=tight placement="local" lifetime="static" access="immutable"
/// @resolution.access source=tight root=tight
"#,
    );
}

/// Assigning a template literal type to a template it refines reports a diagnostic.
#[test]
fn test_reject_a_template_literal_type_assigned_to_a_narrower_template() {
    let session = TestSession::single(
        r#"
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare const loose: Loose;
const tight: Tight = loose;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare const loose: Loose;
const tight: Tight = loose;

=== dir ===
type Loose = `${string}-id`;
/// @type.symbol symbol=Loose source="type Loose = `${string}-id`" type=`${string}-id`
/// @definition.type symbol=Loose source="type Loose = `${string}-id`" value=`${string}-id`

type Tight = `user-${string}-id`;
/// @type.symbol symbol=Tight source="type Tight = `user-${string}-id`" type=`user-${string}-id`
/// @definition.type symbol=Tight source="type Tight = `user-${string}-id`" value=`user-${string}-id`

declare const loose: Loose;
/// @type.symbol symbol=loose source=loose type=Loose
/// @resolution.pattern source=loose kind=binding target=loose
/// @resolution.name source=Loose target=Loose

const tight: Tight = loose;
/// @type.symbol symbol=tight source=tight type=Tight
/// @resolution.pattern source=tight kind=binding target=tight
/// @resolution.name source=Tight target=Tight
/// @resolution.name source=loose target=loose
/// @resolution.place source=loose placement="local" lifetime="static" access="immutable"
/// @resolution.access source=loose root=loose
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Loose' is not assignable to type 'Tight'"
/// @diagnostic.label line=6 column=22 span="loose" line_source="const tight: Tight = loose;"
/// @diagnostic.related line=6 column=14 span="Tight" line_source="const tight: Tight = loose;" message="expected due to this annotation"
/// @diagnostic.note message="'Loose' reduces to '`${string}-id`'"
/// @diagnostic.note message="'Tight' reduces to '`user-${string}-id`'"
"#,
    );
}

/// A numeric template span assigns to the same template with a string span.
#[test]
fn test_assign_a_numeric_template_span_to_a_string_span() {
    let session = TestSession::single(
        r#"
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare const numeric: NumericId;
const id: StringId = numeric;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare const numeric: NumericId;
const id: StringId = numeric;

=== dir ===
type NumericId = `id-${number}`;
/// @type.symbol symbol=NumericId source="type NumericId = `id-${number}`" type=`id-${float64}`
/// @definition.type symbol=NumericId source="type NumericId = `id-${number}`" value=`id-${float64}`

type StringId = `id-${string}`;
/// @type.symbol symbol=StringId source="type StringId = `id-${string}`" type=`id-${string}`
/// @definition.type symbol=StringId source="type StringId = `id-${string}`" value=`id-${string}`

declare const numeric: NumericId;
/// @type.symbol symbol=numeric source=numeric type=NumericId
/// @resolution.pattern source=numeric kind=binding target=numeric
/// @resolution.name source=NumericId target=NumericId

const id: StringId = numeric;
/// @type.symbol symbol=id source=id type=StringId
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=StringId target=StringId
/// @resolution.name source=numeric target=numeric
/// @resolution.place source=numeric placement="local" lifetime="static" access="immutable"
/// @resolution.access source=numeric root=numeric
"#,
    );
}

/// Assigning a string template span to a numeric span reports a diagnostic.
#[test]
fn test_reject_a_string_template_span_assigned_to_a_numeric_span() {
    let session = TestSession::single(
        r#"
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare const id: StringId;
const numeric: NumericId = id;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare const id: StringId;
const numeric: NumericId = id;

=== dir ===
type NumericId = `id-${number}`;
/// @type.symbol symbol=NumericId source="type NumericId = `id-${number}`" type=`id-${float64}`
/// @definition.type symbol=NumericId source="type NumericId = `id-${number}`" value=`id-${float64}`

type StringId = `id-${string}`;
/// @type.symbol symbol=StringId source="type StringId = `id-${string}`" type=`id-${string}`
/// @definition.type symbol=StringId source="type StringId = `id-${string}`" value=`id-${string}`

declare const id: StringId;
/// @type.symbol symbol=id source=id type=StringId
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=StringId target=StringId

const numeric: NumericId = id;
/// @type.symbol symbol=numeric source=numeric type=NumericId
/// @resolution.pattern source=numeric kind=binding target=numeric
/// @resolution.name source=NumericId target=NumericId
/// @resolution.name source=id target=id
/// @resolution.place source=id placement="local" lifetime="static" access="immutable"
/// @resolution.access source=id root=id
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'StringId' is not assignable to type 'NumericId'"
/// @diagnostic.label line=6 column=28 span="id" line_source="const numeric: NumericId = id;"
/// @diagnostic.related line=6 column=16 span="NumericId" line_source="const numeric: NumericId = id;" message="expected due to this annotation"
/// @diagnostic.note message="'StringId' reduces to '`id-${string}`'"
/// @diagnostic.note message="'NumericId' reduces to '`id-${float64}`'"
"#,
    );
}

/// Assign a string literal into a matching template and reject a mismatched one.
#[test]
fn test_assign_a_string_literal_into_a_matching_template() {
    let session = TestSession::single(
        r#"
type Route = `/${string}`;

const ok: Route = "/users";
const bad: Route = "users";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `/${string}`;

const ok: Route = "/users";
const bad: Route = "users";

=== dir ===
type Route = `/${string}`;
/// @type.symbol symbol=Route source="type Route = `/${string}`" type=`/${string}`
/// @definition.type symbol=Route source="type Route = `/${string}`" value=`/${string}`

const ok: Route = "/users";
/// @type.symbol symbol=ok source=ok type=Route
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Route target=Route

const bad: Route = "users";
/// @type.symbol symbol=bad source=bad type=Route
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Route target=Route
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"users\"' is not assignable to type 'Route'"
/// @diagnostic.label line=5 column=20 span="\"users\"" line_source="const bad: Route = \"users\";"
/// @diagnostic.related line=5 column=12 span="Route" line_source="const bad: Route = \"users\";" message="expected due to this annotation"
/// @diagnostic.note message="'Route' reduces to '`/${string}`'"
"#,
    );
}
