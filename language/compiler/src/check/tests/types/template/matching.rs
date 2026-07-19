use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_types_match_string_union_spans() {
    let session = TestSession::single(
        r#"
type Route = `api:${"users" | "posts"}`;

const users: Route = "api:users";
const posts: Route = "api:posts";

users satisfies "api:users" | "api:posts";
posts satisfies Route;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${"users" | "posts"}`;

const users: Route = "api:users" as Route;
const posts: Route = "api:posts" as Route;

users satisfies "api:users" | "api:posts";
posts satisfies Route;

=== checked ===
type Route = `api:${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" type=`api:${"users" | "posts"}` reduced="api:users" | "api:posts"
/// @definition.type symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" value=`api:${"users" | "posts"}` reduced="api:users" | "api:posts"

const users: Route = "api:users";
/// @type.symbol symbol=users source=users type=Route reduced="api:users" | "api:posts"
/// @resolution.name source=Route target=Route

const posts: Route = "api:posts";
/// @type.symbol symbol=posts source=posts type=Route reduced="api:users" | "api:posts"
/// @resolution.name source=Route target=Route

users satisfies "api:users" | "api:posts";
/// @resolution.name source=users target=users

posts satisfies Route;
/// @resolution.name source=posts target=posts
/// @resolution.name source=Route target=Route
"#,
    );
}

#[test]
fn test_template_literal_types_cross_product_union_spans() {
    let session = TestSession::single(
        r#"
type Route = `${"en" | "de"}-${"users" | "posts"}`;

const enUsers: Route = "en-users";
const dePosts: Route = "de-posts";
const bad: Route = "fr-users";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `${"en" | "de"}-${"users" | "posts"}`;

const enUsers: Route = "en-users" as Route;
const dePosts: Route = "de-posts" as Route;
const bad: Route = "fr-users";

=== checked ===
type Route = `${"en" | "de"}-${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `${\"en\" | \"de\"}-${\"users\" | \"posts\"}`" type=`${"en" | "de"}-${"users" | "posts"}` reduced="en-users" | "en-posts" | "de-users" | "de-posts"
/// @definition.type symbol=Route source="type Route = `${\"en\" | \"de\"}-${\"users\" | \"posts\"}`" value=`${"en" | "de"}-${"users" | "posts"}` reduced="en-users" | "en-posts" | "de-users" | "de-posts"

const enUsers: Route = "en-users";
/// @type.symbol symbol=enUsers source=enUsers type=Route reduced="en-users" | "en-posts" | "de-users" | "de-posts"
/// @resolution.name source=Route target=Route

const dePosts: Route = "de-posts";
/// @type.symbol symbol=dePosts source=dePosts type=Route reduced="en-users" | "en-posts" | "de-users" | "de-posts"
/// @resolution.name source=Route target=Route

const bad: Route = "fr-users";
/// @type.symbol symbol=bad source=bad type=Route reduced="en-users" | "en-posts" | "de-users" | "de-posts"
/// @resolution.name source=Route target=Route
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"fr-users\"' is not assignable to type 'Route'"
/// @diagnostic.label line=6 column=20 span="\"fr-users\"" line_source="const bad: Route = \"fr-users\";"
/// @diagnostic.note message="'Route' reduces to '\"en-users\" | \"en-posts\" | \"de-users\" | \"de-posts\"'"
"#,
    );
}

#[test]
fn test_template_literal_types_reject_non_member_string_spans() {
    let session = TestSession::single(
        r#"
type Route = `api:${"users" | "posts"}`;

const bad: Route = "api:orders";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${"users" | "posts"}`;

const bad: Route = "api:orders";

=== checked ===
type Route = `api:${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" type=`api:${"users" | "posts"}` reduced="api:users" | "api:posts"
/// @definition.type symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" value=`api:${"users" | "posts"}` reduced="api:users" | "api:posts"

const bad: Route = "api:orders";
/// @type.symbol symbol=bad source=bad type=Route reduced="api:users" | "api:posts"
/// @resolution.name source=Route target=Route
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"api:orders\"' is not assignable to type 'Route'"
/// @diagnostic.label line=4 column=20 span="\"api:orders\"" line_source="const bad: Route = \"api:orders\";"
/// @diagnostic.note message="'Route' reduces to '\"api:users\" | \"api:posts\"'"
"#,
    );
}

#[test]
fn test_template_literal_types_match_stringifiable_primitive_spans() {
    let session = TestSession::single(
        r#"
type PrimitiveText = `${boolean}-${null}-${undefined}`;

const ok: PrimitiveText = "true-null-undefined";
const bad: PrimitiveText = "yes-null-undefined";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type PrimitiveText = `${boolean}-${null}-${undefined}`;

const ok: PrimitiveText = "true-null-undefined";
const bad: PrimitiveText = "yes-null-undefined";

=== checked ===
type PrimitiveText = `${boolean}-${null}-${undefined}`;
/// @type.symbol symbol=PrimitiveText source="type PrimitiveText = `${boolean}-${null}-${undefined}`" type=`${boolean}-${null}-${undefined}`
/// @definition.type symbol=PrimitiveText source="type PrimitiveText = `${boolean}-${null}-${undefined}`" value=`${boolean}-${null}-${undefined}`

const ok: PrimitiveText = "true-null-undefined";
/// @type.symbol symbol=ok source=ok type=PrimitiveText reduced=`${boolean}-${null}-${undefined}`
/// @resolution.name source=PrimitiveText target=PrimitiveText

const bad: PrimitiveText = "yes-null-undefined";
/// @type.symbol symbol=bad source=bad type=PrimitiveText reduced=`${boolean}-${null}-${undefined}`
/// @resolution.name source=PrimitiveText target=PrimitiveText
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"yes-null-undefined\"' is not assignable to type 'PrimitiveText'"
/// @diagnostic.label line=5 column=28 span="\"yes-null-undefined\"" line_source="const bad: PrimitiveText = \"yes-null-undefined\";"
/// @diagnostic.note message="'PrimitiveText' reduces to '`${boolean}-${null}-${undefined}`'"
"#,
    );
}

#[test]
fn test_template_literal_types_reduce_never_span_to_never() {
    let session = TestSession::single(
        r#"
type Nothing = `id:${never}`;

const bad: Nothing = "id:anything";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Nothing = `id:${never}`;

const bad: Nothing = "id:anything";

=== checked ===
type Nothing = `id:${never}`;
/// @type.symbol symbol=Nothing source="type Nothing = `id:${never}`" type=`id:${never}` reduced=never
/// @definition.type symbol=Nothing source="type Nothing = `id:${never}`" value=`id:${never}` reduced=never

const bad: Nothing = "id:anything";
/// @type.symbol symbol=bad source=bad type=Nothing reduced=never
/// @resolution.name source=Nothing target=Nothing
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"id:anything\"' is not assignable to type 'Nothing'"
/// @diagnostic.label line=4 column=22 span="\"id:anything\"" line_source="const bad: Nothing = \"id:anything\";"
/// @diagnostic.note message="'Nothing' reduces to 'never'"
"#,
    );
}

#[test]
fn test_template_literal_types_accept_broad_string_spans() {
    let session = TestSession::single(
        r#"
type AnyString = `${string}${string}`;

declare const value: string;
const ok: AnyString = value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type AnyString = `${string}${string}`;

declare const value: string;
const ok: AnyString = value;

=== checked ===
type AnyString = `${string}${string}`;
/// @type.symbol symbol=AnyString source="type AnyString = `${string}${string}`" type=`${string}${string}`
/// @definition.type symbol=AnyString source="type AnyString = `${string}${string}`" value=`${string}${string}`

declare const value: string;
/// @type.symbol symbol=value source=value type=string

const ok: AnyString = value;
/// @type.symbol symbol=ok source=ok type=AnyString reduced=`${string}${string}`
/// @resolution.name source=AnyString target=AnyString
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_template_literal_types_match_numeric_spans() {
    let session = TestSession::single(
        r#"
type NumericRoute = `item:${number}`;

const item: NumericRoute = "item:42";

item satisfies `item:${number}`;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type NumericRoute = `item:${number}`;

const item: NumericRoute = "item:42";

item satisfies `item:${number}`;

=== checked ===
type NumericRoute = `item:${number}`;
/// @type.symbol symbol=NumericRoute source="type NumericRoute = `item:${number}`" type=`item:${float64}`
/// @definition.type symbol=NumericRoute source="type NumericRoute = `item:${number}`" value=`item:${float64}`

const item: NumericRoute = "item:42";
/// @type.symbol symbol=item source=item type=NumericRoute reduced=`item:${float64}`
/// @resolution.name source=NumericRoute target=NumericRoute

item satisfies `item:${number}`;
/// @resolution.name source=item target=item
"#,
    );
}

#[test]
fn test_template_literal_types_reject_non_numeric_spans() {
    let session = TestSession::single(
        r#"
type NumericRoute = `item:${number}`;

const bad: NumericRoute = "item:abc";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type NumericRoute = `item:${number}`;

const bad: NumericRoute = "item:abc";

=== checked ===
type NumericRoute = `item:${number}`;
/// @type.symbol symbol=NumericRoute source="type NumericRoute = `item:${number}`" type=`item:${float64}`
/// @definition.type symbol=NumericRoute source="type NumericRoute = `item:${number}`" value=`item:${float64}`

const bad: NumericRoute = "item:abc";
/// @type.symbol symbol=bad source=bad type=NumericRoute reduced=`item:${float64}`
/// @resolution.name source=NumericRoute target=NumericRoute
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"item:abc\"' is not assignable to type 'NumericRoute'"
/// @diagnostic.label line=4 column=27 span="\"item:abc\"" line_source="const bad: NumericRoute = \"item:abc\";"
/// @diagnostic.note message="'NumericRoute' reduces to '`item:${float64}`'"
"#,
    );
}

#[test]
fn test_template_literal_types_match_nested_templates() {
    let session = TestSession::single(
        r#"
type Nested = `prefix-${`id-${number}`}`;

const value: Nested = "prefix-id-1";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Nested = `prefix-${`id-${number}`}`;

const value: Nested = "prefix-id-1";

=== checked ===
type Nested = `prefix-${`id-${number}`}`;
/// @type.symbol symbol=Nested source="type Nested = `prefix-${`id-${number}`}`" type=`prefix-${`id-${float64}`}`
/// @definition.type symbol=Nested source="type Nested = `prefix-${`id-${number}`}`" value=`prefix-${`id-${float64}`}`

const value: Nested = "prefix-id-1";
/// @type.symbol symbol=value source=value type=Nested reduced=`prefix-${`id-${float64}`}`
/// @resolution.name source=Nested target=Nested
"#,
    );
}

#[test]
fn test_template_literal_types_reject_nested_template_mismatches() {
    let session = TestSession::single(
        r#"
type Nested = `prefix-${`id-${number}`}`;

const value: Nested = "prefix-id-a";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Nested = `prefix-${`id-${number}`}`;

const value: Nested = "prefix-id-a";

=== checked ===
type Nested = `prefix-${`id-${number}`}`;
/// @type.symbol symbol=Nested source="type Nested = `prefix-${`id-${number}`}`" type=`prefix-${`id-${float64}`}`
/// @definition.type symbol=Nested source="type Nested = `prefix-${`id-${number}`}`" value=`prefix-${`id-${float64}`}`

const value: Nested = "prefix-id-a";
/// @type.symbol symbol=value source=value type=Nested reduced=`prefix-${`id-${float64}`}`
/// @resolution.name source=Nested target=Nested
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"prefix-id-a\"' is not assignable to type 'Nested'"
/// @diagnostic.label line=4 column=23 span="\"prefix-id-a\"" line_source="const value: Nested = \"prefix-id-a\";"
/// @diagnostic.note message="'Nested' reduces to '`prefix-${`id-${float64}`}`'"
"#,
    );
}
