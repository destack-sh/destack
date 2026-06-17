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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare const tight: Tight;
const loose: Loose = tight;

=== checked ===
type Loose = `${string}-id`;
/// @type.symbol symbol=Loose source="type Loose = `${string}-id`" type=`${string}-id`
/// @definition.type symbol=Loose source="type Loose = `${string}-id`" value=`${string}-id`

type Tight = `user-${string}-id`;
/// @type.symbol symbol=Tight source="type Tight = `user-${string}-id`" type=`user-${string}-id`
/// @definition.type symbol=Tight source="type Tight = `user-${string}-id`" value=`user-${string}-id`

declare const tight: Tight;
/// @type.symbol symbol=tight source=tight type=`user-${string}-id`
/// @resolution.name source=Tight target=Tight

const loose: Loose = tight;
/// @type.symbol symbol=loose source=loose type=`${string}-id`
/// @resolution.name source=Loose target=Loose
/// @resolution.name source=tight target=tight
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare const loose: Loose;
const tight: Tight = loose;

=== checked ===
type Loose = `${string}-id`;
/// @type.symbol symbol=Loose source="type Loose = `${string}-id`" type=`${string}-id`
/// @definition.type symbol=Loose source="type Loose = `${string}-id`" value=`${string}-id`

type Tight = `user-${string}-id`;
/// @type.symbol symbol=Tight source="type Tight = `user-${string}-id`" type=`user-${string}-id`
/// @definition.type symbol=Tight source="type Tight = `user-${string}-id`" value=`user-${string}-id`

declare const loose: Loose;
/// @type.symbol symbol=loose source=loose type=`${string}-id`
/// @resolution.name source=Loose target=Loose

const tight: Tight = loose;
/// @type.symbol symbol=tight source=tight type=`user-${string}-id`
/// @resolution.name source=Tight target=Tight
/// @resolution.name source=loose target=loose
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Loose' is not assignable to type 'Tight'"
/// @diagnostic.label line=6 column=7 source="const tight: Tight = loose;"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare const numeric: NumericId;
const id: StringId = numeric;

=== checked ===
type NumericId = `id-${number}`;
/// @type.symbol symbol=NumericId source="type NumericId = `id-${number}`" type=`id-${number}`
/// @definition.type symbol=NumericId source="type NumericId = `id-${number}`" value=`id-${number}`

type StringId = `id-${string}`;
/// @type.symbol symbol=StringId source="type StringId = `id-${string}`" type=`id-${string}`
/// @definition.type symbol=StringId source="type StringId = `id-${string}`" value=`id-${string}`

declare const numeric: NumericId;
/// @type.symbol symbol=numeric source=numeric type=`id-${number}`
/// @resolution.name source=NumericId target=NumericId

const id: StringId = numeric;
/// @type.symbol symbol=id source=id type=`id-${string}`
/// @resolution.name source=StringId target=StringId
/// @resolution.name source=numeric target=numeric
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare const id: StringId;
const numeric: NumericId = id;

=== checked ===
type NumericId = `id-${number}`;
/// @type.symbol symbol=NumericId source="type NumericId = `id-${number}`" type=`id-${number}`
/// @definition.type symbol=NumericId source="type NumericId = `id-${number}`" value=`id-${number}`

type StringId = `id-${string}`;
/// @type.symbol symbol=StringId source="type StringId = `id-${string}`" type=`id-${string}`
/// @definition.type symbol=StringId source="type StringId = `id-${string}`" value=`id-${string}`

declare const id: StringId;
/// @type.symbol symbol=id source=id type=`id-${string}`
/// @resolution.name source=StringId target=StringId

const numeric: NumericId = id;
/// @type.symbol symbol=numeric source=numeric type=`id-${number}`
/// @resolution.name source=NumericId target=NumericId
/// @resolution.name source=id target=id
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'StringId' is not assignable to type 'NumericId'"
/// @diagnostic.label line=6 column=7 source="const numeric: NumericId = id;"
"#,
    );
}
