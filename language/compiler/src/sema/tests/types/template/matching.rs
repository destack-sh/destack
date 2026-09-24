use crate::tests::{DirRows, TestSession};

/// A template literal type matches each arm of a string union span.
#[test]
fn test_match_string_union_spans_against_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type Route = `api:${"users" | "posts"}`;

const users: Route = "api:users";
const posts: Route = "api:posts";

users satisfies "api:users" | "api:posts";
posts satisfies Route;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${"users" | "posts"}`;

const users: Route = "api:users";
const posts: Route = "api:posts";

users satisfies "api:users" | "api:posts";
posts satisfies Route;

=== dir ===
type Route = `api:${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" type="api:users" | "api:posts"
/// @definition.type symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" value=`api:${"users" | "posts"}`

const users: Route = "api:users";
/// @type.symbol symbol=users source=users type=Route
/// @resolution.pattern source=users kind=binding target=users
/// @resolution.name source=Route target=Route

const posts: Route = "api:posts";
/// @type.symbol symbol=posts source=posts type=Route
/// @resolution.pattern source=posts kind=binding target=posts
/// @resolution.name source=Route target=Route

users satisfies "api:users" | "api:posts";
/// @resolution.name source=users target=users
/// @resolution.place source=users placement="local" lifetime="static" access="immutable"
/// @resolution.access source=users root=users

posts satisfies Route;
/// @resolution.name source=posts target=posts
/// @resolution.place source=posts placement="local" lifetime="static" access="immutable"
/// @resolution.access source=posts root=posts
/// @resolution.name source=Route target=Route
"#,
    );
}

/// Two union spans in one template match every combination of their arms.
#[test]
fn test_cross_product_union_spans_in_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type Route = `${"en" | "de"}-${"users" | "posts"}`;

const enUsers: Route = "en-users";
const dePosts: Route = "de-posts";
const bad: Route = "fr-users";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `${"en" | "de"}-${"users" | "posts"}`;

const enUsers: Route = "en-users";
const dePosts: Route = "de-posts";
const bad: Route = "fr-users";

=== dir ===
type Route = `${"en" | "de"}-${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `${\"en\" | \"de\"}-${\"users\" | \"posts\"}`" type="en-users" | "en-posts" | "de-users" | "de-posts"
/// @definition.type symbol=Route source="type Route = `${\"en\" | \"de\"}-${\"users\" | \"posts\"}`" value=`${"en" | "de"}-${"users" | "posts"}`

const enUsers: Route = "en-users";
/// @type.symbol symbol=enUsers source=enUsers type=Route
/// @resolution.pattern source=enUsers kind=binding target=enUsers
/// @resolution.name source=Route target=Route

const dePosts: Route = "de-posts";
/// @type.symbol symbol=dePosts source=dePosts type=Route
/// @resolution.pattern source=dePosts kind=binding target=dePosts
/// @resolution.name source=Route target=Route

const bad: Route = "fr-users";
/// @type.symbol symbol=bad source=bad type=Route
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Route target=Route
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"fr-users\"' is not assignable to type 'Route'"
/// @diagnostic.label line=6 column=20 span="\"fr-users\"" line_source="const bad: Route = \"fr-users\";"
/// @diagnostic.related line=6 column=12 span="Route" line_source="const bad: Route = \"fr-users\";" message="expected due to this annotation"
/// @diagnostic.note message="'Route' reduces to '\"en-users\" | \"en-posts\" | \"de-users\" | \"de-posts\"'"
"#,
    );
}

/// A string outside the union span reports a diagnostic.
#[test]
fn test_reject_non_member_string_spans_in_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type Route = `api:${"users" | "posts"}`;

const bad: Route = "api:orders";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${"users" | "posts"}`;

const bad: Route = "api:orders";

=== dir ===
type Route = `api:${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" type="api:users" | "api:posts"
/// @definition.type symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" value=`api:${"users" | "posts"}`

const bad: Route = "api:orders";
/// @type.symbol symbol=bad source=bad type=Route
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Route target=Route
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"api:orders\"' is not assignable to type 'Route'"
/// @diagnostic.label line=4 column=20 span="\"api:orders\"" line_source="const bad: Route = \"api:orders\";"
/// @diagnostic.related line=4 column=12 span="Route" line_source="const bad: Route = \"api:orders\";" message="expected due to this annotation"
/// @diagnostic.note message="'Route' reduces to '\"api:users\" | \"api:posts\"'"
"#,
    );
}

/// A template matches boolean, null, and undefined spans by their text.
#[test]
fn test_match_stringifiable_primitive_spans_against_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type PrimitiveText = `${boolean}-${null}-${undefined}`;

