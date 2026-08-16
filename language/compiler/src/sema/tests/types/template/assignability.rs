use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_type_assigns_to_broader_template() {
    let session = TestSession::single(
        r#"
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare const tight: Tight;
const loose: Loose = tight;
"#,
    );

    session.assert_dir(
        "main.ds",
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
/// @type.symbol symbol=tight source=tight type=`user-${string}-id`
/// @resolution.pattern source=tight kind=binding target=tight
/// @resolution.name source=Tight target=Tight

const loose: Loose = tight;
/// @type.symbol symbol=loose source=loose type=`${string}-id`
/// @resolution.pattern source=loose kind=binding target=loose
/// @resolution.name source=Loose target=Loose
/// @resolution.name source=tight target=tight
/// @resolution.place source=tight placement="local" lifetime="static" access="readonly"
/// @resolution.access source=tight root=tight
"#,
    );
}

#[test]
fn test_template_literal_type_rejects_assignment_to_narrower_template() {
    let session = TestSession::single(
        r#"
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare const loose: Loose;
const tight: Tight = loose;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
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
/// @type.symbol symbol=loose source=loose type=`${string}-id`
/// @resolution.pattern source=loose kind=binding target=loose
/// @resolution.name source=Loose target=Loose

const tight: Tight = loose;
/// @type.symbol symbol=tight source=tight type=`user-${string}-id`
/// @resolution.pattern source=tight kind=binding target=tight
/// @resolution.name source=Tight target=Tight
/// @resolution.name source=loose target=loose
/// @resolution.place source=loose placement="local" lifetime="static" access="readonly"
/// @resolution.access source=loose root=loose
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '`${string}-id`' is not assignable to type '`user-${string}-id`'"
/// @diagnostic.label line=6 column=22 span="loose" line_source="const tight: Tight = loose;"
/// @diagnostic.related line=6 column=14 span="Tight" line_source="const tight: Tight = loose;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_template_literal_numeric_span_assigns_to_string_span() {
    let session = TestSession::single(
        r#"
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare const numeric: NumericId;
const id: StringId = numeric;
"#,
    );

    session.assert_dir(
        "main.ds",
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
/// @type.symbol symbol=numeric source=numeric type=`id-${float64}`
/// @resolution.pattern source=numeric kind=binding target=numeric
/// @resolution.name source=NumericId target=NumericId

const id: StringId = numeric;
/// @type.symbol symbol=id source=id type=`id-${string}`
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=StringId target=StringId
/// @resolution.name source=numeric target=numeric
/// @resolution.place source=numeric placement="local" lifetime="static" access="readonly"
/// @resolution.access source=numeric root=numeric
"#,
    );
}

#[test]
fn test_template_literal_string_span_rejects_numeric_span_target() {
    let session = TestSession::single(
        r#"
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare const id: StringId;
const numeric: NumericId = id;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
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
/// @type.symbol symbol=id source=id type=`id-${string}`
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=StringId target=StringId

const numeric: NumericId = id;
/// @type.symbol symbol=numeric source=numeric type=`id-${float64}`
/// @resolution.pattern source=numeric kind=binding target=numeric
/// @resolution.name source=NumericId target=NumericId
/// @resolution.name source=id target=id
/// @resolution.place source=id placement="local" lifetime="static" access="readonly"
/// @resolution.access source=id root=id
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '`id-${string}`' is not assignable to type '`id-${float64}`'"
/// @diagnostic.label line=6 column=28 span="id" line_source="const numeric: NumericId = id;"
/// @diagnostic.related line=6 column=16 span="NumericId" line_source="const numeric: NumericId = id;" message="expected due to this annotation"
"#,
    );
}