const ok: PrimitiveText = "true-null-undefined";
const bad: PrimitiveText = "yes-null-undefined";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type PrimitiveText = `${boolean}-${null}-${undefined}`;

const ok: PrimitiveText = "true-null-undefined";
const bad: PrimitiveText = "yes-null-undefined";

=== dir ===
type PrimitiveText = `${boolean}-${null}-${undefined}`;
/// @type.symbol symbol=PrimitiveText source="type PrimitiveText = `${boolean}-${null}-${undefined}`" type="false-null-undefined" | "true-null-undefined"
/// @definition.type symbol=PrimitiveText source="type PrimitiveText = `${boolean}-${null}-${undefined}`" value=`${boolean}-${null}-${undefined}`

const ok: PrimitiveText = "true-null-undefined";
/// @type.symbol symbol=ok source=ok type=PrimitiveText
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=PrimitiveText target=PrimitiveText

const bad: PrimitiveText = "yes-null-undefined";
/// @type.symbol symbol=bad source=bad type=PrimitiveText
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=PrimitiveText target=PrimitiveText
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"yes-null-undefined\"' is not assignable to type 'PrimitiveText'"
/// @diagnostic.label line=5 column=28 span="\"yes-null-undefined\"" line_source="const bad: PrimitiveText = \"yes-null-undefined\";"
/// @diagnostic.related line=5 column=12 span="PrimitiveText" line_source="const bad: PrimitiveText = \"yes-null-undefined\";" message="expected due to this annotation"
/// @diagnostic.note message="'PrimitiveText' reduces to '\"false-null-undefined\" | \"true-null-undefined\"'"
"#,
    );
}

/// A template holding a never span reduces to never.
#[test]
fn test_reduce_a_never_span_to_never_in_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type Nothing = `id:${never}`;

const bad: Nothing = "id:anything";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Nothing = `id:${never}`;

const bad: Nothing = "id:anything";

=== dir ===
type Nothing = `id:${never}`;
/// @type.symbol symbol=Nothing source="type Nothing = `id:${never}`" type=never
/// @definition.type symbol=Nothing source="type Nothing = `id:${never}`" value=`id:${never}`

const bad: Nothing = "id:anything";
/// @type.symbol symbol=bad source=bad type=Nothing
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Nothing target=Nothing
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"id:anything\"' is not assignable to type 'Nothing'"
/// @diagnostic.label line=4 column=22 span="\"id:anything\"" line_source="const bad: Nothing = \"id:anything\";"
/// @diagnostic.related line=4 column=12 span="Nothing" line_source="const bad: Nothing = \"id:anything\";" message="expected due to this annotation"
/// @diagnostic.note message="'Nothing' reduces to 'never'"
"#,
    );
}

/// A template of broad string spans accepts any string.
#[test]
fn test_accept_broad_string_spans_in_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type AnyString = `${string}${string}`;

declare const value: string;
const ok: AnyString = value;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type AnyString = `${string}${string}`;

declare const value: string;
const ok: AnyString = value;

=== dir ===
type AnyString = `${string}${string}`;
/// @type.symbol symbol=AnyString source="type AnyString = `${string}${string}`" type=`${string}${string}`
/// @definition.type symbol=AnyString source="type AnyString = `${string}${string}`" value=`${string}${string}`

declare const value: string;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value

const ok: AnyString = value;
/// @type.symbol symbol=ok source=ok type=AnyString
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=AnyString target=AnyString
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// A template literal type matches a numeric span by its text.
#[test]
fn test_match_numeric_spans_against_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type NumericRoute = `item:${number}`;

const item: NumericRoute = "item:42";

item satisfies `item:${number}`;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type NumericRoute = `item:${number}`;

const item: NumericRoute = "item:42";

item satisfies `item:${number}`;

=== dir ===
type NumericRoute = `item:${number}`;
/// @type.symbol symbol=NumericRoute source="type NumericRoute = `item:${number}`" type=`item:${float64}`
/// @definition.type symbol=NumericRoute source="type NumericRoute = `item:${number}`" value=`item:${float64}`

const item: NumericRoute = "item:42";
/// @type.symbol symbol=item source=item type=NumericRoute
/// @resolution.pattern source=item kind=binding target=item
/// @resolution.name source=NumericRoute target=NumericRoute

item satisfies `item:${number}`;
/// @resolution.name source=item target=item
/// @resolution.place source=item placement="local" lifetime="static" access="immutable"
/// @resolution.access source=item root=item
"#,
    );
}

/// A non-numeric text reports a diagnostic at a numeric span.
#[test]
fn test_reject_non_numeric_spans_in_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type NumericRoute = `item:${number}`;

const bad: NumericRoute = "item:abc";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type NumericRoute = `item:${number}`;

const bad: NumericRoute = "item:abc";

=== dir ===
type NumericRoute = `item:${number}`;
/// @type.symbol symbol=NumericRoute source="type NumericRoute = `item:${number}`" type=`item:${float64}`
/// @definition.type symbol=NumericRoute source="type NumericRoute = `item:${number}`" value=`item:${float64}`

const bad: NumericRoute = "item:abc";
/// @type.symbol symbol=bad source=bad type=NumericRoute
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=NumericRoute target=NumericRoute
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"item:abc\"' is not assignable to type 'NumericRoute'"
/// @diagnostic.label line=4 column=27 span="\"item:abc\"" line_source="const bad: NumericRoute = \"item:abc\";"
/// @diagnostic.related line=4 column=12 span="NumericRoute" line_source="const bad: NumericRoute = \"item:abc\";" message="expected due to this annotation"
/// @diagnostic.note message="'NumericRoute' reduces to '`item:${float64}`'"
"#,
    );
}

/// A template holding another template matches the flattened text.
#[test]
fn test_match_nested_templates_in_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type Nested = `prefix-${`id-${number}`}`;

const value: Nested = "prefix-id-1";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Nested = `prefix-${`id-${number}`}`;

const value: Nested = "prefix-id-1";

=== dir ===
type Nested = `prefix-${`id-${number}`}`;
/// @type.symbol symbol=Nested source="type Nested = `prefix-${`id-${number}`}`" type=`prefix-id-${float64}`
/// @definition.type symbol=Nested source="type Nested = `prefix-${`id-${number}`}`" value=`prefix-${`id-${float64}`}`

const value: Nested = "prefix-id-1";
/// @type.symbol symbol=value source=value type=Nested
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Nested target=Nested
"#,
    );
}

/// A text failing the nested template reports a diagnostic.
#[test]
fn test_reject_nested_template_mismatches_in_a_template_literal_type() {
    let session = TestSession::single(
        r#"
type Nested = `prefix-${`id-${number}`}`;

const value: Nested = "prefix-id-a";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Nested = `prefix-${`id-${number}`}`;

const value: Nested = "prefix-id-a";

=== dir ===
type Nested = `prefix-${`id-${number}`}`;
/// @type.symbol symbol=Nested source="type Nested = `prefix-${`id-${number}`}`" type=`prefix-id-${float64}`
/// @definition.type symbol=Nested source="type Nested = `prefix-${`id-${number}`}`" value=`prefix-${`id-${float64}`}`

const value: Nested = "prefix-id-a";
/// @type.symbol symbol=value source=value type=Nested
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Nested target=Nested
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"prefix-id-a\"' is not assignable to type 'Nested'"
/// @diagnostic.label line=4 column=23 span="\"prefix-id-a\"" line_source="const value: Nested = \"prefix-id-a\";"
/// @diagnostic.related line=4 column=14 span="Nested" line_source="const value: Nested = \"prefix-id-a\";" message="expected due to this annotation"
/// @diagnostic.note message="'Nested' reduces to '`prefix-id-${float64}`'"
"#,
    );
}

/// Reduce a boolean span to the two strings it can spell.
#[test]
fn test_reduce_a_boolean_span_to_two_strings() {
    let session = TestSession::single(
        r#"
type Flag = `${boolean}`;

declare const flag: Flag;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flag = `${boolean}`;

declare const flag: Flag;

=== dir ===
type Flag = `${boolean}`;
/// @type.symbol symbol=Flag source="type Flag = `${boolean}`" type="false" | "true"
/// @definition.type symbol=Flag source="type Flag = `${boolean}`" value=`${boolean}`

declare const flag: Flag;
/// @type.symbol symbol=flag source=flag type=Flag
/// @resolution.pattern source=flag kind=binding target=flag
/// @resolution.name source=Flag target=Flag
"#,
        r#"
"#,
    );
}

/// Flatten a template span that holds another template into one template.
#[test]
fn test_flatten_a_nested_template_span_into_one_template() {
    let session = TestSession::single(
        r#"
type Nested = `a${`b${string}`}c`;

declare const value: Nested;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Nested = `a${`b${string}`}c`;

declare const value: Nested;

=== dir ===
type Nested = `a${`b${string}`}c`;
/// @type.symbol symbol=Nested source="type Nested = `a${`b${string}`}c`" type=`ab${string}c`
/// @definition.type symbol=Nested source="type Nested = `a${`b${string}`}c`" value=`a${`b${string}`}c`

declare const value: Nested;
/// @type.symbol symbol=value source=value type=Nested
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Nested target=Nested
"#,
        r#"
"#,
    );
}
